//! Local Volt IDE: `voltc ide` serves a Monaco editor on localhost.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use crate::ide::{self, AnalyzeResult};
use crate::target;

const INDEX_HTML: &str = include_str!("../ide/web/index.html");

pub fn run(bind: &str, open_browser: bool) -> Result<(), String> {
    let listener = TcpListener::bind(bind).map_err(|e| format!("cannot bind {bind}: {e}"))?;
    let url = url_of(&listener)?;
    eprintln!("Volt IDE listening on {url}");
    match crate::workspace::ensure() {
        Ok(ws) => {
            eprintln!("projects  {}", ws.projects.display());
            eprintln!("addons    {}", ws.addons.display());
        }
        Err(e) => eprintln!("workspace: {e}"),
    }
    if open_browser {
        let _ = open_url(&url);
    }
    serve(listener)
}

/// Bind and serve on a background thread. Returns the `http://host:port` URL.
pub fn spawn(bind: &str) -> Result<String, String> {
    let _ = crate::workspace::ensure();
    let listener = TcpListener::bind(bind).map_err(|e| format!("cannot bind {bind}: {e}"))?;
    let url = url_of(&listener)?;
    std::thread::Builder::new()
        .name("volt-ide".into())
        .spawn(move || {
            if let Err(e) = serve(listener) {
                eprintln!("ide: {e}");
            }
        })
        .map_err(|e| format!("cannot spawn IDE server: {e}"))?;
    wait_ready(&url)?;
    Ok(url)
}

fn url_of(listener: &TcpListener) -> Result<String, String> {
    let addr = listener
        .local_addr()
        .map_err(|e| format!("local addr: {e}"))?;
    Ok(format!("http://{addr}"))
}

fn wait_ready(url: &str) -> Result<(), String> {
    let hostport = url.strip_prefix("http://").unwrap_or(url);
    for _ in 0..50 {
        if std::net::TcpStream::connect(hostport).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    Err(format!("IDE server did not start at {url}"))
}

fn serve(listener: TcpListener) -> Result<(), String> {
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                if let Err(e) = handle(s) {
                    eprintln!("ide: {e}");
                }
            }
            Err(e) => eprintln!("ide accept: {e}"),
        }
    }
    Ok(())
}

fn open_url(url: &str) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open").arg(url).status();
        if status.as_ref().map(|s| s.success()).unwrap_or(false) {
            return status;
        }
        return std::process::Command::new("open")
            .args(["-a", "Safari", url])
            .status();
    }
    #[cfg(target_os = "windows")]
    {
        // `start` treats the first quoted arg as a window title. The empty
        // title is required so the URL is not swallowed. Fall back to explorer.
        let status = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .status();
        if status.as_ref().map(|s| s.success()).unwrap_or(false) {
            return status;
        }
        return std::process::Command::new("explorer").arg(url).status();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::process::Command::new("xdg-open").arg(url).status()
    }
}

fn handle(mut stream: TcpStream) -> Result<(), String> {
    let mut buf = vec![0u8; 64 * 1024];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    if n == 0 {
        return Ok(());
    }
    let req = std::str::from_utf8(&buf[..n]).map_err(|e| e.to_string())?;
    let req = req.replace("\r\n", "\n");
    let mut lines = req.split('\n');
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    let mut content_length = 0usize;
    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let header_end = buf[..n]
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4)
        .or_else(|| buf[..n].windows(2).position(|w| w == b"\n\n").map(|i| i + 2))
        .unwrap_or(n);
    let mut body = buf[header_end.min(n)..n].to_vec();
    while body.len() < content_length {
        let mut more = vec![0u8; content_length - body.len()];
        let k = stream.read(&mut more).map_err(|e| e.to_string())?;
        if k == 0 {
            break;
        }
        body.extend_from_slice(&more[..k]);
    }
    let body = String::from_utf8_lossy(&body).into_owned();

    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            write_http(&mut stream, "200 OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes())
        }
        ("GET", "/api/meta") => json(&mut stream, meta_json()),
        ("GET", "/api/examples") => json(&mut stream, examples_json()),
        ("GET", "/api/projects") => json(&mut stream, files_json(crate::workspace::list_projects())),
        ("GET", "/api/addons") => json(&mut stream, files_json(crate::workspace::list_addons())),
        ("POST", "/api/save") => json(&mut stream, save_json(&body)),
        ("POST", "/api/open-folder") => json(&mut stream, open_folder_json(&body)),
        ("POST", "/api/analyze") => json(&mut stream, analyze_json(&body)),
        ("POST", "/api/complete") => json(&mut stream, complete_json(&body)),
        ("POST", "/api/hover") => json(&mut stream, hover_json(&body)),
        _ => write_http(&mut stream, "404 Not Found", "text/plain", b"not found"),
    }
}

fn write_http(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) -> Result<(), String> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
    stream.write_all(body).map_err(|e| e.to_string())?;
    Ok(())
}

fn json(stream: &mut TcpStream, value: serde_json::Value) -> Result<(), String> {
    let body = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    write_http(stream, "200 OK", "application/json", &body)
}

fn meta_json() -> serde_json::Value {
    let boards: Vec<serde_json::Value> = crate::board::all_boards()
        .iter()
        .map(|b| {
            serde_json::json!({
                "id": b.name,
                "name": b.display,
                "fqbn": b.fqbn,
                "mcu": b.mcu,
                "package": b.package,
                "led": b.target.led,
                "aliases": b.aliases,
            })
        })
        .collect();
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": crate::ports::os_name(),
        "targets": boards.iter().map(|b| b["id"].clone()).collect::<Vec<_>>(),
        "boards": boards,
        "ports": crate::ports::list_serial_ports().iter().map(|p| {
            serde_json::json!({ "address": p.address, "label": p.label })
        }).collect::<Vec<_>>(),
        "workspace": match crate::workspace::ensure() {
            Ok(ws) => serde_json::json!({
                "root": ws.root.to_string_lossy(),
                "projects": ws.projects.to_string_lossy(),
                "addons": ws.addons.to_string_lossy(),
            }),
            Err(e) => serde_json::json!({ "error": e }),
        },
    })
}

fn examples_json() -> serde_json::Value {
    let mut examples = Vec::new();
    push_example(&mut examples, "add.volt", include_str!("../examples/add.volt"));
    push_example(&mut examples, "blink.volt", include_str!("../examples/blink.volt"));
    push_example(
        &mut examples,
        "registers.volt",
        include_str!("../examples/registers.volt"),
    );
    push_example(&mut examples, "isr.volt", include_str!("../examples/isr.volt"));
    push_example(&mut examples, "branch.volt", include_str!("../examples/branch.volt"));
    push_example(&mut examples, "math.volt", include_str!("../examples/math.volt"));
    push_example(&mut examples, "power.volt", include_str!("../examples/power.volt"));
    if let Ok(dir) = std::fs::read_dir("examples") {
        for ent in dir.flatten() {
            let path = ent.path();
            if path.extension().and_then(|s| s.to_str()) != Some("volt") {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if examples.iter().any(|e| e["name"] == name) {
                continue;
            }
            if let Ok(src) = std::fs::read_to_string(&path) {
                push_example(&mut examples, &name, &src);
            }
        }
    }
    serde_json::json!(examples)
}

fn push_example(out: &mut Vec<serde_json::Value>, name: &str, source: &str) {
    out.push(serde_json::json!({ "name": name, "source": source }));
}

fn files_json(files: Vec<crate::workspace::ListedFile>) -> serde_json::Value {
    serde_json::to_value(files).unwrap_or(serde_json::json!([]))
}

#[derive(serde::Deserialize)]
struct SaveReq {
    name: String,
    source: String,
}

fn save_json(body: &str) -> serde_json::Value {
    match serde_json::from_str::<SaveReq>(body) {
        Ok(req) => match crate::workspace::save_project(&req.name, &req.source) {
            Ok(path) => serde_json::json!({ "ok": true, "path": path.to_string_lossy() }),
            Err(e) => serde_json::json!({ "ok": false, "error": e }),
        },
        Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
    }
}

#[derive(serde::Deserialize)]
struct OpenFolderReq {
    #[serde(default)]
    which: String,
}

fn open_folder_json(body: &str) -> serde_json::Value {
    let which = serde_json::from_str::<OpenFolderReq>(body)
        .map(|r| r.which)
        .unwrap_or_default();
    match crate::workspace::ensure() {
        Ok(ws) => {
            let path = if which == "addons" {
                ws.addons
            } else {
                ws.projects
            };
            match crate::workspace::reveal(&path) {
                Ok(_) => serde_json::json!({ "ok": true, "path": path.to_string_lossy() }),
                Err(e) => serde_json::json!({ "ok": false, "error": e.to_string() }),
            }
        }
        Err(e) => serde_json::json!({ "ok": false, "error": e }),
    }
}

#[derive(serde::Deserialize)]
struct AnalyzeReq {
    source: String,
    #[serde(default = "default_target")]
    target: String,
    #[serde(default = "default_name")]
    name: String,
}

fn default_target() -> String {
    "host".into()
}
fn default_name() -> String {
    "main.volt".into()
}

fn analyze_json(body: &str) -> serde_json::Value {
    match serde_json::from_str::<AnalyzeReq>(body) {
        Ok(req) => match ide::options_for(&req.target, None) {
            Ok(opts) => {
                let result: AnalyzeResult = ide::analyze(&req.name, &req.source, &opts);
                serde_json::to_value(result).unwrap_or(serde_json::json!({"ok": false}))
            }
            Err(e) => serde_json::json!({"ok": false, "diagnostics": [{"level":"error","message": e, "file":"","line":1,"col":1,"end_line":1,"end_col":1}]}),
        },
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    }
}

#[derive(serde::Deserialize)]
struct CursorReq {
    source: String,
    offset: usize,
    #[serde(default = "default_target")]
    target: String,
}

fn complete_json(body: &str) -> serde_json::Value {
    match serde_json::from_str::<CursorReq>(body) {
        Ok(req) => serde_json::json!({ "items": ide::completions(&req.source, req.offset) }),
        Err(e) => serde_json::json!({ "error": e.to_string(), "items": [] }),
    }
}

fn hover_json(body: &str) -> serde_json::Value {
    match serde_json::from_str::<CursorReq>(body) {
        Ok(req) => {
            let target = target::find_target(&req.target).copied().unwrap_or(crate::target::HOST);
            match ide::hover(&req.source, req.offset, target) {
                Some(h) => serde_json::to_value(h).unwrap_or(serde_json::json!({})),
                None => serde_json::json!({}),
            }
        }
        Err(_) => serde_json::json!({}),
    }
}

#[allow(dead_code)]
fn _std_hint() -> PathBuf {
    PathBuf::from("std")
}
