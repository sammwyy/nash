use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Tree;

impl Builtin for Tree {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        let mut max_depth: Option<usize> = None;
        let mut show_hidden = false;
        let mut start = ctx.cwd.clone();

        let mut iter = args.iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-L" => {
                    max_depth = iter.next().and_then(|s| s.parse().ok());
                }
                "-a" => show_hidden = true,
                s if s.starts_with('-') => {}
                _ => start = VfsPath::join(&ctx.cwd, arg),
            }
        }

        let mut out = String::new();
        out.push_str(&start);
        out.push('\n');

        let mut dirs = 0usize;
        let mut files_count = 0usize;

        tree_recursive(
            &ctx.vfs,
            &start,
            "",
            max_depth,
            0,
            show_hidden,
            &mut out,
            &mut dirs,
            &mut files_count,
        );

        out.push('\n');
        out.push_str(&format!(
            "{} director{}, {} file{}\n",
            dirs,
            if dirs == 1 { "y" } else { "ies" },
            files_count,
            if files_count == 1 { "" } else { "s" }
        ));

        Ok(Output::success(out))
    }
}

fn tree_recursive(
    vfs: &crate::vfs::Vfs,
    path: &str,
    prefix: &str,
    max_depth: Option<usize>,
    depth: usize,
    show_hidden: bool,
    out: &mut String,
    dirs: &mut usize,
    files: &mut usize,
) {
    if let Some(max) = max_depth {
        if depth >= max {
            return;
        }
    }

    let entries = match vfs.list_dir(path) {
        Ok(e) => e,
        Err(_) => return,
    };

    let entries: Vec<_> = entries
        .into_iter()
        .filter(|e| show_hidden || !e.name.starts_with('.'))
        .collect();

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i + 1 == entries.len();
        let connector = if is_last { "└── " } else { "├── " };
        let extension = if is_last { "    " } else { "│   " };

        if entry.is_dir {
            out.push_str(&format!("{}{}{}/\n", prefix, connector, entry.name));
            *dirs += 1;
            let child_path = format!("{}/{}", path.trim_end_matches('/'), entry.name);
            tree_recursive(
                vfs,
                &child_path,
                &format!("{}{}", prefix, extension),
                max_depth,
                depth + 1,
                show_hidden,
                out,
                dirs,
                files,
            );
        } else {
            out.push_str(&format!("{}{}{}\n", prefix, connector, entry.name));
            *files += 1;
        }
    }
}
