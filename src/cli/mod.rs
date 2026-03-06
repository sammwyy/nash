use anyhow::{bail, Context, Result};
use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{self, BufRead, IsTerminal};

use crate::parser::parse;
use crate::runtime::{Executor, ExecutorConfig};
use crate::vfs::mount::MountOptions;

/// Nash — Not A Shell: a sandboxed bash-like command interpreter.
#[derive(Parser, Debug)]
#[command(
    name = "nash",
    version,
    about = "Nash — Not A Shell: a sandboxed bash-like interpreter",
    long_about = None,
    disable_version_flag = true,
    disable_help_flag = true,
    after_help = "\
EXAMPLES:
    nash                              Interactive REPL (default user)
    nash -U alice                     REPL logged in as alice
    nash script.sh                    Execute a shell script
    nash -c 'echo hello | grep hi'    One-shot command  (-c like bash/sh)
    nash -E FOO=bar -E X=1            Set environment variables
    nash -B ./proj:/proj -C /proj     Mount + start inside it
    nash -s < commands.txt            Read commands from stdin
    nash -x script.sh                 Run script with xtrace (prints each cmd)
    nash -e script.sh                 Exit immediately on any error
    nash --rcfile ~/.nashrc script.sh Load custom rc file

SHELL FLAGS  (bash/sh compatible):
    -c CMD          Execute command string and exit
    -i              Force interactive mode (even when stdin is not a tty)
    -l / --login    Login shell: source /etc/profile and ~/.nashrc
    -s              Read commands from stdin
    -e / --errexit  Exit on first error (set -e)
    -u / --nounset  Error on unset variable access (set -u)
    -x / --xtrace   Print each command before executing (set -x)
    -v / --verbose  Print each input line as it is read (set -v)

NASH FLAGS:
    -U / --user NAME        Set session username        [default: user]
    -C / --cwd PATH         Override start directory    [default: /home/<user>]
    -E / --env KEY=VALUE    Set env var (repeatable)
    -B / --bind HOST:VFS    Mount host dir read-write (repeatable)
         --bind-ro HOST:VFS Mount host dir read-only (repeatable)

OPTIONS:
         --rcfile FILE       Source FILE instead of ~/.nashrc
         --norc              Do not source any rc file
         --login             Same as -l
         --posix             Posix-compatible mode (reserved)
         --version           Print version and exit
         --help              Print this help
"
)]
pub struct NashCli {
    // ── Positional ────────────────────────────────────────────────────────────
    /// Shell script file to execute.
    #[arg(value_name = "SCRIPT", help = "Script file to run (host filesystem)")]
    pub script: Option<String>,

    // ── Standard shell flags (bash/sh compatible) ─────────────────────────────
    /// Execute command string and exit  (-c, like bash/sh/dash).
    #[arg(
        short = 'c',
        long = "command",
        value_name = "CMD",
        help = "Execute CMD string and exit  [bash: -c]"
    )]
    pub command: Option<String>,

    /// Force interactive mode even when stdin is not a tty.
    #[arg(
        short = 'i',
        long = "interactive",
        help = "Force interactive mode  [bash: -i]"
    )]
    pub interactive: bool,

    /// Login shell: source /etc/profile then ~/.nashrc.
    #[arg(
        short = 'l',
        long = "login",
        help = "Login shell: source /etc/profile and ~/.nashrc  [bash: -l]"
    )]
    pub login: bool,

    /// Read commands from stdin.
    #[arg(
        short = 's',
        long = "stdin",
        help = "Read commands from stdin  [bash: -s]"
    )]
    pub read_stdin: bool,

    /// Exit immediately if any command exits with a non-zero status (set -e).
    #[arg(
        short = 'e',
        long = "errexit",
        help = "Exit on first error  [bash: set -e]"
    )]
    pub errexit: bool,

    /// Treat unset variables as errors (set -u).
    #[arg(
        short = 'u',
        long = "nounset",
        help = "Error on unset variable reference  [bash: set -u]"
    )]
    pub nounset: bool,

    /// Print each command to stderr before executing (set -x).
    #[arg(
        short = 'x',
        long = "xtrace",
        help = "Print each command before executing  [bash: set -x]"
    )]
    pub xtrace: bool,

    /// Print each input line to stderr as it is read (set -v).
    #[arg(
        short = 'v',
        long = "verbose",
        help = "Print each input line as it is read  [bash: set -v]"
    )]
    pub verbose: bool,

    // ── Nash-specific flags ───────────────────────────────────────────────────
    /// Session username. Sets $USER, $HOME=/home/<name>, and creates the
    /// home directory inside the VFS.
    #[arg(
        short = 'U',
        long = "user",
        value_name = "NAME",
        default_value = "user",
        help = "Session username  [default: user]"
    )]
    pub user: String,

    /// Override the initial working directory inside the VFS.
    /// Defaults to /home/<user>.
    #[arg(
        short = 'C',
        long = "cwd",
        value_name = "PATH",
        help = "Start in this VFS directory  [default: /home/<user>]"
    )]
    pub cwd: Option<String>,

    /// Set an environment variable inside Nash (KEY=VALUE). Repeatable.
    ///
    ///   nash -E FOO=bar -E BAZ=qux
    #[arg(
        short = 'E',
        long = "env",
        value_name = "KEY=VALUE",
        help = "Set env var KEY=VALUE (repeatable)"
    )]
    pub env_vars: Vec<String>,

    /// Bind a host directory into the VFS read-write (HOST:VFS). Repeatable.
    #[arg(
        short = 'B',
        long = "bind",
        value_name = "HOST:VFS",
        help = "Mount HOST dir at VFS path read-write (repeatable)"
    )]
    pub binds: Vec<String>,

    /// Bind a host directory into the VFS read-only (HOST:VFS). Repeatable.
    #[arg(
        long = "bind-ro",
        value_name = "HOST:VFS",
        help = "Mount HOST dir at VFS path read-only (repeatable)"
    )]
    pub readonly_binds: Vec<String>,

    // ── Compatibility / rc flags ──────────────────────────────────────────────
    /// Source FILE as the rc/init script instead of ~/.nashrc.
    #[arg(
        long = "rcfile",
        value_name = "FILE",
        help = "Source FILE on startup instead of ~/.nashrc"
    )]
    pub rcfile: Option<String>,

    /// Do not source any rc file on startup.
    #[arg(
        long = "norc",
        help = "Do not source ~/.nashrc or any rc file on startup"
    )]
    pub norc: bool,

    /// Enable a shopt-style option (reserved for future use).
    ///
    ///   nash -O extglob
    #[arg(
        short = 'O',
        value_name = "OPTION",
        help = "Enable shell option (e.g. -O extglob)  [bash: -O]"
    )]
    pub shopt: Vec<String>,

    /// Print version and exit (--version).
    #[arg(long = "version", help = "Print version and exit")]
    pub print_version: bool,

    /// Print help and exit (--help / -h).
    #[arg(short = 'h', long = "help", help = "Print help")]
    pub print_help: bool,
}

// ─── Shell option flags (set -e / -u / -x / -v state) ────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ShellOpts {
    /// set -e: exit on error
    pub errexit: bool,
    /// set -u: error on unset var
    pub nounset: bool,
    /// set -x: xtrace
    pub xtrace: bool,
    /// set -v: verbose
    pub verbose: bool,
}

// ─── impl NashCli ─────────────────────────────────────────────────────────────

impl NashCli {
    pub fn run(self) -> Result<()> {
        // --version / --help handled first so they never need a valid config
        if self.print_version {
            println!("nash {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        if self.print_help {
            // Re-run clap help. Simplest way: print the after_help text.
            // clap doesn't easily let us print help from within run(), so we
            // just replicate the key info.
            println!("nash {} — Not A Shell", env!("CARGO_PKG_VERSION"));
            println!("Usage: nash [OPTIONS] [SCRIPT]");
            println!("Run `nash --help` for full option list.");
            return Ok(());
        }

        let username = self.user.trim().to_string();
        validate_username(&username)?;

        let opts = ShellOpts {
            errexit: self.errexit,
            nounset: self.nounset,
            xtrace: self.xtrace,
            verbose: self.verbose,
        };

        // Resolve home dir and starting cwd
        let home_dir = format!("/home/{}", username);
        let cwd = self.cwd.clone().unwrap_or_else(|| home_dir.clone());

        // Build executor config
        let mut config = ExecutorConfig::default();
        config.cwd = cwd.clone();

        // ── Baseline Unix environment ────────────────────────────────────────
        config.env.insert("USER".into(), username.clone());
        config.env.insert("LOGNAME".into(), username.clone());
        config.env.insert("HOME".into(), home_dir.clone());
        config.env.insert("SHELL".into(), "nash".into());
        config.env.insert("TERM".into(), "xterm-256color".into());
        config.env.insert("LANG".into(), "en_US.UTF-8".into());
        config.env.insert("LC_ALL".into(), "en_US.UTF-8".into());
        config.env.insert(
            "PATH".into(),
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        );
        config.env.insert("PWD".into(), cwd.clone());
        config.env.insert("OLDPWD".into(), "/".into());
        config.env.insert("HOSTNAME".into(), "nash".into());
        config.env.insert("SHLVL".into(), "1".into());
        config.env.insert("_".into(), "nash".into());

        // shell option flags reflected as $- string (bash compatible)
        let dash_flags = build_dash_flags(&opts);
        config.env.insert("-".into(), dash_flags);

        // ── -E KEY=VALUE overrides ───────────────────────────────────────────
        for kv in &self.env_vars {
            let (k, v) = kv
                .split_once('=')
                .with_context(|| format!("invalid -E value '{}' — expected KEY=VALUE", kv))?;
            config.env.insert(k.to_string(), v.to_string());
        }

        // ── Host mounts ──────────────────────────────────────────────────────
        for spec in &self.binds {
            let (host, vfs) = parse_bind(spec)?;
            config
                .mounts
                .push((host, vfs, MountOptions { read_only: false }));
        }
        for spec in &self.readonly_binds {
            let (host, vfs) = parse_bind(spec)?;
            config
                .mounts
                .push((host, vfs, MountOptions { read_only: true }));
        }

        // ── Build executor ───────────────────────────────────────────────────
        let mut executor = Executor::new(config, &username)?;

        // ── rc / init file ───────────────────────────────────────────────────
        let is_login = self.login;
        if !self.norc {
            source_rc(
                &mut executor,
                &home_dir,
                self.rcfile.as_deref(),
                is_login,
                &opts,
            )?;
        }

        // ── Determine execution mode ─────────────────────────────────────────
        //
        // Priority (highest → lowest):
        //   1. -c CMD           one-shot command string
        //   2. -s               read from stdin
        //   3. SCRIPT           positional script file
        //   4. -i               forced interactive
        //   5. stdin is a tty   interactive REPL
        //   6. stdin is a pipe  read from stdin (implicit -s)

        if let Some(cmd) = self.command {
            // -c "command string"
            run_line_opts(&mut executor, &cmd, &opts)?;
        } else if self.read_stdin {
            // -s: read commands from stdin
            run_stdin(&mut executor, &opts)?;
        } else if let Some(ref script_path) = self.script {
            run_script(&mut executor, script_path, &opts)?;
        } else if self.interactive || is_interactive_tty() {
            // Interactive REPL
            run_repl(&mut executor, &username, &opts)?;
        } else {
            // Non-tty stdin without -s: read stdin implicitly (like bash)
            run_stdin(&mut executor, &opts)?;
        }

        Ok(())
    }
}

// ─── Shell option helpers ──────────────────────────────────────────────────────

/// Build the value for `$-` (active single-char flags), like bash.
fn build_dash_flags(opts: &ShellOpts) -> String {
    let mut s = String::new();
    if opts.errexit {
        s.push('e');
    }
    if opts.nounset {
        s.push('u');
    }
    if opts.xtrace {
        s.push('x');
    }
    if opts.verbose {
        s.push('v');
    }
    s
}

/// Returns true if stdin appears to be an interactive terminal.
fn is_interactive_tty() -> bool {
    io::stdin().is_terminal()
}

// ─── rc / init sourcing ────────────────────────────────────────────────────────

fn source_rc(
    executor: &mut Executor,
    home_dir: &str,
    rcfile: Option<&str>,
    is_login: bool,
    opts: &ShellOpts,
) -> Result<()> {
    // Login shell sources /etc/profile first (VFS only)
    if is_login {
        let profile = "/etc/profile";
        if executor.vfs_exists(profile) {
            run_vfs_script(executor, profile, opts)?;
        }
    }

    // Then source the rc file
    let rc_path = if let Some(f) = rcfile {
        f.to_string()
    } else {
        format!("{}/.nashrc", home_dir)
    };

    // Try VFS first, then host filesystem
    if executor.vfs_exists(&rc_path) {
        run_vfs_script(executor, &rc_path, opts)?;
    } else if std::path::Path::new(&rc_path).exists() {
        run_script(executor, &rc_path, opts)?;
    }

    Ok(())
}

/// Execute a script that lives inside the VFS.
fn run_vfs_script(executor: &mut Executor, vfs_path: &str, opts: &ShellOpts) -> Result<()> {
    let content = match executor.vfs_read_string(vfs_path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };
    for (lineno, line) in content.lines().enumerate() {
        if lineno == 0 && line.starts_with("#!") {
            continue;
        }
        if opts.verbose {
            eprintln!("{}", line);
        }
        run_line_opts(executor, line, opts)?;
    }
    Ok(())
}

// ─── Execution primitives ──────────────────────────────────────────────────────

/// Execute one line, honouring shell option flags.
pub fn run_line_opts(executor: &mut Executor, line: &str, opts: &ShellOpts) -> Result<()> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(());
    }

    if opts.verbose {
        eprintln!("{}", trimmed);
    }

    executor.push_history(trimmed.to_string());

    match parse(trimmed) {
        Ok(expr) => {
            if opts.xtrace {
                eprintln!("+ {}", trimmed);
            }
            let output = executor.execute(&expr)?;
            if !output.stdout.is_empty() {
                print!("{}", output.stdout);
            }
            if !output.stderr.is_empty() {
                eprint!("{}", output.stderr);
            }

            if opts.errexit && !output.is_success() {
                bail!("errexit: command exited with status {}", output.exit_code);
            }
        }
        Err(e) => {
            eprintln!("nash: parse error: {e}");
            if opts.errexit {
                bail!("errexit: parse error");
            }
        }
    }
    Ok(())
}

/// Read and execute commands from stdin line by line.
fn run_stdin(executor: &mut Executor, opts: &ShellOpts) -> Result<()> {
    let stdin = io::stdin();
    let mut line_no = 0usize;
    for line_result in stdin.lock().lines() {
        line_no += 1;
        let line = line_result.with_context(|| format!("stdin read error on line {}", line_no))?;
        if let Err(e) = run_line_opts(executor, &line, opts) {
            if opts.errexit {
                return Err(e);
            }
            eprintln!("nash: {e}");
        }
    }
    Ok(())
}

/// Run a host-filesystem script file.
fn run_script(executor: &mut Executor, path: &str, opts: &ShellOpts) -> Result<()> {
    let host_path = std::path::Path::new(path);
    if !host_path.exists() {
        bail!("nash: {}: No such file or directory", path);
    }
    if !host_path.is_file() {
        bail!("nash: {}: Not a regular file", path);
    }

    let file =
        std::fs::File::open(host_path).with_context(|| format!("cannot open script: {}", path))?;
    let reader = io::BufReader::new(file);

    let mut line_no = 0usize;
    for line_result in reader.lines() {
        line_no += 1;
        let line = line_result.with_context(|| format!("read error at {}:{}", path, line_no))?;
        if line_no == 1 && line.starts_with("#!") {
            continue;
        }
        if let Err(e) = run_line_opts(executor, &line, opts) {
            if opts.errexit {
                return Err(e);
            }
            eprintln!("nash: {}:{}: {}", path, line_no, e);
        }
    }
    Ok(())
}

/// Interactive REPL.
fn run_repl(executor: &mut Executor, username: &str, opts: &ShellOpts) -> Result<()> {
    let mut rl = DefaultEditor::new()?;

    println!(
    "Nash — Not A Shell  │  logged in as \x1b[1;32m{}\x1b[0m  │  type \x1b[1mhelp\x1b[0m or Ctrl-D to exit",
    username
);
    println!();

    loop {
        let cwd = executor.cwd().to_string();
        let sigil = if username == "root" { "#" } else { "$" };
        // \x01 / \x02 = RL_PROMPT_START/END_IGNORE so rustyline counts width correctly.
        let prompt = format!(
            "\x01\x1b[1;32m\x02{}@nash\x01\x1b[0m\x02:\x01\x1b[1;34m\x02{}\x01\x1b[0m\x02{} ",
            username, cwd, sigil
        );

        match rl.readline(&prompt) {
            Ok(line) => {
                let _ = rl.add_history_entry(&line);
                let trimmed = line.trim();
                if trimmed == "exit" || trimmed == "quit" {
                    println!("logout");
                    break;
                }
                if let Err(e) = run_line_opts(executor, trimmed, opts) {
                    eprintln!("nash: {e}");
                }
                executor.sync_pwd();
            }
            Err(ReadlineError::Interrupted) => println!("^C"),
            Err(ReadlineError::Eof) => {
                println!("\nlogout");
                break;
            }
            Err(e) => {
                eprintln!("nash: readline error: {e}");
                break;
            }
        }
    }

    Ok(())
}

// ─── Misc helpers ──────────────────────────────────────────────────────────────

fn validate_username(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("username cannot be empty");
    }
    if name.len() > 32 {
        bail!("username too long (max 32 chars)");
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        bail!(
            "username '{}' contains invalid characters (use a-z, 0-9, _, -)",
            name
        );
    }
    Ok(())
}

fn parse_bind(spec: &str) -> Result<(String, String)> {
    let (host, vfs_path) = spec
        .split_once(':')
        .with_context(|| format!("--bind expects HOST:VFS format, got: '{}'", spec))?;
    Ok((host.to_string(), vfs_path.to_string()))
}
