use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub fn run(args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
    let mut count = false;
    let mut duplicates_only = false;
    let mut unique_only = false;
    let mut files: Vec<String> = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-c" => count = true,
            "-d" => duplicates_only = true,
            "-u" => unique_only = true,
            s if s.starts_with('-') => {}
            _ => files.push(arg.clone()),
        }
    }

    let text = if files.is_empty() {
        stdin.to_string()
    } else {
        let mut buf = String::new();
        for f in &files {
            let abs = VfsPath::join(ctx.get_cwd(), f);
            buf.push_str(&ctx.state.vfs.read_to_string(&abs)?);
        }
        buf
    };

    let mut out = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let current = lines[i];
        let mut run_count = 1usize;
        while i + run_count < lines.len() && lines[i + run_count] == current {
            run_count += 1;
        }

        let should_print = match (duplicates_only, unique_only) {
            (true, _) => run_count > 1,
            (_, true) => run_count == 1,
            _ => true,
        };

        if should_print {
            if count {
                out.push_str(&format!("{:>7} {}\n", run_count, current));
            } else {
                out.push_str(current);
                out.push('\n');
            }
        }

        i += run_count;
    }

    Ok(Output::success(out))
}
