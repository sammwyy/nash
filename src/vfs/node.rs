//! Filesystem node types.

use std::collections::HashMap;

/// A node in the virtual filesystem.
#[derive(Debug, Clone)]
pub enum FsNode {
    /// A regular file holding raw bytes.
    File(Vec<u8>),
    /// A directory.  Children are tracked lazily via the flat path map,
    /// but we keep the variant for type-checking purposes.
    Directory(HashMap<String, ()>),
}

/// A single directory entry (name + type).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}
