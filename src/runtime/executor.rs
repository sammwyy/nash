//! AST executor — the heart of the Nash runtime.

use anyhow::{bail, Result};
use indexmap::IndexMap;

use crate::builtins;
use crate::parser::ast::{Expr, RedirectMode, Word, WordPart};
use crate::vfs::mount::MountOptions;
use crate::vfs::path::VfsPath;
use crate::vfs::Vfs;

use super::context::Context;
use super::output::Output;

/// Configuration for constructing an [`Executor`].
#[derive(Debug, Default)]
pub struct ExecutorConfig {
    /// Initial VFS working directory.
    pub cwd: String,
    /// Initial environment variables.
    pub env: IndexMap<String, String>,
    /// Host directory mounts: (host_path, vfs_path, opts).
    pub mounts: Vec<(String, String, MountOptions)>,
}

/// The sandboxed executor.
pub struct Executor {
    ctx: Context,
}

impl Executor {
    /// Create a new executor, set up VFS, mounts, and defaults.
    pub fn new(config: ExecutorConfig, username: &str) -> Result<Self> {
        let mut vfs = Vfs::new();

        // Apply host mounts
        for (host, vfs_path, opts) in config.mounts {
            if !std::path::Path::new(&host).exists() {
                bail!("mount: host path does not exist: {}", host);
            }
            vfs.mount(host, vfs_path, opts)?;
        }

        // Realistic Unix directory skeleton
        let home_dir = format!("/home/{}", username);
        for dir in &[
            "/",
            "/bin",
            "/sbin",
            "/usr",
            "/usr/bin",
            "/usr/sbin",
            "/usr/local",
            "/usr/local/bin",
            "/etc",
            "/var",
            "/var/log",
            "/var/tmp",
            "/tmp",
            "/lib",
            "/lib64",
            "/opt",
            "/root",
            "/proc",
            "/dev",
            home_dir.as_str(),
        ] {
            vfs.mkdir_p(dir)?;
        }
        // User home sub-dirs
        for sub in &["Desktop", "Documents", "Downloads"] {
            vfs.mkdir_p(&format!("{}/{}", home_dir, sub))?;
        }

        // Skeleton files
        vfs.write_str(
            &format!("{}/.nashrc", home_dir),
            &format!("# ~/.nashrc\nexport USER={}\n", username),
        )?;
        vfs.write_str("/etc/hostname", "nash\n")?;
        vfs.write_str("/etc/shells", "/bin/nash\n/bin/sh\n")?;
        vfs.write_str(
            &format!("{}/welcome.txt", home_dir),
            "Welcome to Nash!\nType \'help\' to see available commands.\n",
        )?;

        let cwd = if config.cwd.is_empty() {
            home_dir.clone()
        } else {
            config.cwd.clone()
        };
        vfs.mkdir_p(&cwd)?;

        // env comes pre-populated from CLI; or fill in minimal defaults for tests
        let mut env = config.env;
        env.entry("USER".into())
            .or_insert_with(|| username.to_string());
        env.entry("LOGNAME".into())
            .or_insert_with(|| username.to_string());
        env.entry("HOME".into()).or_insert_with(|| home_dir.clone());
        env.entry("SHELL".into()).or_insert_with(|| "nash".into());
        env.entry("TERM".into())
            .or_insert_with(|| "xterm-256color".into());
        env.entry("LANG".into())
            .or_insert_with(|| "en_US.UTF-8".into());
        env.entry("PATH".into())
            .or_insert_with(|| "/usr/local/bin:/usr/bin:/bin".into());
        env.entry("PWD".into()).or_insert_with(|| cwd.clone());
        env.entry("OLDPWD".into()).or_insert_with(|| "/".into());
        env.entry("HOSTNAME".into())
            .or_insert_with(|| "nash".into());
        env.entry("SHLVL".into()).or_insert_with(|| "1".into());

        Ok(Executor {
            ctx: Context::new(cwd, env, vfs),
        })
    }

    /// Return the current VFS working directory.
    pub fn cwd(&self) -> &str {
        &self.ctx.cwd
    }

    /// Push a raw command line into the session history.
    pub fn push_history(&mut self, line: String) {
        self.ctx.history.push(line);
    }

    /// Keep $PWD env var in sync with actual cwd after each command.
    pub fn sync_pwd(&mut self) {
        let cwd = self.ctx.cwd.clone();
        self.ctx.env.insert("PWD".into(), cwd);
    }

    /// Check whether a VFS path exists (used by rc sourcing in cli.rs).
    pub fn vfs_exists(&self, path: &str) -> bool {
        self.ctx.vfs.exists(path)
    }

    /// Read a VFS file as a UTF-8 string (used by rc sourcing in cli.rs).
    pub fn vfs_read_string(&self, path: &str) -> Result<String> {
        self.ctx.vfs.read_to_string(path)
    }

    /// Execute an expression, returning its output.
    pub fn execute(&mut self, expr: &Expr) -> Result<Output> {
        self.eval(expr, "")
    }

    // ─── Core evaluator ──────────────────────────────────────────────────────

    fn eval(&mut self, expr: &Expr, stdin: &str) -> Result<Output> {
        match expr {
            Expr::Command { name, args } => self.eval_command(name, args, stdin),

            Expr::Pipe { left, right } => {
                let left_out = self.eval(left, stdin)?;
                self.eval(right, &left_out.stdout)
            }

            Expr::Redirect { expr, file, mode } => self.eval_redirect(expr, file, mode, stdin),

            Expr::Sequence { left, right } => {
                let left_out = self.eval(left, stdin)?;
                // Always print left output
                if !left_out.stdout.is_empty() {
                    print!("{}", left_out.stdout);
                }
                if !left_out.stderr.is_empty() {
                    eprint!("{}", left_out.stderr);
                }
                self.eval(right, stdin)
            }

            Expr::And { left, right } => {
                let left_out = self.eval(left, stdin)?;
                if left_out.is_success() {
                    // Print left output before continuing
                    if !left_out.stdout.is_empty() {
                        print!("{}", left_out.stdout);
                    }
                    self.eval(right, stdin)
                } else {
                    Ok(left_out)
                }
            }

            Expr::Or { left, right } => {
                let left_out = self.eval(left, stdin)?;
                if !left_out.is_success() {
                    self.eval(right, stdin)
                } else {
                    Ok(left_out)
                }
            }

            Expr::Subshell { expr } => {
                // Subshell: run in a cloned context (env changes don't propagate back)
                let saved_cwd = self.ctx.cwd.clone();
                let saved_env = self.ctx.env.clone();
                let result = self.eval(expr, stdin);
                self.ctx.cwd = saved_cwd;
                self.ctx.env = saved_env;
                result
            }
        }
    }

    fn eval_command(&mut self, name: &Word, args: &[Word], stdin: &str) -> Result<Output> {
        let name_str = self.expand_word(name)?;
        let arg_strs: Vec<String> = args
            .iter()
            .map(|w| self.expand_word(w))
            .collect::<Result<Vec<_>>>()?;

        if let Some(builtin) = builtins::dispatch(&name_str) {
            builtin.run(&arg_strs, &mut self.ctx, stdin)
        } else {
            Ok(Output::error(
                127,
                "",
                &format!("nash: command not found: {}\n", name_str),
            ))
        }
    }

    fn eval_redirect(
        &mut self,
        expr: &Expr,
        file: &Word,
        mode: &RedirectMode,
        stdin: &str,
    ) -> Result<Output> {
        match mode {
            RedirectMode::Input => {
                // Read file and pass as stdin to command
                let path = self.expand_word(file)?;
                let abs = VfsPath::join(&self.ctx.cwd, &path);
                let content = match self.ctx.vfs.read_to_string(&abs) {
                    Ok(c) => c,
                    Err(e) => {
                        return Ok(Output::error(1, "", &format!("nash: {}\n", e)));
                    }
                };
                self.eval(expr, &content)
            }
            RedirectMode::Overwrite | RedirectMode::Append => {
                // Execute command, then write stdout to file
                let output = self.eval(expr, stdin)?;
                let path = self.expand_word(file)?;
                let abs = VfsPath::join(&self.ctx.cwd, &path);
                if *mode == RedirectMode::Overwrite {
                    self.ctx.vfs.write_str(&abs, &output.stdout)?;
                } else {
                    self.ctx
                        .vfs
                        .append(&abs, output.stdout.as_bytes().to_vec())?;
                }
                // Return output with stdout consumed (it went to file)
                Ok(Output {
                    stdout: String::new(),
                    stderr: output.stderr,
                    exit_code: output.exit_code,
                })
            }
        }
    }

    // ─── Word expansion ──────────────────────────────────────────────────────

    fn expand_word(&mut self, word: &Word) -> Result<String> {
        let mut result = String::new();
        for part in &word.0 {
            match part {
                WordPart::Literal(s) => result.push_str(s),
                WordPart::Variable(name) => {
                    let val = self.ctx.env.get(name).cloned().unwrap_or_default();
                    result.push_str(&val);
                }
                WordPart::CommandSubst(expr) => {
                    let output = self.eval(expr, "")?;
                    // Trim trailing newline (bash behaviour)
                    result.push_str(output.stdout.trim_end_matches('\n'));
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn exec(cmd: &str) -> Output {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        let expr = parse(cmd).unwrap();
        executor.execute(&expr).unwrap()
    }

    #[test]
    fn test_echo() {
        let out = exec("echo hello world");
        assert_eq!(out.stdout, "hello world\n");
        assert!(out.is_success());
    }

    #[test]
    fn test_echo_no_newline() {
        let out = exec("echo -n hello");
        assert_eq!(out.stdout, "hello");
    }

    #[test]
    fn test_pwd_default() {
        let out = exec("pwd");
        assert!(out.stdout.trim() == "/home/user");
    }

    #[test]
    fn test_mkdir_and_ls() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        let mk = parse("mkdir /tmp/testdir").unwrap();
        executor.execute(&mk).unwrap();
        let ls = parse("ls /tmp/testdir").unwrap();
        let out = executor.execute(&ls).unwrap();
        // Empty directory — ls should succeed with no output
        assert!(out.is_success());
    }

    #[test]
    fn test_touch_and_cat() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("touch /tmp/hello.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("cat /tmp/hello.txt").unwrap())
            .unwrap();
        assert!(out.is_success());
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn test_redirect_write_and_cat() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("echo hello > /tmp/out.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("cat /tmp/out.txt").unwrap())
            .unwrap();
        assert_eq!(out.stdout, "hello\n");
    }

    #[test]
    fn test_pipe_echo_cat() {
        let out = exec("echo hello | cat");
        assert_eq!(out.stdout, "hello\n");
    }

    #[test]
    fn test_pipe_echo_grep() {
        let out = exec("echo hello | grep hello");
        assert_eq!(out.stdout, "hello\n");
    }

    #[test]
    fn test_pipe_echo_grep_miss() {
        let out = exec("echo hello | grep world");
        assert!(!out.is_success());
    }

    #[test]
    fn test_cd_and_pwd() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor.execute(&parse("cd /tmp").unwrap()).unwrap();
        let out = executor.execute(&parse("pwd").unwrap()).unwrap();
        assert_eq!(out.stdout.trim(), "/tmp");
    }

    #[test]
    fn test_and_both_succeed() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        let out = executor
            .execute(&parse("true && echo yes").unwrap())
            .unwrap();
        assert_eq!(out.stdout, "yes\n");
    }

    #[test]
    fn test_and_first_fails() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        let out = executor
            .execute(&parse("false && echo yes").unwrap())
            .unwrap();
        assert!(!out.is_success());
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn test_or_first_fails() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        let out = executor
            .execute(&parse("false || echo fallback").unwrap())
            .unwrap();
        assert_eq!(out.stdout, "fallback\n");
    }

    #[test]
    fn test_variable_expansion() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .ctx
            .env
            .insert("GREETING".to_string(), "hello".to_string());
        let out = executor.execute(&parse("echo $GREETING").unwrap()).unwrap();
        assert_eq!(out.stdout, "hello\n");
    }

    #[test]
    fn test_command_substitution() {
        let out = exec("echo $(echo inner)");
        assert_eq!(out.stdout, "inner\n");
    }

    #[test]
    fn test_wc_lines() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("echo -e 'a\\nb\\nc' > /tmp/wc.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("echo hello | wc -w").unwrap())
            .unwrap();
        assert!(out.stdout.contains('1'));
    }

    #[test]
    fn test_grep_invert() {
        let out = exec("echo hello | grep -v world");
        assert_eq!(out.stdout, "hello\n");
    }

    #[test]
    fn test_unknown_command() {
        let out = exec("nonexistent_cmd");
        assert_eq!(out.exit_code, 127);
    }

    #[test]
    fn test_sort() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("echo 'c\\nb\\na' > /tmp/sort.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("echo banana | cat | sort").unwrap())
            .unwrap();
        assert!(out.is_success());
    }

    #[test]
    fn test_subshell_isolation() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("export TESTVAR=before").unwrap())
            .unwrap();
        executor
            .execute(&parse("(export TESTVAR=inside)").unwrap())
            .unwrap();
        let out = executor.execute(&parse("echo $TESTVAR").unwrap()).unwrap();
        assert_eq!(out.stdout, "before\n");
    }

    // ── Extended builtins ───────────────────────────────────────────────────

    #[test]
    fn test_find_basic() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("touch /tmp/findme.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("find /tmp -name findme.txt").unwrap())
            .unwrap();
        assert!(out.stdout.contains("findme.txt"));
    }

    #[test]
    fn test_find_type_d() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("mkdir /tmp/finddir").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("find /tmp -type d").unwrap())
            .unwrap();
        assert!(out.stdout.contains("finddir"));
    }

    #[test]
    fn test_find_glob() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("touch /tmp/a.txt").unwrap())
            .unwrap();
        executor
            .execute(&parse("touch /tmp/b.log").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("find /tmp -name *.txt").unwrap())
            .unwrap();
        assert!(out.stdout.contains("a.txt"));
        assert!(!out.stdout.contains("b.log"));
    }

    #[test]
    fn test_which_builtin() {
        let out = exec("which echo");
        assert!(out.stdout.contains("builtin"));
        assert!(out.is_success());
    }

    #[test]
    fn test_which_not_found() {
        let out = exec("which nonexistent_xyz");
        assert!(!out.is_success());
    }

    #[test]
    fn test_history() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor.push_history("echo first".to_string());
        executor.push_history("echo second".to_string());
        let out = executor.execute(&parse("history").unwrap()).unwrap();
        assert!(out.stdout.contains("echo first"));
        assert!(out.stdout.contains("echo second"));
    }

    #[test]
    fn test_history_limit() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        for i in 0..5 {
            executor.push_history(format!("cmd {}", i));
        }
        let out = executor.execute(&parse("history 2").unwrap()).unwrap();
        let lines: Vec<_> = out.stdout.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_clear() {
        let out = exec("clear");
        assert!(out.stdout.contains('\x1b'));
    }

    #[test]
    fn test_sed_substitute() {
        let out = exec("echo hello world | sed s/world/nash/");
        assert_eq!(out.stdout.trim(), "hello nash");
    }

    #[test]
    fn test_sed_global() {
        let out = exec("echo aaa | sed s/a/b/g");
        assert_eq!(out.stdout.trim(), "bbb");
    }

    #[test]
    fn test_sed_delete() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("echo hello > /tmp/sed_d.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("cat /tmp/sed_d.txt | sed d").unwrap())
            .unwrap();
        assert_eq!(out.stdout, "");
    }

    #[test]
    fn test_cut_fields() {
        let out = exec("echo a:b:c | cut -d : -f 2");
        assert_eq!(out.stdout.trim(), "b");
    }

    #[test]
    fn test_cut_chars() {
        let out = exec("echo hello | cut -c 1-3");
        assert_eq!(out.stdout.trim(), "hel");
    }

    #[test]
    fn test_uniq_basic() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("echo a > /tmp/uniq.txt").unwrap())
            .unwrap();
        // pipe echo with repeated lines
        let out = executor
            .execute(&parse("echo hello | uniq").unwrap())
            .unwrap();
        assert_eq!(out.stdout.trim(), "hello");
    }

    #[test]
    fn test_tree_basic() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("mkdir /tmp/treedir").unwrap())
            .unwrap();
        executor
            .execute(&parse("touch /tmp/treedir/file.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("tree /tmp/treedir").unwrap())
            .unwrap();
        assert!(out.stdout.contains("file.txt"));
        assert!(out.is_success());
    }

    #[test]
    fn test_stat_file() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("echo data > /tmp/stat_test.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("stat /tmp/stat_test.txt").unwrap())
            .unwrap();
        assert!(out.stdout.contains("regular file"));
    }

    #[test]
    fn test_stat_dir() {
        let out = exec("stat /tmp");
        assert!(out.stdout.contains("directory"));
    }

    #[test]
    fn test_file_text() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse("echo hello > /tmp/file_text.txt").unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("file /tmp/file_text.txt").unwrap())
            .unwrap();
        assert!(out.stdout.contains("text") || out.stdout.contains("ASCII"));
    }

    #[test]
    fn test_file_dir() {
        let out = exec("file /tmp");
        assert!(out.stdout.contains("directory"));
    }

    #[test]
    fn test_help() {
        let out = exec("help");
        assert!(out.stdout.contains("Nash"));
        assert!(out.stdout.contains("cd"));
        assert!(out.stdout.contains("jq"));
    }

    #[test]
    fn test_jq_identity() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse(r#"echo '{"a":1}' > /tmp/test.json"#).unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("cat /tmp/test.json | jq .").unwrap())
            .unwrap();
        assert!(out.stdout.contains('"') || out.stdout.contains('a'));
    }

    #[test]
    fn test_jq_field() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse(r#"echo '{"name":"nash"}' > /tmp/j.json"#).unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("jq .name /tmp/j.json").unwrap())
            .unwrap();
        assert!(out.stdout.contains("nash"));
    }

    #[test]
    fn test_jq_keys() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse(r#"echo '{"z":1,"a":2}' > /tmp/k.json"#).unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("jq keys /tmp/k.json").unwrap())
            .unwrap();
        assert!(out.stdout.contains("\"a\"") || out.stdout.contains("\"z\""));
    }

    #[test]
    fn test_jq_length_array() {
        let mut executor = Executor::new(ExecutorConfig::default(), "user").unwrap();
        executor
            .execute(&parse(r#"echo '[1,2,3]' > /tmp/arr.json"#).unwrap())
            .unwrap();
        let out = executor
            .execute(&parse("jq length /tmp/arr.json").unwrap())
            .unwrap();
        assert_eq!(out.stdout.trim(), "3");
    }
}
