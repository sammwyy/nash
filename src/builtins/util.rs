use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct True;
pub struct False;
pub struct Test;

impl Builtin for True {
    fn run(&self, _args: &[String], _ctx: &mut Context, _stdin: &str) -> Result<Output> {
        Ok(Output::success(""))
    }
}

impl Builtin for False {
    fn run(&self, _args: &[String], _ctx: &mut Context, _stdin: &str) -> Result<Output> {
        Ok(Output::error(1, "", ""))
    }
}

impl Builtin for Test {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        // Strip trailing ] if invoked as [
        let args: Vec<_> = args.iter().filter(|a| *a != "]").collect();

        let result = match args.as_slice() {
            // Unary: -f file, -d file, -e file, -z str, -n str
            [flag, operand] if *flag == "-f" => {
                let abs = VfsPath::join(&ctx.cwd, operand);
                ctx.vfs.exists(&abs) && !ctx.vfs.is_dir(&abs)
            }
            [flag, operand] if *flag == "-d" => {
                let abs = VfsPath::join(&ctx.cwd, operand);
                ctx.vfs.is_dir(&abs)
            }
            [flag, operand] if *flag == "-e" => {
                let abs = VfsPath::join(&ctx.cwd, operand);
                ctx.vfs.exists(&abs)
            }
            [flag, operand] if *flag == "-z" => operand.is_empty(),
            [flag, operand] if *flag == "-n" => !operand.is_empty(),
            // Binary string comparisons
            [lhs, op, rhs] if *op == "=" || *op == "==" => lhs == rhs,
            [lhs, op, rhs] if *op == "!=" => lhs != rhs,
            // Numeric comparisons
            [lhs, op, rhs] if *op == "-eq" => parse_i(lhs) == parse_i(rhs),
            [lhs, op, rhs] if *op == "-ne" => parse_i(lhs) != parse_i(rhs),
            [lhs, op, rhs] if *op == "-lt" => parse_i(lhs) < parse_i(rhs),
            [lhs, op, rhs] if *op == "-le" => parse_i(lhs) <= parse_i(rhs),
            [lhs, op, rhs] if *op == "-gt" => parse_i(lhs) > parse_i(rhs),
            [lhs, op, rhs] if *op == "-ge" => parse_i(lhs) >= parse_i(rhs),
            _ => false,
        };

        if result {
            Ok(Output::success(""))
        } else {
            Ok(Output::error(1, "", ""))
        }
    }
}

fn parse_i(s: &str) -> i64 {
    s.parse().unwrap_or(0)
}
