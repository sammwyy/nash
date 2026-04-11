use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    for arg in args {
        if arg.starts_with('-') {
            continue;
        }
        let abs = VfsPath::join(ctx.get_cwd(), arg);
        ctx.state.vfs.touch(&abs)?;
    }
    Ok(Output::success("".into()))
}
