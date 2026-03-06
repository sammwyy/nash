//! Command output type.

/// The result of executing any shell expression.
#[derive(Debug, Clone, Default)]
pub struct Output {
    /// Text written to stdout.
    pub stdout: String,
    /// Text written to stderr.
    pub stderr: String,
    /// Exit code (0 = success).
    pub exit_code: i32,
}

impl Output {
    /// Successful output with content.
    pub fn success(stdout: impl Into<String>) -> Self {
        Output {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: 0,
        }
    }

    /// Failed output.
    pub fn error(exit_code: i32, stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Output {
            stdout: stdout.into(),
            stderr: stderr.into(),
            exit_code,
        }
    }

    /// Did this command succeed?
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}
