//! Execution context (environment variables, cwd, VFS reference).

use crate::vfs::Vfs;
use indexmap::IndexMap;

/// The mutable state shared across an entire shell session.
pub struct Context {
    /// Current working directory (VFS path).
    pub cwd: String,
    /// Environment variables.
    pub env: IndexMap<String, String>,
    /// The virtual filesystem.
    pub vfs: Vfs,
    /// Host binaries that are allowed to be executed.
    pub allowed_bins: IndexMap<String, String>,
    /// Command history (most recent last).
    pub history: Vec<String>,
}

impl Context {
    pub fn new(
        cwd: String,
        env: IndexMap<String, String>,
        vfs: Vfs,
        allowed_bins: IndexMap<String, String>,
    ) -> Self {
        Self {
            cwd,
            env,
            vfs,
            allowed_bins,
            history: Vec::new(),
        }
    }
}
