use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Rm;

impl Builtin for Rm {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        let mut recursive = false;
        let mut targets: Vec<String> = Vec::new();

        for arg in args {
            if arg == "-r" || arg == "-rf" || arg == "-R" {
                recursive = true;
            } else if arg.starts_with('-') { /* ignore unknown flags */
            } else {
                targets.push(arg.clone());
            }
        }

        if targets.is_empty() {
            return Ok(Output::error(1, "", "rm: missing operand\n"));
        }

        for target in &targets {
            let abs = VfsPath::join(&ctx.cwd, target);
            if recursive {
                ctx.vfs.remove_recursive(&abs)?;
            } else {
                ctx.vfs.remove(&abs)?;
            }
        }

        Ok(Output::success(""))
    }
}
