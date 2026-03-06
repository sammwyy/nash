use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct Grep;

impl Builtin for Grep {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        let mut invert = false;
        let mut ignore_case = false;
        let mut line_number = false;
        let mut pattern: Option<String> = None;
        let mut files: Vec<String> = Vec::new();

        for arg in args {
            match arg.as_str() {
                "-v" | "--invert-match" => invert = true,
                "-i" | "--ignore-case" => ignore_case = true,
                "-n" | "--line-number" => line_number = true,
                s if s.starts_with('-') => {} // ignore unknown
                _ => {
                    if pattern.is_none() {
                        pattern = Some(arg.clone());
                    } else {
                        files.push(arg.clone());
                    }
                }
            }
        }

        let pat = match pattern {
            Some(p) => p,
            None => {
                return Ok(Output::error(
                    1,
                    "",
                    "grep: missing pattern
",
                ))
            }
        };

        let text = if files.is_empty() {
            stdin.to_string()
        } else {
            let mut buf = String::new();
            for f in &files {
                let abs = VfsPath::join(&ctx.cwd, f);
                match ctx.vfs.read_to_string(&abs) {
                    Ok(c) => buf.push_str(&c),
                    Err(e) => {
                        return Ok(Output::error(
                            1,
                            "",
                            &format!(
                                "grep: {}
",
                                e
                            ),
                        ))
                    }
                }
            }
            buf
        };

        let mut out = String::new();
        let mut matched = false;
        for (i, line) in text.lines().enumerate() {
            let haystack = if ignore_case {
                line.to_lowercase()
            } else {
                line.to_string()
            };
            let needle = if ignore_case {
                pat.to_lowercase()
            } else {
                pat.clone()
            };
            let found = haystack.contains(&needle);
            let show = if invert { !found } else { found };
            if show {
                matched = true;
                if line_number {
                    out.push_str(&format!(
                        "{}:{}
",
                        i + 1,
                        line
                    ));
                } else {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }

        if matched {
            Ok(Output::success(out))
        } else {
            Ok(Output::error(1, "", ""))
        }
    }
}
