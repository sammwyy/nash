use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    let mut start = ctx.get_cwd().to_string();
    let mut name_pattern: Option<String> = None;
    let mut type_filter: Option<char> = None;
    let mut max_depth: Option<usize> = None;

    let mut iter = args.iter().peekable();
    if let Some(first) = iter.peek() {
        if !first.starts_with('-') {
            start = VfsPath::join(ctx.get_cwd(), first);
            iter.next();
        }
    }

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-name" => {
                name_pattern = iter.next().cloned();
            }
            "-type" => {
                type_filter = iter.next().and_then(|s| s.chars().next());
            }
            "-maxdepth" => {
                max_depth = iter.next().and_then(|s| s.parse().ok());
            }
            _ => {}
        }
    }

    let mut out = String::new();
    find_recursive(
        &ctx.state.vfs,
        &start,
        &name_pattern,
        type_filter,
        max_depth,
        0,
        &mut out,
    );

    Ok(Output::success(out))
}

fn find_recursive(
    vfs: &crate::vfs::Vfs,
    path: &str,
    name_pattern: &Option<String>,
    type_filter: Option<char>,
    max_depth: Option<usize>,
    depth: usize,
    out: &mut String,
) {
    if let Some(max) = max_depth {
        if depth > max {
            return;
        }
    }

    let is_dir = vfs.is_dir(path);
    let name = VfsPath::basename(path);

    let name_match = match name_pattern {
        Some(pat) => glob_match(pat, &name),
        None => true,
    };
    let type_match = match type_filter {
        Some('f') => !is_dir,
        Some('d') => is_dir,
        _ => true,
    };

    if name_match && type_match {
        out.push_str(path);
        out.push('\n');
    }

    if is_dir {
        if let Ok(entries) = vfs.list_dir(path) {
            for entry in entries {
                let child = format!("{}/{}", path.trim_end_matches('/'), entry.name);
                find_recursive(
                    vfs,
                    &child,
                    name_pattern,
                    type_filter,
                    max_depth,
                    depth + 1,
                    out,
                );
            }
        }
    }
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_inner(&p, &n)
}

fn glob_inner(p: &[char], n: &[char]) -> bool {
    match (p.first(), n.first()) {
        (None, None) => true,
        (Some('*'), _) => {
            glob_inner(&p[1..], n) || (!n.is_empty() && glob_inner(p, &n[1..]))
        }
        (Some('?'), Some(_)) => glob_inner(&p[1..], &n[1..]),
        (Some(pc), Some(nc)) if pc == nc => glob_inner(&p[1..], &n[1..]),
        _ => false,
    }
}
