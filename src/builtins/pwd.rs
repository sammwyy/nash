use crate::runtime::context::Context;
use shellframe::Output;
use anyhow::Result;

pub fn run(_args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    let dir = ctx
        .env
        .get("PWD")
        .cloned()
        .unwrap_or_else(|| ctx.get_cwd().to_string());
    Ok(Output::success(format!("{}\n", dir)))
}
