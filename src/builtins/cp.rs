use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    let plain: Vec<_> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if plain.len() < 2 {
        bail!("cp: missing destination operand");
    }
    let dst_arg = plain.last().unwrap();
    let srcs = &plain[..plain.len() - 1];
    let dst = VfsPath::join(ctx.get_cwd(), dst_arg);

    for src_arg in srcs {
        let src = VfsPath::join(ctx.get_cwd(), src_arg);
        let final_dst = if ctx.state.vfs.is_dir(&dst) {
            let name = VfsPath::basename(&src);
            format!("{}/{}", dst, name)
        } else {
            dst.clone()
        };
        ctx.state.vfs.copy_file(&src, &final_dst)?;
    }

    Ok(Output::success("".into()))
}
