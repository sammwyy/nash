use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Ls;

impl Builtin for Ls {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        // Simple flag/path parsing
        let mut show_hidden = false;
        let mut long_format = false;
        let mut paths: Vec<String> = Vec::new();

        for arg in args {
            if arg.starts_with('-') {
                if arg.contains('a') {
                    show_hidden = true;
                }
                if arg.contains('l') {
                    long_format = true;
                }
            } else {
                paths.push(arg.clone());
            }
        }

        if paths.is_empty() {
            paths.push(ctx.cwd.clone());
        }

        let mut out = String::new();
        for path in &paths {
            let abs = VfsPath::join(&ctx.cwd, path);
            if ctx.vfs.is_dir(&abs) {
                let entries = ctx.vfs.list_dir(&abs)?;
                if paths.len() > 1 {
                    out.push_str(&format!("{}:\n", abs));
                }
                for entry in entries {
                    if !show_hidden && entry.name.starts_with('.') {
                        continue;
                    }
                    if long_format {
                        let kind = if entry.is_dir { "d" } else { "-" };
                        if entry.is_dir {
                            out.push_str(&format!("{}rwxr-xr-x  {}\n", kind, entry.name));
                        } else {
                            let size = ctx
                                .vfs
                                .read(&format!("{}/{}", abs, entry.name))
                                .map(|b| b.len())
                                .unwrap_or(0);
                            out.push_str(&format!(
                                "{}rw-r--r--  {:>8}  {}\n",
                                kind, size, entry.name
                            ));
                        }
                    } else {
                        if entry.is_dir {
                            out.push_str(&format!("{}/\n", entry.name));
                        } else {
                            out.push_str(&format!("{}\n", entry.name));
                        }
                    }
                }
            } else if ctx.vfs.exists(&abs) {
                out.push_str(&format!("{}\n", abs));
            } else {
                return Ok(Output::error(
                    1,
                    "",
                    &format!("ls: cannot access '{}': No such file or directory\n", path),
                ));
            }
        }

        Ok(Output::success(out))
    }
}
