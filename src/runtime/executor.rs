use anyhow::{bail, Result};
use indexmap::IndexMap;

use crate::builtins;
use crate::vfs::mount::MountOptions;
use crate::vfs::path::VfsPath;
use crate::vfs::Vfs;

use super::context::{Context, NashState};
use shellframe::{Shell, RedirectMode, Expr, Output};

pub struct ExecutorConfig {
    pub cwd: String,
    pub env: IndexMap<String, String>,
    pub mounts: Vec<(String, String, MountOptions)>,
    pub allowed_bins: IndexMap<String, String>,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            cwd: String::new(),
            env: IndexMap::new(),
            mounts: Vec::new(),
            allowed_bins: IndexMap::new(),
        }
    }
}

pub struct Executor {
    shell: Shell<NashState>,
}

impl Executor {
    pub fn new(config: ExecutorConfig, username: &str) -> Result<Self> {
        let mut vfs = Vfs::new();

        for (host, vfs_path, opts) in config.mounts {
            if !std::path::Path::new(&host).exists() {
                bail!("mount: host path does not exist: {}", host);
            }
            vfs.mount(host, vfs_path, opts)?;
        }

        let home_dir = format!("/home/{}", username);
        for dir in &["/", "/bin", "/sbin", "/usr", "/usr/bin", "/usr/sbin", "/usr/local", "/usr/local/bin", "/etc", "/var", "/var/log", "/var/tmp", "/tmp", "/lib", "/lib64", "/opt", "/root", "/proc", "/dev", home_dir.as_str()] {
            vfs.mkdir_p(dir)?;
        }
        for sub in &["Desktop", "Documents", "Downloads"] {
            vfs.mkdir_p(&format!("{}/{}", home_dir, sub))?;
        }

        vfs.write_str(&format!("{}/.nashrc", home_dir), &format!("# ~/.nashrc\nexport USER={}\n", username))?;
        vfs.write_str("/etc/hostname", "nash\n")?;
        vfs.write_str("/etc/shells", "/bin/nash\n/bin/sh\n")?;
        vfs.write_str(&format!("{}/welcome.txt", home_dir), "Welcome to Nash!\nType 'help' to see available commands.\n")?;

        let cwd = if config.cwd.is_empty() { home_dir.clone() } else { config.cwd.clone() };
        vfs.mkdir_p(&cwd)?;

        let env = config.env;
        // shellframe Context::new handles basic defaults and PWD if passed, 
        // but we'll let nash continue managing its own baseline here if needed.
        
        let state = NashState::new(vfs, config.allowed_bins);
        let ctx = Context::new(cwd, env, state);
        let mut shell = Shell::new(ctx);

        // Register builtins
        builtins::register_all(&mut shell);

        // Set hook for external binaries
        shell.set_hook(|name, args, context, stdin| {
            use std::io::Write;
            use std::process::{Command, Stdio};
            
            if let Some(real_path) = context.state.allowed_bins.get(name).cloned() {
                let mut child = Command::new(real_path)
                    .args(args)
                    .envs(&context.env)
                    // Ensure the binary runs in the correct VFS-mapped CWD if possible, 
                    // though for host binaries context.get_cwd() is a VFS path.
                    // If it's a bind mount, we might want to map it back, but 
                    // for now we just use the VFS path as a hint or ignore if it doesn't exist on host.
                    // Actually, most host binaries won't understand nash's VFS paths.
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;

                if let Some(mut child_stdin) = child.stdin.take() {
                    child_stdin.write_all(stdin.as_bytes())?;
                }

                let output = child.wait_with_output()?;
                let exit_code = output.status.code().unwrap_or(1);

                Ok(Output::new(
                    exit_code,
                    String::from_utf8_lossy(&output.stdout).to_string(),
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            } else {
                Ok(Output::error(127, "".into(), format!("nash: command not found: {}\n", name)))
            }
        });

        // Set redirect handler
        shell.set_redirect_handler(|sh, expr, file, mode, stdin| {
             match mode {
                RedirectMode::Input => {
                    let abs = VfsPath::join(sh.context.get_cwd(), file);
                    let content = match sh.context.state.vfs.read_to_string(&abs) {
                        Ok(c) => c,
                        Err(e) => {
                            return Ok(Output::error(1, "".into(), format!("nash: {}\n", e)));
                        }
                    };
                    sh.eval(expr, &content)
                }
                RedirectMode::Overwrite | RedirectMode::Append => {
                    let output = sh.eval(expr, stdin)?;
                    let abs = VfsPath::join(sh.context.get_cwd(), file);
                    if *mode == RedirectMode::Overwrite {
                        sh.context.state.vfs.write_str(&abs, &output.stdout)?;
                    } else {
                        sh.context.state.vfs.append(&abs, output.stdout.as_bytes().to_vec())?;
                    }
                    Ok(Output::new(output.exit_code, String::new(), output.stderr))
                }
            }
        });

        Ok(Executor { shell })
    }

    pub fn cwd(&self) -> &str {
        self.shell.context.get_cwd()
    }

    pub fn push_history(&mut self, line: String) {
        self.shell.context.state.history.push(line);
    }

    pub fn sync_pwd(&mut self) {
        // Now handled by set_cwd, but we can keep it as a no-op or sanity check.
        let cwd = self.shell.context.get_cwd().to_string();
        self.shell.context.set_cwd(cwd);
    }

    pub fn vfs_exists(&self, path: &str) -> bool {
        self.shell.context.state.vfs.exists(path)
    }

    pub fn vfs_read_string(&self, path: &str) -> Result<String> {
        self.shell.context.state.vfs.read_to_string(path)
    }

    pub fn execute(&mut self, expr: &Expr) -> Result<Output> {
         self.shell.eval(expr, "")
    }
}
