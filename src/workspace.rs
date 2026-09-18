//! User folders: programs in `projects/`, extra libraries in `addons/`.
//!
//! Default location is `Documents/Volt` (override with `VOLT_HOME`).

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Workspace {
    pub root: PathBuf,
    pub projects: PathBuf,
    pub addons: PathBuf,
    pub tools: PathBuf,
}

pub fn home_dir() -> PathBuf {
    if let Ok(p) = std::env::var("VOLT_HOME") {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    documents_dir().join("Volt")
}

fn documents_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Documents");
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(profile).join("Documents");
    }
    PathBuf::from(".")
}

/// Create `projects/` and `addons/` if they are missing. Safe to call often.
pub fn ensure() -> Result<Workspace, String> {
    ensure_at(home_dir())
}

fn ensure_at(root: PathBuf) -> Result<Workspace, String> {
    let projects = root.join("projects");
    let addons = root.join("addons");
    let tools = root.join("tools");
    std::fs::create_dir_all(&projects)
        .map_err(|e| format!("cannot create {}: {e}", projects.display()))?;
    std::fs::create_dir_all(&addons)
        .map_err(|e| format!("cannot create {}: {e}", addons.display()))?;
    std::fs::create_dir_all(&tools)
        .map_err(|e| format!("cannot create {}: {e}", tools.display()))?;
    write_if_missing(
        &projects.join("README.txt"),
        "Your Volt programs live in this folder.\n\n\
         Save from the IDE, or drop a .volt file here. Each file (or folder of files)\n\
         is a project.\n",
    );
    write_if_missing(
        &addons.join("README.txt"),
        "Drop extra libraries here (addons / libs).\n\n\
         A library is a .volt file, or a folder with a .volt file of the same name:\n\n\
         addons/servo.volt\n\
         addons/servo/servo.volt\n\n\
         Then in a project:\n\n\
         #include <servo>\n",
    );
    write_if_missing(
        &tools.join("README.txt"),
        "Chip compilers live here (AVR is also inside the Volt app).\n\n\
         The IDE Get compiler button unpacks Pico / ESP toolchains into this folder.\n\
         You can also run:  voltc tools install arm\n",
    );
    Ok(Workspace {
        root,
        projects,
        addons,
        tools,
    })
}

/// Folders the compiler searches for `#include` / `use` besides `std/`.
pub fn addon_search_paths() -> Vec<PathBuf> {
    match ensure() {
        Ok(ws) => vec![ws.addons],
        Err(_) => Vec::new(),
    }
}

fn write_if_missing(path: &Path, text: &str) {
    if !path.exists() {
        let _ = std::fs::write(path, text);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ListedFile {
    pub name: String,
    pub source: String,
    pub kind: String,
}

pub fn list_projects() -> Vec<ListedFile> {
    match ensure() {
        Ok(ws) => list_volt_files(&ws.projects, 2),
        Err(_) => Vec::new(),
    }
}

pub fn list_addons() -> Vec<ListedFile> {
    match ensure() {
        Ok(ws) => list_volt_files(&ws.addons, 3),
        Err(_) => Vec::new(),
    }
}

fn list_volt_files(root: &Path, depth: u32) -> Vec<ListedFile> {
    let mut out = Vec::new();
    collect_volt(root, root, depth, &mut out);
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn collect_volt(root: &Path, dir: &Path, depth: u32, out: &mut Vec<ListedFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let path = ent.path();
        if path.is_dir() {
            if depth > 0 {
                collect_volt(root, &path, depth - 1, out);
            }
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("volt") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(ListedFile {
            name,
            source,
            kind: "file".into(),
        });
    }
}

pub fn save_project(name: &str, source: &str) -> Result<PathBuf, String> {
    let ws = ensure()?;
    save_project_in(&ws, name, source)
}

fn save_project_in(ws: &Workspace, name: &str, source: &str) -> Result<PathBuf, String> {
    let path = safe_child(&ws.projects, name)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, source).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

fn safe_child(root: &Path, name: &str) -> Result<PathBuf, String> {
    let raw = name.trim().trim_start_matches(['/', '\\']);
    if raw.is_empty() {
        return Err("missing file name".into());
    }
    let mut path = root.to_path_buf();
    for comp in Path::new(raw).components() {
        match comp {
            Component::Normal(part) => path.push(part),
            _ => return Err("file name cannot contain `..`".into()),
        }
    }
    if path.extension().and_then(|s| s.to_str()) != Some("volt") {
        path.set_extension("volt");
    }
    if !path.starts_with(root) {
        return Err("file would land outside the projects folder".into());
    }
    Ok(path)
}

pub fn reveal(path: &Path) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("open").arg(path).status();
    }
    #[cfg(target_os = "windows")]
    {
        return std::process::Command::new("explorer").arg(path).status();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::process::Command::new("xdg-open").arg(path).status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_projects_and_addons() {
        let dir = std::env::temp_dir().join(format!(
            "volt-home-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let ws = ensure_at(dir.clone()).expect("workspace");
        assert!(ws.projects.is_dir());
        assert!(ws.addons.is_dir());
        assert!(ws.tools.is_dir());
        let saved = save_project_in(&ws, "hello", "function main() { return }\n").expect("save");
        assert!(saved.ends_with("hello.volt"));
        assert!(list_volt_files(&ws.projects, 2)
            .iter()
            .any(|f| f.name == "hello.volt"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
