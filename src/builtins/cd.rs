use crate::runtime::context::Context;
use shellframe::Output;
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub fn run(args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
    // No args -> go to $HOME, or /home/<user> as fallback
    let raw = match args.first() {
        Some(p) if p == "-" => {
            // cd - => go back to $OLDPWD
            ctx.env
                .get("OLDPWD")
                .cloned()
                .unwrap_or_else(|| "/".to_string())
        }
        Some(p) => p.clone(),
        None => ctx
            .env
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/home/user".to_string()),
    };

    let target = VfsPath::join(ctx.get_cwd(), &raw);

    if !ctx.state.vfs.is_dir(&target) {
        bail!("cd: not a directory: {}", raw);
    }

    // Update OLDPWD before moving
    let old = ctx.get_cwd().to_string();
    ctx.env.insert("OLDPWD".into(), old);

    // Update CWD (also updates PWD env)
    ctx.set_cwd(target);

    Ok(Output::success("".into()))
}
