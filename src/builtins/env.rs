use crate::runtime::context::Context;
use shellframe::Output;
use anyhow::Result;

pub fn run_env(_args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    let mut out = String::new();
    for (k, v) in &ctx.env {
        out.push_str(&format!("{}={}\n", k, v));
    }
    Ok(Output::success(out))
}

pub fn run_export(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            ctx.env.insert(k.to_string(), v.to_string());
        }
    }
    Ok(Output::success("".into()))
}

pub fn run_unset(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    for arg in args {
        ctx.env.swap_remove(arg);
    }
    Ok(Output::success("".into()))
}
