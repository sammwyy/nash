use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Wc;

impl Builtin for Wc {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        let mut count_lines = false;
        let mut count_words = false;
        let mut count_bytes = false;
        let mut files: Vec<String> = Vec::new();

        for arg in args {
            match arg.as_str() {
                "-l" => count_lines = true,
                "-w" => count_words = true,
                "-c" => count_bytes = true,
                s if s.starts_with('-') => {}
                _ => files.push(arg.clone()),
            }
        }

        // Default: count everything
        if !count_lines && !count_words && !count_bytes {
            count_lines = true;
            count_words = true;
            count_bytes = true;
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

        let lines = text.lines().count();
        let words = text.split_whitespace().count();
        let bytes = text.len();

        let mut out = String::new();
        if count_lines {
            out.push_str(&format!("{:>8}", lines));
        }
        if count_words {
            out.push_str(&format!("{:>8}", words));
        }
        if count_bytes {
            out.push_str(&format!("{:>8}", bytes));
        }
        out.push('\n');

        Ok(Output::success(out))
    }
}
