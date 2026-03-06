use super::Builtin;
use crate::runtime::{Context, Output};
use anyhow::Result;

pub struct Echo;

impl Builtin for Echo {
    fn run(&self, args: &[String], _ctx: &mut Context, _stdin: &str) -> Result<Output> {
        let mut no_newline = false;
        let mut parts: Vec<&str> = Vec::new();

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if arg == "-n" {
                no_newline = true;
            } else if arg == "-e" {
                // enable escape interpretation (handled below)
            } else {
                parts.push(arg.as_str());
            }
        }

        let text = parts.join(" ");
        // Basic escape sequence handling (like bash echo -e)
        let expanded = expand_escapes(&text);

        let out = if no_newline {
            expanded
        } else {
            format!("{}\n", expanded)
        };

        Ok(Output::success(out))
    }
}

fn expand_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}
