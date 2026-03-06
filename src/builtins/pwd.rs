use super::Builtin;
use crate::runtime::{Context, Output};
use anyhow::Result;

pub struct Pwd;

impl Builtin for Pwd {
    fn run(&self, _args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        // Use $PWD env if set (kept in sync by cd / sync_pwd), else ctx.cwd
        let dir = ctx
            .env
            .get("PWD")
            .cloned()
            .unwrap_or_else(|| ctx.cwd.clone());
        Ok(Output::success(format!("{}\n", dir)))
    }
}
