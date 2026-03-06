use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Cat;

impl Builtin for Cat {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        if args.is_empty() {
            // Read from stdin
            return Ok(Output::success(stdin));
        }

        let mut out = String::new();
        for arg in args {
            if arg.starts_with('-') {
                continue;
            }
            let abs = VfsPath::join(&ctx.cwd, arg);
            match ctx.vfs.read_to_string(&abs) {
                Ok(content) => out.push_str(&content),
                Err(e) => {
                    return Ok(Output::error(
                        1,
                        "",
                        &format!(
                            "cat: {}: {}
",
                            arg, e
                        ),
                    ));
                }
            }
        }

        Ok(Output::success(out))
    }
}
