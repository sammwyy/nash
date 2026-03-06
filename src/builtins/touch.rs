use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Touch;

impl Builtin for Touch {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        for arg in args {
            if arg.starts_with('-') {
                continue;
            }
            let abs = VfsPath::join(&ctx.cwd, arg);
            ctx.vfs.touch(&abs)?;
        }
        Ok(Output::success(""))
    }
}
