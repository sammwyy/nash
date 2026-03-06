use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Stat;

impl Builtin for Stat {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        if args.is_empty() {
            return Ok(Output::error(1, "", "stat: missing operand\n"));
        }

        let mut out = String::new();
        for arg in args {
            if arg.starts_with('-') {
                continue;
            }
            let abs = VfsPath::join(&ctx.cwd, arg);

            if !ctx.vfs.exists(&abs) {
                out.push_str(&format!(
                    "stat: cannot stat '{}': No such file or directory\n",
                    arg
                ));
                continue;
            }

            let is_dir = ctx.vfs.is_dir(&abs);
            let (size, file_type) = if is_dir {
                (0usize, "directory")
            } else {
                let sz = ctx.vfs.read(&abs).map(|b| b.len()).unwrap_or(0);
                (sz, "regular file")
            };

            out.push_str(&format!("  File: {}\n", abs));
            out.push_str(&format!(
                "  Size: {:<15} Blocks: {:<10} IO Block: 4096\n",
                size,
                (size + 511) / 512
            ));
            out.push_str(&format!("  Type: {}\n", file_type));
            out.push_str(&format!(
                "Access: (0644/-rw-r--r--)  Uid: (1000/user)  Gid: (1000/user)\n"
            ));
            out.push_str(&format!("  VFS: in-memory sandbox (no real inode)\n"));
        }

        Ok(Output::success(out))
    }
}
