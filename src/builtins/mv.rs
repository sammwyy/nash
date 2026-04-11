use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    let plain: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if plain.len() < 2 {
        bail!("mv: missing destination operand");
    }
    let dst_arg = plain.last().unwrap();
    let src = VfsPath::join(ctx.get_cwd(), plain[0]);
    let dst_base = VfsPath::join(ctx.get_cwd(), dst_arg);
    let dst = if ctx.state.vfs.is_dir(&dst_base) {
        let name = VfsPath::basename(&src);
        format!("{}/{}", dst_base, name)
    } else {
        dst_base
    };
    ctx.state.vfs.rename(&src, &dst)?;
    Ok(Output::success("".into()))
}
