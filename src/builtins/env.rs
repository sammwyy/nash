use super::Builtin;
use crate::runtime::{Context, Output};
use anyhow::Result;

pub struct EnvCmd;
pub struct Export;
pub struct Unset;

impl Builtin for EnvCmd {
    fn run(&self, _args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        let mut out = String::new();
        for (k, v) in &ctx.env {
            out.push_str(&format!("{}={}\n", k, v));
        }
        Ok(Output::success(out))
    }
}

impl Builtin for Export {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        for arg in args {
            if let Some((k, v)) = arg.split_once('=') {
                ctx.env.insert(k.to_string(), v.to_string());
            }
        }
        Ok(Output::success(""))
    }
}

impl Builtin for Unset {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        for arg in args {
            ctx.env.swap_remove(arg);
        }
        Ok(Output::success(""))
    }
}
