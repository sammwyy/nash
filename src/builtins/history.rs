use super::Builtin;
use crate::runtime::{Context, Output};
use anyhow::Result;

pub struct History;

impl Builtin for History {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        // Optionally limit to last N entries
        let limit: Option<usize> = args.first().and_then(|s| s.parse().ok());

        let entries = &ctx.history;
        let start = match limit {
            Some(n) if n < entries.len() => entries.len() - n,
            _ => 0,
        };

        let mut out = String::new();
        for (i, line) in entries[start..].iter().enumerate() {
            out.push_str(&format!("{:>5}  {}\n", start + i + 1, line));
        }

        Ok(Output::success(out))
    }
}
