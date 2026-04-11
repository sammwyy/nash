use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub fn run(args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
    if args.is_empty() {
        return Ok(Output::success(stdin.to_string()));
    }

    let mut out = String::new();
    for arg in args {
        if arg.starts_with('-') {
            continue;
        }
        let abs = VfsPath::join(ctx.get_cwd(), arg);
        match ctx.state.vfs.read_to_string(&abs) {
            Ok(content) => out.push_str(&content),
            Err(e) => {
                return Ok(Output::error(1, "".into(), format!("cat: {}: {}\n", arg, e)));
            }
        }
    }

    Ok(Output::success(out))
}
