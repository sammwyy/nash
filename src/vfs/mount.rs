//! Mount point types.

/// Options for a host-directory mount.
#[derive(Debug, Clone)]
pub struct MountOptions {
    /// If true, writes through this mount are rejected.
    pub read_only: bool,
}

/// A host directory binding.
#[derive(Debug, Clone)]
pub struct MountPoint {
    /// Absolute path on the host filesystem.
    pub host_path: String,
    /// Absolute VFS path where the host directory appears.
    pub vfs_path: String,
    pub opts: MountOptions,
}
