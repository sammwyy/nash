use super::Builtin;
use crate::runtime::{Context, Output};
use anyhow::Result;

pub struct Clear;

impl Builtin for Clear {
    fn run(&self, _args: &[String], _ctx: &mut Context, _stdin: &str) -> Result<Output> {
        // ANSI escape: clear screen and move cursor to top-left
        Ok(Output::success("\x1b[2J\x1b[H"))
    }
}
