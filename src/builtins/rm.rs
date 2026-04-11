use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    let mut recursive = false;
    let mut targets: Vec<String> = Vec::new();

    for arg in args {
        if arg == "-r" || arg == "-rf" || arg == "-R" {
            recursive = true;
        } else if arg.starts_with('-') { /* ignore unknown flags */
        } else {
            targets.push(arg.clone());
        }
    }

    if targets.is_empty() {
        return Ok(Output::error(1, "".into(), "rm: missing operand\n".into()));
    }

    for target in &targets {
        let abs = VfsPath::join(ctx.get_cwd(), target);
        if recursive {
            ctx.state.vfs.remove_recursive(&abs)?;
        } else {
            ctx.state.vfs.remove(&abs)?;
        }
    }

    Ok(Output::success("".into()))
}
