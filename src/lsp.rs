//! Language Server Protocol over stdio: `voltc lsp`.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::ide;
use crate::target;

pub fn run() -> Result<(), String> {
    set_stdio_binary();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut docs: HashMap<String, String> = HashMap::new();
    let mut reader = stdin.lock();

    loop {
        let Some(msg) = read_message(&mut reader)? else {
            break;
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        match method {
            "initialize" => {
                respond(
                    &mut stdout,
                    id,
                    json!({
                        "capabilities": {
                            "textDocumentSync": 1,
                            "completionProvider": { "triggerCharacters": [".", ":"] },
                            "hoverProvider": true
                        },
                        "serverInfo": { "name": "voltc", "version": env!("CARGO_PKG_VERSION") }
                    }),
                )?;
            }
            "initialized" | "shutdown" => {
                if id.is_some() {
                    respond(&mut stdout, id, json!(null))?;
                }
            }
            "exit" => break,
            "textDocument/didOpen" => {
                if let Some((uri, text)) = doc_from_params(&msg) {
                    docs.insert(uri.clone(), text);
                    publish_diags(&mut stdout, &uri, docs.get(&uri).unwrap())?;
                }
            }
            "textDocument/didChange" => {
                if let Some((uri, text)) = change_from_params(&msg) {
                    docs.insert(uri.clone(), text);
                    publish_diags(&mut stdout, &uri, docs.get(&uri).unwrap())?;
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = uri_from_params(&msg) {
                    docs.remove(&uri);
                }
            }
            "textDocument/completion" => {
                let items = completion_items(&docs, &msg);
                respond(&mut stdout, id, json!(items))?;
            }
            "textDocument/hover" => {
                let hover = hover_item(&docs, &msg);
                respond(&mut stdout, id, hover)?;
            }
            _ => {
                if id.is_some() {
                    respond(&mut stdout, id, json!(null))?;
                }
            }
        }
    }
    Ok(())
}

fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().ok();
        }
    }
    let len = content_length.ok_or_else(|| "missing Content-Length".to_string())?;
    let mut buf = vec![0u8; len];
    std::io::Read::read_exact(reader, &mut buf).map_err(|e| e.to_string())?;
    let v = serde_json::from_slice(&buf).map_err(|e| e.to_string())?;
    Ok(Some(v))
}

fn respond(out: &mut impl Write, id: Option<Value>, result: Value) -> Result<(), String> {
    let Some(id) = id else {
        return Ok(());
    };
    write_rpc(out, json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn notify(out: &mut impl Write, method: &str, params: Value) -> Result<(), String> {
    write_rpc(out, json!({ "jsonrpc": "2.0", "method": method, "params": params }))
}

fn write_rpc(out: &mut impl Write, v: Value) -> Result<(), String> {
    let body = serde_json::to_vec(&v).map_err(|e| e.to_string())?;
    write!(out, "Content-Length: {}\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
    out.write_all(&body).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn uri_from_params(msg: &Value) -> Option<String> {
    msg.pointer("/params/textDocument/uri")
        .and_then(|u| u.as_str())
        .map(|s| s.to_string())
}

fn doc_from_params(msg: &Value) -> Option<(String, String)> {
    let uri = uri_from_params(msg)?;
    let text = msg
        .pointer("/params/textDocument/text")
        .and_then(|t| t.as_str())?
        .to_string();
    Some((uri, text))
}

fn change_from_params(msg: &Value) -> Option<(String, String)> {
    let uri = uri_from_params(msg)?;
    let text = msg
        .pointer("/params/contentChanges/0/text")
        .and_then(|t| t.as_str())?
        .to_string();
    Some((uri, text))
}

fn publish_diags(out: &mut impl Write, uri: &str, src: &str) -> Result<(), String> {
    let name = uri_to_name(uri);
    let opts = ide::options_for("host", None).unwrap_or_else(|_| crate::driver::CompileOptions {
        target: crate::target::HOST,
        std_path: PathBuf::from("std"),
        include_paths: Vec::new(),
    });
    let result = ide::analyze(&name, src, &opts);
    let diags: Vec<Value> = result
        .diagnostics
        .iter()
        .map(|d| {
            let sev = if d.level == "warning" { 2 } else { 1 };
            json!({
                "range": {
                    "start": { "line": d.line.saturating_sub(1), "character": d.col.saturating_sub(1) },
                    "end": { "line": d.end_line.saturating_sub(1), "character": d.end_col.saturating_sub(1).max(d.col.saturating_sub(1) + 1) }
                },
                "severity": sev,
                "source": "voltc",
                "message": d.message
            })
        })
        .collect();
    notify(
        out,
        "textDocument/publishDiagnostics",
        json!({ "uri": uri, "diagnostics": diags }),
    )
}

fn completion_items(docs: &HashMap<String, String>, msg: &Value) -> Vec<Value> {
    let Some(uri) = uri_from_params(msg) else {
        return Vec::new();
    };
    let Some(src) = docs.get(&uri) else {
        return Vec::new();
    };
    let line = msg.pointer("/params/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let col = msg
        .pointer("/params/position/character")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let offset = offset_at(src, line, col);
    ide::completions(src, offset)
        .into_iter()
        .map(|c| {
            json!({
                "label": c.label,
                "detail": c.detail,
                "insertText": c.insert,
                "kind": match c.kind.as_str() {
                    "keyword" => 14,
                    "snippet" => 15,
                    _ => 6
                }
            })
        })
        .collect()
}

fn hover_item(docs: &HashMap<String, String>, msg: &Value) -> Value {
    let Some(uri) = uri_from_params(msg) else {
        return json!(null);
    };
    let Some(src) = docs.get(&uri) else {
        return json!(null);
    };
    let line = msg.pointer("/params/position/line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let col = msg
        .pointer("/params/position/character")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let offset = offset_at(src, line, col);
    match ide::hover(src, offset, target::HOST) {
        Some(h) => json!({
            "contents": { "kind": "markdown", "value": format!("**{}**\n\n{}", h.title, h.detail) }
        }),
        None => json!(null),
    }
}

fn offset_at(src: &str, line: usize, col: usize) -> usize {
    let mut cur_line = 0usize;
    let mut idx = 0usize;
    for (i, ch) in src.char_indices() {
        if cur_line == line {
            idx = i;
            break;
        }
        if ch == '\n' {
            cur_line += 1;
            idx = i + 1;
        }
    }
    if cur_line != line {
        return src.len();
    }
    idx + col.min(src[idx..].split('\n').next().unwrap_or("").len())
}

fn uri_to_name(uri: &str) -> String {
    let decoded = percent_decode(uri);
    let stripped = decoded
        .strip_prefix("file:///")
        .or_else(|| decoded.strip_prefix("file://"))
        .unwrap_or(&decoded);
    let path = stripped.replace('\\', "/");
    path.rsplit('/').next().unwrap_or("buffer.volt").to_string()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &s[i + 1..i + 3];
            if let Ok(v) = u8::from_str_radix(hex, 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// LSP framing is length-prefixed bytes. Windows CRT otherwise translates
/// `\n` to `\r\n` and corrupts Content-Length.
fn set_stdio_binary() {
    #[cfg(windows)]
    {
        const O_BINARY: i32 = 0x8000;
        extern "C" {
            fn _setmode(fd: i32, mode: i32) -> i32;
        }
        unsafe {
            _setmode(0, O_BINARY);
            _setmode(1, O_BINARY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_file_uri_basename() {
        assert_eq!(
            uri_to_name("file:///C:/Users/ben/blink.volt"),
            "blink.volt"
        );
        assert_eq!(
            uri_to_name("file:///c%3A/Users/ben/foo%20bar.volt"),
            "foo bar.volt"
        );
        assert_eq!(uri_to_name(r"file:///C:\Users\ben\app.volt"), "app.volt");
    }

    #[test]
    fn offset_at_crlf_lines() {
        let src = "line1\r\nline2\r\nx";
        assert_eq!(offset_at(src, 1, 0), 7);
        assert_eq!(&src[offset_at(src, 1, 0)..offset_at(src, 1, 0) + 5], "line2");
    }
}
