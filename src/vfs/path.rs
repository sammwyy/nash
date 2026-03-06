//! Path normalization and manipulation utilities.

pub struct VfsPath;

impl VfsPath {
    /// Normalize a VFS path:
    /// - Resolve `.` and `..` components
    /// - Ensure it starts with `/`
    /// - Remove trailing slashes (except root)
    pub fn normalize(path: &str) -> String {
        let mut components: Vec<&str> = Vec::new();
        let absolute = path.starts_with('/');

        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    components.pop();
                }
                other => components.push(other),
            }
        }

        let result = if absolute {
            format!("/{}", components.join("/"))
        } else {
            components.join("/")
        };

        // Ensure absolute paths always start with /
        if result.is_empty() {
            "/".to_string()
        } else {
            result
        }
    }

    /// Return the parent path of `path`.
    pub fn parent(path: &str) -> String {
        let p = Self::normalize(path);
        if p == "/" {
            return "/".to_string();
        }
        match p.rfind('/') {
            Some(0) => "/".to_string(),
            Some(i) => p[..i].to_string(),
            None => "/".to_string(),
        }
    }

    /// Return the final component (file/dir name) of a path.
    pub fn basename(path: &str) -> String {
        let p = Self::normalize(path);
        match p.rfind('/') {
            Some(i) => p[i + 1..].to_string(),
            None => p,
        }
    }

    /// Join a base path with a relative component.
    pub fn join(base: &str, rel: &str) -> String {
        if rel.starts_with('/') {
            Self::normalize(rel)
        } else {
            Self::normalize(&format!("{}/{}", base, rel))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_simple() {
        assert_eq!(VfsPath::normalize("/home/user"), "/home/user");
    }

    #[test]
    fn test_normalize_dot() {
        assert_eq!(VfsPath::normalize("/home/./user"), "/home/user");
    }

    #[test]
    fn test_normalize_dotdot() {
        assert_eq!(VfsPath::normalize("/home/user/../other"), "/home/other");
    }

    #[test]
    fn test_normalize_root() {
        assert_eq!(VfsPath::normalize("/"), "/");
    }

    #[test]
    fn test_parent() {
        assert_eq!(VfsPath::parent("/home/user"), "/home");
        assert_eq!(VfsPath::parent("/home"), "/");
        assert_eq!(VfsPath::parent("/"), "/");
    }

    #[test]
    fn test_basename() {
        assert_eq!(VfsPath::basename("/home/user/file.txt"), "file.txt");
        assert_eq!(VfsPath::basename("/"), "");
    }

    #[test]
    fn test_join() {
        assert_eq!(VfsPath::join("/home/user", "docs"), "/home/user/docs");
        assert_eq!(VfsPath::join("/home/user", "/etc"), "/etc");
        assert_eq!(VfsPath::join("/home/user", "../other"), "/home/other");
    }
}
