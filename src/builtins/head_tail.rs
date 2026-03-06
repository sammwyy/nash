use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

fn get_text(_args: &[String], files: &[String], ctx: &mut Context, stdin: &str) -> Result<String> {
    if files.is_empty() {
        Ok(stdin.to_string())
    } else {
        let mut buf = String::new();
        for f in files {
            let abs = VfsPath::join(&ctx.cwd, f);
            buf.push_str(&ctx.vfs.read_to_string(&abs)?);
        }
        Ok(buf)
    }
}

pub struct Head;

impl Builtin for Head {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        let mut n = 10usize;
        let mut files: Vec<String> = Vec::new();
        let mut next_is_n = false;

        for arg in args {
            if next_is_n {
                n = arg.parse().unwrap_or(10);
                next_is_n = false;
            } else if arg == "-n" {
                next_is_n = true;
            } else if arg.starts_with("-n") {
                n = arg[2..].parse().unwrap_or(10);
            } else if !arg.starts_with('-') {
                files.push(arg.clone());
            }
        }

        let text = get_text(args, &files, ctx, stdin)?;
        let out: Vec<_> = text.lines().take(n).collect();
        Ok(Output::success(out.join("\n") + "\n"))
    }
}

pub struct Tail;

impl Builtin for Tail {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        let mut n = 10usize;
        let mut files: Vec<String> = Vec::new();
        let mut next_is_n = false;

        for arg in args {
            if next_is_n {
                n = arg.parse().unwrap_or(10);
                next_is_n = false;
            } else if arg == "-n" {
                next_is_n = true;
            } else if arg.starts_with("-n") {
                n = arg[2..].parse().unwrap_or(10);
            } else if !arg.starts_with('-') {
                files.push(arg.clone());
            }
        }

        let text = get_text(args, &files, ctx, stdin)?;
        let lines: Vec<_> = text.lines().collect();
        let skip = if lines.len() > n { lines.len() - n } else { 0 };
        let out: Vec<&str> = lines[skip..].iter().copied().collect();
        Ok(Output::success(out.join("\n") + "\n"))
    }
}
