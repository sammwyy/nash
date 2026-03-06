use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub struct Mv;

impl Builtin for Mv {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        let plain: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
        if plain.len() < 2 {
            bail!("mv: missing destination operand");
        }
        let dst_arg = plain.last().unwrap();
        let src = VfsPath::join(&ctx.cwd, plain[0]);
        let dst_base = VfsPath::join(&ctx.cwd, dst_arg);
        let dst = if ctx.vfs.is_dir(&dst_base) {
            let name = VfsPath::basename(&src);
            format!("{}/{}", dst_base, name)
        } else {
            dst_base
        };
        ctx.vfs.rename(&src, &dst)?;
        Ok(Output::success(""))
    }
}
