//! # Built-in Commands
//!
//! All commands operate exclusively through the VFS API.
//! No system binaries are ever invoked.

mod cat;
mod cd;
mod clear;
mod cp;
mod cut;
mod echo;
mod env;
mod file;
mod find;
mod grep;
mod head_tail;
mod help;
mod history;
mod jq;
mod ls;
mod mkdir;
mod mv;
mod pwd;
mod rm;
mod sed;
mod sort;
mod stat;
mod touch;
mod tree;
mod uniq;
mod util;
mod wc;
mod which;

use crate::runtime::{Context, Output};
use anyhow::Result;

/// Trait that every built-in command must implement.
pub trait Builtin {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output>;
}

/// Dispatch a command name to its builtin implementation, if one exists.
pub fn dispatch(name: &str) -> Option<Box<dyn Builtin>> {
    match name {
        "cat" => Some(Box::new(cat::Cat)),
        "cd" => Some(Box::new(cd::Cd)),
        "clear" => Some(Box::new(clear::Clear)),
        "cp" => Some(Box::new(cp::Cp)),
        "cut" => Some(Box::new(cut::Cut)),
        "echo" => Some(Box::new(echo::Echo)),
        "env" => Some(Box::new(env::EnvCmd)),
        "export" => Some(Box::new(env::Export)),
        "unset" => Some(Box::new(env::Unset)),
        "file" => Some(Box::new(file::FileCmd)),
        "find" => Some(Box::new(find::Find)),
        "grep" => Some(Box::new(grep::Grep)),
        "head" => Some(Box::new(head_tail::Head)),
        "tail" => Some(Box::new(head_tail::Tail)),
        "help" => Some(Box::new(help::Help)),
        "history" => Some(Box::new(history::History)),
        "jq" => Some(Box::new(jq::Jq)),
        "ls" => Some(Box::new(ls::Ls)),
        "mkdir" => Some(Box::new(mkdir::Mkdir)),
        "mv" => Some(Box::new(mv::Mv)),
        "pwd" => Some(Box::new(pwd::Pwd)),
        "rm" => Some(Box::new(rm::Rm)),
        "sed" => Some(Box::new(sed::Sed)),
        "sort" => Some(Box::new(sort::Sort)),
        "stat" => Some(Box::new(stat::Stat)),
        "touch" => Some(Box::new(touch::Touch)),
        "tree" => Some(Box::new(tree::Tree)),
        "true" => Some(Box::new(util::True)),
        "false" => Some(Box::new(util::False)),
        "test" | "[" => Some(Box::new(util::Test)),
        "uniq" => Some(Box::new(uniq::Uniq)),
        "wc" => Some(Box::new(wc::Wc)),
        "which" => Some(Box::new(which::Which)),
        _ => None,
    }
}
