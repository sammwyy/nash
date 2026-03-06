use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Sort;

impl Builtin for Sort {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        let mut reverse = false;
        let mut unique = false;
        let mut files: Vec<String> = Vec::new();

        for arg in args {
            match arg.as_str() {
                "-r" => reverse = true,
                "-u" => unique = true,
                s if s.starts_with('-') => {}
                _ => files.push(arg.clone()),
            }
        }

        let text = if files.is_empty() {
            stdin.to_string()
        } else {
            let mut buf = String::new();
            for f in &files {
                let abs = VfsPath::join(&ctx.cwd, f);
                buf.push_str(&ctx.vfs.read_to_string(&abs)?);
            }
            buf
        };

        let mut lines: Vec<&str> = text.lines().collect();
        lines.sort();
        if reverse {
            lines.reverse();
        }
        if unique {
            lines.dedup();
        }

        let out = lines.join("\n") + "\n";
        Ok(Output::success(out))
    }
}
