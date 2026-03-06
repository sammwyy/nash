use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub struct Cd;

impl Builtin for Cd {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
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

        let target = VfsPath::join(&ctx.cwd, &raw);

        if !ctx.vfs.is_dir(&target) {
            bail!("cd: not a directory: {}", raw);
        }

        // Update OLDPWD before moving
        let old = ctx.cwd.clone();
        ctx.env.insert("OLDPWD".into(), old);

        ctx.cwd = target.clone();
        ctx.env.insert("PWD".into(), target);

        Ok(Output::success(""))
    }
}
