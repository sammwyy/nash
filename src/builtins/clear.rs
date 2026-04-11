use crate::runtime::context::Context;
use shellframe::Output;
use anyhow::Result;

pub fn run(_args: &[String], _ctx: &mut Context, _stdin: &str) -> Result<Output> {
    // ANSI escape: clear screen and move cursor to top-left
    Ok(Output::success("\x1b[2J\x1b[H".into()))
}
