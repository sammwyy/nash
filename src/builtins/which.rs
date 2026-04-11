use crate::runtime::context::Context;
use shellframe::Output;
use anyhow::Result;

const BUILTINS: &[&str] = &[
    "cat", "cd", "clear", "cp", "cut", "echo", "env", "export", "false", "file", "find", "grep",
    "head", "help", "history", "jq", "ls", "mkdir", "mv", "pwd", "rm", "sed", "sort", "stat",
    "tail", "test", "touch", "tree", "true", "uniq", "unset", "wc", "which", "[",
];

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    if args.is_empty() {
        return Ok(Output::error(1, "".into(), "which: missing argument\n".into()));
    }

    let mut out = String::new();
    let mut exit_code = 0i32;

    for name in args {
        if BUILTINS.contains(&name.as_str()) {
            out.push_str(&format!("{} (nash builtin)\n", name));
        } else if let Some(path) = ctx.state.allowed_bins.get(name) {
            out.push_str(&format!("{} (allowed host binary: {})\n", name, path));
        } else {
            out.push_str(&format!("{}: not found\n", name));
            exit_code = 1;
        }
    }

    Ok(Output::new(exit_code, out, "".into()))
}
