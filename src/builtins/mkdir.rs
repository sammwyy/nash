use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    let mut parents = false;
    let mut dirs: Vec<String> = Vec::new();

    for arg in args {
        if arg == "-p" {
            parents = true;
        } else {
            dirs.push(arg.clone());
        }
    }

    if dirs.is_empty() {
        return Ok(Output::error(1, "".into(), "mkdir: missing operand\n".into()));
    }

    for dir in &dirs {
        let abs = VfsPath::join(ctx.get_cwd(), dir);
        if parents {
            ctx.state.vfs.mkdir_p(&abs)?;
        } else {
            ctx.state.vfs.mkdir(&abs)?;
        }
    }

    Ok(Output::success("".into()))
}
