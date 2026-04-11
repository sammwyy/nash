use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub fn run(args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
    let mut delimiter = '\t';
    let mut fields: Option<Vec<usize>> = None;
    let mut chars: Option<Vec<usize>> = None;
    let mut files: Vec<String> = Vec::new();

    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-d" => {
                if let Some(d) = iter.next() {
                    delimiter = d.chars().next().unwrap_or('\t');
                }
            }
            "-f" => {
                if let Some(spec) = iter.next() {
                    fields = Some(parse_field_spec(spec));
                }
            }
            "-c" => {
                if let Some(spec) = iter.next() {
                    chars = Some(parse_field_spec(spec));
                }
            }
            s if s.starts_with('-') => {}
            _ => files.push(arg.clone()),
        }
    }

    if fields.is_none() && chars.is_none() {
        bail!("cut: missing -f or -c option");
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
    for line in text.lines() {
        if let Some(ref flds) = fields {
            let parts: Vec<&str> = line.split(delimiter).collect();
            let selected: Vec<&str> = flds
                .iter()
                .filter_map(|&i| parts.get(i.saturating_sub(1)).copied())
                .collect();
            out.push_str(&selected.join(&delimiter.to_string()));
            out.push('\n');
        } else if let Some(ref ch_idxs) = chars {
            let chars_vec: Vec<char> = line.chars().collect();
            let selected: String = ch_idxs
                .iter()
                .filter_map(|&i| chars_vec.get(i.saturating_sub(1)).copied())
                .collect();
            out.push_str(&selected);
            out.push('\n');
        }
    }

    Ok(Output::success(out))
}

fn parse_field_spec(spec: &str) -> Vec<usize> {
    let mut result = Vec::new();
    for part in spec.split(',') {
        if let Some((start, end)) = part.split_once('-') {
            let s: usize = start.parse().unwrap_or(1);
            let e: usize = end.parse().unwrap_or(1000);
            for i in s..=e {
                result.push(i);
            }
        } else if let Ok(n) = part.parse::<usize>() {
            result.push(n);
        }
    }
    result.sort_unstable();
    result.dedup();
    result
}
