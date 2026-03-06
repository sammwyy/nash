//! # Sandbox Runtime
//!
//! Executes a Nash AST in a fully sandboxed environment.
//! No system binaries are ever called; everything runs through VFS and builtins.

mod context;
mod executor;
mod output;

pub use context::Context;
pub use executor::{Executor, ExecutorConfig};
pub use output::Output;
