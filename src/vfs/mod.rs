//! # Virtual Filesystem
//!
//! An entirely in-memory filesystem with optional host-directory mounts.
//!
//! All Nash commands access the filesystem through this module.
//! Real host paths are NEVER touched except through explicit mount bindings.

pub mod mount;
pub mod node;
pub mod path;

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use mount::{MountOptions, MountPoint};
use node::{DirEntry, FsNode};
use path::VfsPath;

/// The virtual filesystem root.
///
/// Internally, the filesystem is a tree of [`FsNode`]s stored by their
/// absolute VFS path string.  An optional host mount overlay can redirect
/// reads/writes for a subtree to a real directory.
pub struct Vfs {
    nodes: HashMap<String, FsNode>,
    mounts: Vec<MountPoint>,
}

impl Vfs {
    /// Create a new VFS with a standard directory skeleton.
    pub fn new() -> Self {
        let mut vfs = Vfs {
            nodes: HashMap::new(),
            mounts: Vec::new(),
        };
        // Bootstrap standard directories
        for dir in &["/", "/bin", "/usr", "/home", "/tmp", "/lib", "/etc", "/var"] {
            vfs.nodes
                .insert(dir.to_string(), FsNode::Directory(HashMap::new()));
        }
        vfs
    }

    // ─── Mount management ────────────────────────────────────────────────────

    /// Register a host-directory mount.
    pub fn mount(&mut self, host_path: String, vfs_path: String, opts: MountOptions) -> Result<()> {
        // Ensure the vfs mount point directory exists
        self.ensure_dir(&vfs_path)?;
        self.mounts.push(MountPoint {
            host_path,
            vfs_path,
            opts,
        });
        Ok(())
    }

    // ─── Core operations ─────────────────────────────────────────────────────

    /// Check if a path exists.
    pub fn exists(&self, path: &str) -> bool {
        let p = VfsPath::normalize(path);
        if self.nodes.contains_key(&p) {
            return true;
        }
        if let Some(mp) = self.find_mount(&p) {
            let rel = p
                .strip_prefix(&mp.vfs_path)
                .unwrap_or("")
                .trim_start_matches('/');
            let host = format!("{}/{}", mp.host_path, rel);
            return std::path::Path::new(&host).exists();
        }
        false
    }

    /// Check whether path is a directory.
    pub fn is_dir(&self, path: &str) -> bool {
        let p = VfsPath::normalize(path);
        if let Some(node) = self.nodes.get(&p) {
            return matches!(node, FsNode::Directory(_));
        }
        if let Some(mp) = self.find_mount(&p) {
            let rel = p
                .strip_prefix(&mp.vfs_path)
                .unwrap_or("")
                .trim_start_matches('/');
            let host = format!("{}/{}", mp.host_path, rel);
            return std::path::Path::new(&host).is_dir();
        }
        false
    }

    /// Read file contents as bytes.
    pub fn read(&self, path: &str) -> Result<Vec<u8>> {
        let p = VfsPath::normalize(path);

        if let Some(node) = self.nodes.get(&p) {
            match node {
                FsNode::File(data) => return Ok(data.clone()),
                FsNode::Directory(_) => bail!("is a directory: {}", path),
            }
        }

        // Try host mount
        if let Some(mp) = self.find_mount(&p) {
            let rel = p
                .strip_prefix(&mp.vfs_path)
                .unwrap_or("")
                .trim_start_matches('/');
            let host = format!("{}/{}", mp.host_path, rel);
            return std::fs::read(&host).with_context(|| format!("cannot read {}", path));
        }

        bail!("no such file: {}", path)
    }

    /// Read file contents as a UTF-8 string.
    pub fn read_to_string(&self, path: &str) -> Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|_| anyhow::anyhow!("not valid UTF-8: {}", path))
    }

    /// Write (overwrite) a file.
    pub fn write(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        let p = VfsPath::normalize(path);
        self.check_write_allowed(&p)?;

        // Ensure parent directory exists
        let parent = VfsPath::parent(&p);
        if !parent.is_empty() && !self.is_dir(&parent) {
            bail!("no such directory: {}", parent);
        }

        // Try host mount first
        if let Some(mp) = self.find_mount_mut(&p) {
            let host_path = mp.host_path.clone();
            let vfs_path = mp.vfs_path.clone();
            let rel = p
                .strip_prefix(&vfs_path)
                .unwrap_or("")
                .trim_start_matches('/');
            let host = format!("{}/{}", host_path, rel);
            return std::fs::write(&host, &data).with_context(|| format!("cannot write {}", path));
        }

        self.nodes.insert(p, FsNode::File(data));
        Ok(())
    }

    /// Write a UTF-8 string to a file.
    pub fn write_str(&mut self, path: &str, s: &str) -> Result<()> {
        self.write(path, s.as_bytes().to_vec())
    }

    /// Append to a file.
    pub fn append(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        let p = VfsPath::normalize(path);
        self.check_write_allowed(&p)?;

        if let Some(mp) = self.find_mount(&p) {
            let rel = p
                .strip_prefix(&mp.vfs_path)
                .unwrap_or("")
                .trim_start_matches('/');
            let host = format!("{}/{}", mp.host_path, rel);
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&host)
                .with_context(|| format!("cannot open {} for append", path))?;
            f.write_all(&data)?;
            return Ok(());
        }

        let entry = self
            .nodes
            .entry(p)
            .or_insert_with(|| FsNode::File(Vec::new()));
        match entry {
            FsNode::File(existing) => existing.extend_from_slice(&data),
            _ => bail!("is a directory: {}", path),
        }
        Ok(())
    }

    /// Create a directory (and all missing parents).
    pub fn mkdir_p(&mut self, path: &str) -> Result<()> {
        let p = VfsPath::normalize(path);
        let mut current = String::new();
        for component in p.split('/') {
            if component.is_empty() {
                current.push('/');
                continue;
            }
            if current == "/" {
                current.push_str(component);
            } else {
                current.push('/');
                current.push_str(component);
            }
            if !self.nodes.contains_key(&current) {
                self.nodes
                    .insert(current.clone(), FsNode::Directory(HashMap::new()));
            }
        }
        Ok(())
    }

    /// Create a single directory (parent must exist).
    pub fn mkdir(&mut self, path: &str) -> Result<()> {
        let p = VfsPath::normalize(path);
        let parent = VfsPath::parent(&p);
        if !self.is_dir(&parent) {
            bail!("no such file or directory: {}", parent);
        }
        if self.exists(&p) {
            bail!("file exists: {}", path);
        }
        self.nodes.insert(p, FsNode::Directory(HashMap::new()));
        Ok(())
    }

    /// Create an empty file if it does not exist.
    pub fn touch(&mut self, path: &str) -> Result<()> {
        let p = VfsPath::normalize(path);
        if !self.exists(&p) {
            let parent = VfsPath::parent(&p);
            if !self.is_dir(&parent) {
                bail!("no such directory: {}", parent);
            }
            self.nodes.insert(p, FsNode::File(Vec::new()));
        }
        Ok(())
    }

    /// Remove a file or empty directory.
    pub fn remove(&mut self, path: &str) -> Result<()> {
        let p = VfsPath::normalize(path);
        self.check_write_allowed(&p)?;

        if let Some(node) = self.nodes.get(&p) {
            match node {
                FsNode::Directory(children) if !children.is_empty() => {
                    bail!("directory not empty: {}", path);
                }
                _ => {}
            }
            self.nodes.remove(&p);
            return Ok(());
        }
        bail!("no such file or directory: {}", path)
    }

    /// Remove a file or directory tree recursively.
    pub fn remove_recursive(&mut self, path: &str) -> Result<()> {
        let p = VfsPath::normalize(path);
        self.check_write_allowed(&p)?;

        if !self.exists(&p) {
            bail!("no such file or directory: {}", path);
        }

        // Collect all keys that are under `p`
        let to_remove: Vec<String> = self
            .nodes
            .keys()
            .filter(|k| *k == &p || k.starts_with(&format!("{}/", p)))
            .cloned()
            .collect();

        for key in to_remove {
            self.nodes.remove(&key);
        }
        Ok(())
    }

    /// List directory entries.
    pub fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        let p = VfsPath::normalize(path);

        if !self.is_dir(&p) {
            bail!("not a directory: {}", path);
        }

        let mut entries: Vec<DirEntry> = Vec::new();
        let prefix = if p == "/" {
            "/".to_string()
        } else {
            format!("{}/", p)
        };

        // In-memory children
        for key in self.nodes.keys() {
            if key == &p {
                continue;
            }
            if key.starts_with(&prefix) {
                let rest = &key[prefix.len()..];
                if !rest.contains('/') {
                    let is_dir = matches!(self.nodes.get(key), Some(FsNode::Directory(_)));
                    entries.push(DirEntry {
                        name: rest.to_string(),
                        is_dir,
                    });
                }
            }
        }

        // Host mount children
        if let Some(mp) = self.find_mount(&p) {
            let rel = p
                .strip_prefix(&mp.vfs_path)
                .unwrap_or("")
                .trim_start_matches('/');
            let host = if rel.is_empty() {
                mp.host_path.clone()
            } else {
                format!("{}/{}", mp.host_path, rel)
            };
            if let Ok(rd) = std::fs::read_dir(&host) {
                for entry in rd.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                    if !entries.iter().any(|e| e.name == name) {
                        entries.push(DirEntry { name, is_dir });
                    }
                }
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Copy a file.
    pub fn copy_file(&mut self, src: &str, dst: &str) -> Result<()> {
        let data = self.read(src)?;
        self.write(dst, data)
    }

    /// Move / rename a file or directory.
    pub fn rename(&mut self, src: &str, dst: &str) -> Result<()> {
        let s = VfsPath::normalize(src);
        let d = VfsPath::normalize(dst);
        self.check_write_allowed(&s)?;
        self.check_write_allowed(&d)?;

        if let Some(node) = self.nodes.remove(&s) {
            self.nodes.insert(d, node);
        } else {
            bail!("no such file or directory: {}", src);
        }
        Ok(())
    }

    // ─── Private helpers ─────────────────────────────────────────────────────

    fn ensure_dir(&mut self, path: &str) -> Result<()> {
        let p = VfsPath::normalize(path);
        if !self.nodes.contains_key(&p) {
            self.mkdir_p(&p)?;
        }
        Ok(())
    }

    /// Find the most specific mount point for a VFS path (read-only borrow).
    fn find_mount(&self, vfs_path: &str) -> Option<&MountPoint> {
        self.mounts
            .iter()
            .filter(|m| vfs_path == m.vfs_path || vfs_path.starts_with(&format!("{}/", m.vfs_path)))
            .max_by_key(|m| m.vfs_path.len())
    }

    /// Find the most specific mount point for a VFS path (mutable borrow).
    fn find_mount_mut(&mut self, vfs_path: &str) -> Option<&mut MountPoint> {
        let best = self
            .mounts
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                vfs_path == m.vfs_path || vfs_path.starts_with(&format!("{}/", m.vfs_path))
            })
            .max_by_key(|(_, m)| m.vfs_path.len())
            .map(|(i, _)| i);
        best.map(|i| &mut self.mounts[i])
    }

    fn check_write_allowed(&self, vfs_path: &str) -> Result<()> {
        if let Some(mp) = self.find_mount(vfs_path) {
            if mp.opts.read_only {
                bail!("filesystem is read-only: {}", vfs_path);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mkdir_and_exists() {
        let mut vfs = Vfs::new();
        vfs.mkdir("/tmp/mydir").unwrap();
        assert!(vfs.is_dir("/tmp/mydir"));
    }

    #[test]
    fn test_write_and_read() {
        let mut vfs = Vfs::new();
        vfs.write_str("/tmp/hello.txt", "hello world\n").unwrap();
        let content = vfs.read_to_string("/tmp/hello.txt").unwrap();
        assert_eq!(content, "hello world\n");
    }

    #[test]
    fn test_append() {
        let mut vfs = Vfs::new();
        vfs.write_str("/tmp/f.txt", "line1\n").unwrap();
        vfs.append("/tmp/f.txt", b"line2\n".to_vec()).unwrap();
        let content = vfs.read_to_string("/tmp/f.txt").unwrap();
        assert_eq!(content, "line1\nline2\n");
    }

    #[test]
    fn test_list_dir() {
        let mut vfs = Vfs::new();
        vfs.write_str("/tmp/a.txt", "").unwrap();
        vfs.write_str("/tmp/b.txt", "").unwrap();
        vfs.mkdir("/tmp/subdir").unwrap();
        let entries = vfs.list_dir("/tmp").unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"a.txt"));
        assert!(names.contains(&"b.txt"));
        assert!(names.contains(&"subdir"));
    }

    #[test]
    fn test_remove_file() {
        let mut vfs = Vfs::new();
        vfs.write_str("/tmp/del.txt", "x").unwrap();
        vfs.remove("/tmp/del.txt").unwrap();
        assert!(!vfs.exists("/tmp/del.txt"));
    }

    #[test]
    fn test_mkdir_p() {
        let mut vfs = Vfs::new();
        vfs.mkdir_p("/tmp/a/b/c").unwrap();
        assert!(vfs.is_dir("/tmp/a/b/c"));
    }

    #[test]
    fn test_copy_file() {
        let mut vfs = Vfs::new();
        vfs.write_str("/tmp/src.txt", "data").unwrap();
        vfs.copy_file("/tmp/src.txt", "/tmp/dst.txt").unwrap();
        assert_eq!(vfs.read_to_string("/tmp/dst.txt").unwrap(), "data");
    }
}
