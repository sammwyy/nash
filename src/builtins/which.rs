use super::Builtin;
use crate::runtime::{Context, Output};
use anyhow::Result;

/// All built-in command names known to Nash.
const BUILTINS: &[&str] = &[
    "cat", "cd", "clear", "cp", "cut", "echo", "env", "export", "false", "file", "find", "grep",
    "head", "help", "history", "jq", "ls", "mkdir", "mv", "pwd", "rm", "sed", "sort", "stat",
    "tail", "tail", "test", "touch", "tree", "true", "uniq", "unset", "wc", "which", "[",
];

pub struct Which;

impl Builtin for Which {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        if args.is_empty() {
            return Ok(Output::error(1, "", "which: missing argument\n"));
        }

        let mut out = String::new();
        let mut exit_code = 0i32;

        for name in args {
            if BUILTINS.contains(&name.as_str()) {
                out.push_str(&format!("{} (nash builtin)\n", name));
            } else if let Some(path) = ctx.allowed_bins.get(name) {
                out.push_str(&format!("{} (allowed host binary: {})\n", name, path));
            } else {
                out.push_str(&format!("{}: not found\n", name));
                exit_code = 1;
            }
        }

        Ok(Output {
            stdout: out,
            stderr: String::new(),
            exit_code,
        })
    }
}
