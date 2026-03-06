use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::Result;

pub struct FileCmd;

impl Builtin for FileCmd {
    fn run(&self, args: &[String], ctx: &mut Context, _stdin: &str) -> Result<Output> {
        if args.is_empty() {
            return Ok(Output::error(1, "", "file: missing operand\n"));
        }

        let mut out = String::new();
        for arg in args {
            if arg.starts_with('-') {
                continue;
            }
            let abs = VfsPath::join(&ctx.cwd, arg);

            if !ctx.vfs.exists(&abs) {
                out.push_str(&format!(
                    "{}: cannot open (No such file or directory)\n",
                    arg
                ));
                continue;
            }

            if ctx.vfs.is_dir(&abs) {
                out.push_str(&format!("{}: directory\n", arg));
                continue;
            }

            let kind = match ctx.vfs.read(&abs) {
                Ok(bytes) => detect_type(&bytes, arg),
                Err(_) => "data".to_string(),
            };

            out.push_str(&format!("{}: {}\n", arg, kind));
        }

        Ok(Output::success(out))
    }
}

fn detect_type(bytes: &[u8], name: &str) -> String {
    // Magic byte detection
    if bytes.starts_with(b"\x7fELF") {
        return "ELF executable".to_string();
    }
    if bytes.starts_with(b"\x89PNG") {
        return "PNG image data".to_string();
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return "JPEG image data".to_string();
    }
    if bytes.starts_with(b"GIF8") {
        return "GIF image data".to_string();
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return "Zip archive data".to_string();
    }
    if bytes.starts_with(b"%PDF") {
        return "PDF document".to_string();
    }
    if bytes.starts_with(b"#!") {
        // Shebang — peek at the interpreter
        let line = std::str::from_utf8(&bytes[..bytes.len().min(64)]).unwrap_or("");
        return format!("script ({})", line.lines().next().unwrap_or("unknown"));
    }

    // Extension-based fallback
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "Rust source code".to_string(),
        "toml" => "TOML configuration".to_string(),
        "json" => "JSON data".to_string(),
        "yaml" | "yml" => "YAML data".to_string(),
        "md" => "Markdown document".to_string(),
        "txt" => "ASCII text".to_string(),
        "sh" => "shell script".to_string(),
        "py" => "Python script".to_string(),
        "js" => "JavaScript source".to_string(),
        "html" => "HTML document".to_string(),
        "css" => "CSS stylesheet".to_string(),
        _ => {
            // Heuristic: is it valid UTF-8?
            if std::str::from_utf8(bytes).is_ok() {
                "ASCII text".to_string()
            } else {
                "binary data".to_string()
            }
        }
    }
}
