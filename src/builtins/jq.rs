use super::Builtin;
use crate::runtime::{Context, Output};
use crate::vfs::path::VfsPath;
use anyhow::{bail, Result};

pub struct Jq;

impl Builtin for Jq {
    fn run(&self, args: &[String], ctx: &mut Context, stdin: &str) -> Result<Output> {
        let mut compact = false;
        let mut raw_output = false;
        let mut filter = ".".to_string();
        let mut files: Vec<String> = Vec::new();
        let mut null_input = false;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-c" => compact = true,
                "-r" => raw_output = true,
                "-n" => null_input = true,
                s if s.starts_with('-') => {}
                _ => {
                    if filter == "." && files.is_empty() {
                        filter = arg.clone();
                    } else {
                        files.push(arg.clone());
                    }
                }
            }
        }

        let text = if null_input {
            "null".to_string()
        } else if files.is_empty() {
            stdin.to_string()
        } else {
            let mut buf = String::new();
            for f in &files {
                let abs = VfsPath::join(&ctx.cwd, f);
                buf.push_str(&ctx.vfs.read_to_string(&abs)?);
            }
            buf
        };

        let value =
            parse_json(text.trim()).map_err(|e| anyhow::anyhow!("jq: invalid JSON: {}", e))?;

        let result = apply_filter(&value, &filter).map_err(|e| anyhow::anyhow!("jq: {}", e))?;

        let out = if compact {
            format!("{}\n", json_to_string_compact(&result, raw_output))
        } else {
            format!("{}\n", json_to_string_pretty(&result, 0, raw_output))
        };

        Ok(Output::success(out))
    }
}

// ─── Minimal JSON value type ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

// ─── Parser ───────────────────────────────────────────────────────────────────

fn parse_json(s: &str) -> Result<JsonValue> {
    let (val, rest) = parse_value(s.trim())?;
    if !rest.trim().is_empty() {
        bail!("trailing input: {}", rest.trim());
    }
    Ok(val)
}

fn parse_value(s: &str) -> Result<(JsonValue, &str)> {
    let s = s.trim_start();
    match s.chars().next() {
        Some('n') => {
            let rest = s
                .strip_prefix("null")
                .ok_or_else(|| anyhow::anyhow!("expected null"))?;
            Ok((JsonValue::Null, rest))
        }
        Some('t') => {
            let rest = s
                .strip_prefix("true")
                .ok_or_else(|| anyhow::anyhow!("expected true"))?;
            Ok((JsonValue::Bool(true), rest))
        }
        Some('f') => {
            let rest = s
                .strip_prefix("false")
                .ok_or_else(|| anyhow::anyhow!("expected false"))?;
            Ok((JsonValue::Bool(false), rest))
        }
        Some('"') => parse_string(s),
        Some('[') => parse_array(s),
        Some('{') => parse_object(s),
        Some(c) if c == '-' || c.is_ascii_digit() => parse_number(s),
        _ => bail!("unexpected character: {:?}", s.chars().next()),
    }
}

fn parse_string(s: &str) -> Result<(JsonValue, &str)> {
    assert!(s.starts_with('"'));
    let mut result = String::new();
    let mut escape = false;
    let mut end = None;
    for (i, c) in s[1..].char_indices() {
        if escape {
            match c {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '"' => result.push('"'),
                '\\' => result.push('\\'),
                '/' => result.push('/'),
                _ => {
                    result.push('\\');
                    result.push(c);
                }
            }
            escape = false;
        } else if c == '\\' {
            escape = true;
        } else if c == '"' {
            end = Some(i + 1); // +1 for the leading "
            break;
        } else {
            result.push(c);
        }
    }
    match end {
        Some(pos) => Ok((JsonValue::String(result), &s[pos + 1..])),
        None => bail!("unterminated string"),
    }
}

fn parse_number(s: &str) -> Result<(JsonValue, &str)> {
    let end = s
        .find(|c: char| {
            !c.is_ascii_digit() && c != '.' && c != '-' && c != 'e' && c != 'E' && c != '+'
        })
        .unwrap_or(s.len());
    let num: f64 = s[..end]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid number"))?;
    Ok((JsonValue::Number(num), &s[end..]))
}

fn parse_array(s: &str) -> Result<(JsonValue, &str)> {
    let s = s[1..].trim_start(); // skip [
    if s.starts_with(']') {
        return Ok((JsonValue::Array(vec![]), &s[1..]));
    }
    let mut items = Vec::new();
    let mut rest = s;
    loop {
        let (val, r) = parse_value(rest)?;
        items.push(val);
        rest = r.trim_start();
        match rest.chars().next() {
            Some(',') => {
                rest = &rest[1..];
            }
            Some(']') => {
                rest = &rest[1..];
                break;
            }
            _ => bail!("expected , or ] in array"),
        }
    }
    Ok((JsonValue::Array(items), rest))
}

fn parse_object(s: &str) -> Result<(JsonValue, &str)> {
    let s = s[1..].trim_start(); // skip {
    if s.starts_with('}') {
        return Ok((JsonValue::Object(vec![]), &s[1..]));
    }
    let mut pairs = Vec::new();
    let mut rest = s;
    loop {
        let rest_trimmed = rest.trim_start();
        let (key_val, r) = parse_string(rest_trimmed)?;
        let key = match key_val {
            JsonValue::String(k) => k,
            _ => bail!("object key must be string"),
        };
        let r = r.trim_start();
        let r = r
            .strip_prefix(':')
            .ok_or_else(|| anyhow::anyhow!("expected :"))?;
        let (val, r) = parse_value(r)?;
        pairs.push((key, val));
        rest = r.trim_start();
        match rest.chars().next() {
            Some(',') => {
                rest = &rest[1..];
            }
            Some('}') => {
                rest = &rest[1..];
                break;
            }
            _ => bail!("expected , or }} in object"),
        }
    }
    Ok((JsonValue::Object(pairs), rest))
}

// ─── Filter application ───────────────────────────────────────────────────────

fn apply_filter(val: &JsonValue, filter: &str) -> Result<JsonValue> {
    let f = filter.trim();

    // Identity
    if f == "." {
        return Ok(val.clone());
    }

    // .key
    if f.starts_with('.') && !f.contains('[') {
        let key = &f[1..];
        if key.is_empty() {
            return Ok(val.clone());
        }

        // Chained: .a.b.c
        if let Some(dot) = key.find('.') {
            let first = &key[..dot];
            let rest = &key[dot..];
            let inner = apply_filter(val, &format!(".{}", first))?;
            return apply_filter(&inner, rest);
        }

        match val {
            JsonValue::Object(pairs) => {
                for (k, v) in pairs {
                    if k == key {
                        return Ok(v.clone());
                    }
                }
                Ok(JsonValue::Null)
            }
            _ => bail!(".{} on non-object", key),
        }
    }
    // .[N] or .["key"]
    else if f.starts_with(".[") {
        let inner = f[2..].trim_end_matches(']');
        if let Ok(idx) = inner.trim_matches('"').parse::<usize>() {
            match val {
                JsonValue::Array(arr) => Ok(arr.get(idx).cloned().unwrap_or(JsonValue::Null)),
                _ => bail!(".[n] on non-array"),
            }
        } else {
            let key = inner.trim_matches('"');
            match val {
                JsonValue::Object(pairs) => {
                    for (k, v) in pairs {
                        if k == key {
                            return Ok(v.clone());
                        }
                    }
                    Ok(JsonValue::Null)
                }
                _ => bail!(".[key] on non-object"),
            }
        }
    }
    // keys
    else if f == "keys" {
        match val {
            JsonValue::Object(pairs) => Ok(JsonValue::Array(
                pairs
                    .iter()
                    .map(|(k, _)| JsonValue::String(k.clone()))
                    .collect(),
            )),
            JsonValue::Array(arr) => Ok(JsonValue::Array(
                (0..arr.len())
                    .map(|i| JsonValue::Number(i as f64))
                    .collect(),
            )),
            _ => bail!("keys on non-object/array"),
        }
    }
    // values
    else if f == "values" {
        match val {
            JsonValue::Object(pairs) => Ok(JsonValue::Array(
                pairs.iter().map(|(_, v)| v.clone()).collect(),
            )),
            JsonValue::Array(arr) => Ok(JsonValue::Array(arr.clone())),
            _ => bail!("values on non-object/array"),
        }
    }
    // length
    else if f == "length" {
        let n = match val {
            JsonValue::Array(a) => a.len() as f64,
            JsonValue::Object(o) => o.len() as f64,
            JsonValue::String(s) => s.len() as f64,
            JsonValue::Null => 0.0,
            _ => bail!("length on unsupported type"),
        };
        Ok(JsonValue::Number(n))
    }
    // type
    else if f == "type" {
        let t = match val {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        };
        Ok(JsonValue::String(t.to_string()))
    }
    // .[]  — iterate
    else if f == ".[]" {
        match val {
            JsonValue::Array(arr) => {
                // Return all as array for now (full iteration handled in output)
                Ok(JsonValue::Array(arr.clone()))
            }
            JsonValue::Object(pairs) => Ok(JsonValue::Array(
                pairs.iter().map(|(_, v)| v.clone()).collect(),
            )),
            _ => bail!(".[] on non-iterable"),
        }
    }
    // has("key")
    else if f.starts_with("has(") && f.ends_with(')') {
        let key = f[4..f.len() - 1].trim().trim_matches('"');
        let found = match val {
            JsonValue::Object(pairs) => pairs.iter().any(|(k, _)| k == key),
            JsonValue::Array(arr) => key.parse::<usize>().map(|i| i < arr.len()).unwrap_or(false),
            _ => false,
        };
        Ok(JsonValue::Bool(found))
    } else {
        bail!("unsupported filter: {}", f)
    }
}

// ─── Serialization ────────────────────────────────────────────────────────────

fn json_to_string_compact(val: &JsonValue, raw: bool) -> String {
    match val {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => format_number(*n),
        JsonValue::String(s) => {
            if raw {
                s.clone()
            } else {
                format!("\"{}\"", escape_str(s))
            }
        }
        JsonValue::Array(arr) => format!(
            "[{}]",
            arr.iter()
                .map(|v| json_to_string_compact(v, raw))
                .collect::<Vec<_>>()
                .join(",")
        ),
        JsonValue::Object(kvs) => format!(
            "{{{}}}",
            kvs.iter()
                .map(|(k, v)| format!("\"{}\":{}", escape_str(k), json_to_string_compact(v, raw)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn json_to_string_pretty(val: &JsonValue, indent: usize, raw: bool) -> String {
    let pad = "  ".repeat(indent);
    let inner_pad = "  ".repeat(indent + 1);
    match val {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => format_number(*n),
        JsonValue::String(s) => {
            if raw {
                s.clone()
            } else {
                format!("\"{}\"", escape_str(s))
            }
        }
        JsonValue::Array(arr) if arr.is_empty() => "[]".to_string(),
        JsonValue::Array(arr) => {
            let items: Vec<_> = arr
                .iter()
                .map(|v| format!("{}{}", inner_pad, json_to_string_pretty(v, indent + 1, raw)))
                .collect();
            format!("[\n{}\n{}]", items.join(",\n"), pad)
        }
        JsonValue::Object(kvs) if kvs.is_empty() => "{}".to_string(),
        JsonValue::Object(kvs) => {
            let items: Vec<_> = kvs
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}\"{}\": {}",
                        inner_pad,
                        escape_str(k),
                        json_to_string_pretty(v, indent + 1, raw)
                    )
                })
                .collect();
            format!("{{\n{}\n{}}}", items.join(",\n"), pad)
        }
    }
}

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

fn escape_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}
