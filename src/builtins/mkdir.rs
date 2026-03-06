use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Mkdir;

impl Builtin for Mkdir {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        let mut parents = false;
        let mut dirs: Vec<String> = Vec::new();

        for arg in args {
            if arg == "-p" {
                parents = true;
            } else {
                dirs.push(arg.clone());
            }
        }

        if dirs.is_empty() {
            return Ok(Output::error(1, "", "mkdir: missing operand\n"));
        }

        for dir in &dirs {
            let abs = VfsPath::join(&ctx.cwd, dir);
            if parents {
                ctx.vfs.mkdir_p(&abs)?;
            } else {
                ctx.vfs.mkdir(&abs)?;
            }
        }

        Ok(Output::success(""))
    }
}
