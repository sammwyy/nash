use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub struct Sed;

impl Builtin for Sed {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        // Supported: sed 's/old/new/[g]' [file...]
        //            sed -n 's/old/new/p' [file...]  (print only matching)
        //            sed 'Nd'                         (delete line N)
        //            sed 'Np'                         (print line N)
        let mut script: Option<String> = None;
        let mut silent = false;
        let mut files: Vec<String> = Vec::new();

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-n" => silent = true,
                "-e" => {
                    script = iter.next().cloned();
                }
                s if s.starts_with('-') => {}
                _ => {
                    if script.is_none() {
                        script = Some(arg.clone());
                    } else {
                        files.push(arg.clone());
                    }
                }
            }
        }

        let expr = match script {
            Some(s) => s,
            None => bail!("sed: no script specified"),
        };

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

        let out = apply_sed(&expr, &text, silent)?;
        Ok(Output::success(out))
    }
}

fn apply_sed(expr: &str, text: &str, silent: bool) -> Result<String> {
    let mut out = String::new();

    // Bare 'd' — delete all lines
    if expr.trim() == "d" {
        return Ok(String::new());
    }

    // s/old/new/[flags]
    if expr.starts_with('s') && expr.len() > 1 {
        let delim = expr.chars().nth(1).unwrap_or('/');
        let parts: Vec<&str> = expr[2..].splitn(3, delim).collect();
        if parts.len() < 2 {
            bail!("sed: invalid substitution expression");
        }
        let pattern = parts[0];
        let replacement = parts[1];
        let flags = parts.get(2).unwrap_or(&"");
        let global = flags.contains('g');
        let print_match = flags.contains('p');

        for line in text.lines() {
            let (new_line, matched) = if global {
                let r = line.replace(pattern, replacement);
                let m = r != line;
                (r, m)
            } else {
                match line.find(pattern) {
                    Some(idx) => {
                        let r = format!(
                            "{}{}{}",
                            &line[..idx],
                            replacement,
                            &line[idx + pattern.len()..]
                        );
                        (r, true)
                    }
                    None => (line.to_string(), false),
                }
            };

            if !silent {
                out.push_str(&new_line);
                out.push('\n');
            } else if print_match && matched {
                out.push_str(&new_line);
                out.push('\n');
            }
        }
        return Ok(out);
    }

    // Nd — delete line N
    if let Some(stripped) = expr.strip_suffix('d') {
        if let Ok(n) = stripped.trim().parse::<usize>() {
            for (i, line) in text.lines().enumerate() {
                if i + 1 != n {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            return Ok(out);
        }
    }

    // Np — print line N
    if let Some(stripped) = expr.strip_suffix('p') {
        if let Ok(n) = stripped.trim().parse::<usize>() {
            for (i, line) in text.lines().enumerate() {
                if i + 1 == n {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            return Ok(out);
        }
    }

    bail!("sed: unsupported expression: {}", expr)
}
