//! Bundled and on-demand chip compilers (Arduino-style).
//!
//! AVR gcc + avrdude ship in the installer. Pico/STM32 (ARM) and ESP packs
//! download into `Documents/Volt/tools` when you click **Get compiler**.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::board::Board;
use crate::target::Family;
use crate::workspace;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolStatus {
    pub pack: String,
    pub name: String,
    pub installed: bool,
    pub ships_in_installer: bool,
    pub path: Option<String>,
    pub cc: Option<String>,
    pub note: String,
}

pub struct Pack {
    pub id: &'static str,
    pub name: &'static str,
    pub ships_in_installer: bool,
    pub cc_names: &'static [&'static str],
    pub flash_names: &'static [&'static str],
    pub fetches: &'static [Fetch],
}

pub struct Fetch {
    pub os: OsKind,
    pub url: &'static str,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OsKind {
    Mac,
    Windows,
    Linux,
}

pub fn all_packs() -> &'static [Pack] {
    PACKS
}

pub fn pack_id_for_board(board: &Board) -> Option<&'static str> {
    pack_id_for_family(board.target.family)
}

pub fn pack_id_for_family(family: Family) -> Option<&'static str> {
    match family {
        Family::Avr => Some("avr"),
        Family::Rp2040 | Family::Stm32F1 | Family::Stm32F4 | Family::Arm => Some("arm"),
        Family::Esp32 => Some("esp32"),
        Family::Esp8266 => Some("esp8266"),
        Family::Host => None,
    }
}

pub fn find_pack(id: &str) -> Option<&'static Pack> {
    PACKS.iter().find(|p| p.id.eq_ignore_ascii_case(id))
}

pub fn resolve_pack(id: &str) -> Result<&'static Pack, String> {
    if let Some(pack) = find_pack(id) {
        return Ok(pack);
    }
    let ids: Vec<&str> = PACKS.iter().map(|p| p.id).collect();
    if let Some(fixed) = crate::board::unique_typo(id, &ids) {
        if let Some(pack) = find_pack(fixed) {
            return Ok(pack);
        }
    }
    Err(format!("unknown tool pack `{id}`"))
}

pub fn status_for_board(board_id: &str) -> ToolStatus {
    let board = match crate::board::resolve_board(board_id) {
        Ok(r) => r.board,
        Err(note) => {
            return ToolStatus {
                pack: String::new(),
                name: "unknown board".into(),
                installed: false,
                ships_in_installer: false,
                path: None,
                cc: None,
                note,
            };
        }
    };
    match pack_id_for_board(board) {
        Some(id) => status(id),
        None => ToolStatus {
            pack: "host".into(),
            name: "Host C compiler".into(),
            installed: which("cc").is_some() || which("cl").is_some(),
            ships_in_installer: false,
            path: None,
            cc: which("cc")
                .or_else(|| which("cl"))
                .map(|p| p.display().to_string()),
            note: "Uses the C compiler already on this computer.".into(),
        },
    }
}

pub fn status(pack_id: &str) -> ToolStatus {
    let pack = match resolve_pack(pack_id) {
        Ok(pack) => pack,
        Err(note) => {
            return ToolStatus {
                pack: pack_id.into(),
                name: pack_id.into(),
                installed: false,
                ships_in_installer: false,
                path: None,
                cc: None,
                note,
            };
        }
    };
    let root = pack_root(pack);
    let cc = root
        .as_ref()
        .and_then(|r| find_named(r, pack.cc_names));
    let installed = cc.is_some();
    let note = if installed {
        format!("Ready ({})", pack.name)
    } else if pack.ships_in_installer {
        format!(
            "{} is missing. Run the Volt installer, or click Get compiler.",
            pack.name
        )
    } else {
        format!(
            "{} is not installed. Click Get compiler to download it into Documents/Volt/tools.",
            pack.name
        )
    };
    ToolStatus {
        pack: pack.id.into(),
        name: pack.name.into(),
        installed,
        ships_in_installer: pack.ships_in_installer,
        path: root.map(|p| p.display().to_string()),
        cc: cc.map(|p| p.display().to_string()),
        note,
    }
}

pub fn list_status() -> Vec<ToolStatus> {
    PACKS.iter().map(|p| status(p.id)).collect()
}

/// Download a pack into `Documents/Volt/tools/<id>`.
pub fn install(pack_id: &str) -> Result<ToolStatus, String> {
    let pack = resolve_pack(pack_id)?;
    let dest = user_tools_dir()?.join(pack.id);
    let urls = fetches_for_this_os(pack);
    if urls.is_empty() {
        return Err(format!("no {} download for this computer", pack.name));
    }
    // Unpack on local disk first. Extracting straight into Documents can stop
    // halfway when that folder is iCloud-backed, which left ESP8266 without gcc.
    let staging = std::env::temp_dir().join(format!(
        "volt-{}-{}-{}",
        pack.id,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)
        .map_err(|e| format!("cannot create {}: {e}", staging.display()))?;
    let result = (|| {
        let dl = staging.join(".download");
        std::fs::create_dir_all(&dl)
            .map_err(|e| format!("cannot create {}: {e}", dl.display()))?;
        for (i, fetch) in urls.iter().enumerate() {
            let archive = dl.join(format!("part-{i}{}", archive_suffix(fetch.url)));
            download(fetch.url, &archive)?;
            extract(&archive, &staging)?;
        }
        let _ = std::fs::remove_dir_all(&dl);
        write_notice(&staging, pack);
        if find_named(&staging, pack.cc_names).is_none() {
            return Err(format!(
                "downloaded {} but did not find {} — the archive layout may have changed",
                pack.name,
                pack.cc_names.first().copied().unwrap_or("compiler")
            ));
        }
        move_dir(&staging, &dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result?;
    let st = status(pack.id);
    if !st.installed {
        return Err(format!(
            "installed {} but Volt cannot see {} yet",
            pack.name,
            pack.cc_names.first().copied().unwrap_or("compiler")
        ));
    }
    Ok(st)
}

pub fn user_tools_dir() -> Result<PathBuf, String> {
    let ws = workspace::ensure()?;
    std::fs::create_dir_all(&ws.tools)
        .map_err(|e| format!("cannot create {}: {e}", ws.tools.display()))?;
    Ok(ws.tools)
}

pub fn pack_root(pack: &Pack) -> Option<PathBuf> {
    for root in tool_search_roots() {
        let dir = root.join(pack.id);
        if dir.is_dir() && find_named(&dir, pack.cc_names).is_some() {
            return Some(dir);
        }
    }
    None
}

pub fn find_cc(board: &Board) -> Option<PathBuf> {
    if let Some(id) = pack_id_for_board(board) {
        if let Some(pack) = find_pack(id) {
            if let Some(root) = pack_root(pack) {
                if let Some(cc) = find_named(&root, pack.cc_names) {
                    return Some(cc);
                }
            }
        }
    }
    which(board.cc)
}

pub fn find_flash_tool(board: &Board) -> Option<PathBuf> {
    let id = pack_id_for_board(board)?;
    let pack = find_pack(id)?;
    let root = pack_root(pack)?;
    find_named(&root, pack.flash_names).or_else(|| {
        pack.flash_names
            .iter()
            .find_map(|n| which(Path::new(n).file_stem().and_then(|s| s.to_str()).unwrap_or(n)))
    })
}

/// Compile generated C with the board's bundled (or PATH) toolchain.
pub fn build_firmware(c_source: &str, board: &Board, out_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    let c_path = out_dir.join("firmware.c");
    std::fs::write(&c_path, c_source).map_err(|e| format!("cannot write {}: {e}", c_path.display()))?;
    let cc = find_cc(board).ok_or_else(|| {
        format!(
            "no compiler for {} — click Get compiler, or run: voltc tools install {}",
            board.display,
            pack_id_for_board(board).unwrap_or("host")
        )
    })?;
    let mut cmd = Command::new(&cc);
    for flag in board.cc_flags.split_whitespace() {
        cmd.arg(flag);
    }
    cmd.arg(&c_path);
    match board.target.family {
        Family::Avr => {
            let elf = out_dir.join("firmware.elf");
            cmd.arg("-o").arg(&elf);
            run_cmd(&mut cmd, "avr-gcc")?;
            let objcopy = cc
                .parent()
                .map(|p| p.join(exe("avr-objcopy")))
                .filter(|p| p.is_file())
                .or_else(|| which("avr-objcopy"))
                .ok_or("avr-objcopy not found next to avr-gcc")?;
            let hex = out_dir.join("firmware.hex");
            run_cmd(
                Command::new(objcopy)
                    .args(["-O", "ihex", "-R", ".eeprom"])
                    .arg(&elf)
                    .arg(&hex),
                "avr-objcopy",
            )?;
            Ok(hex)
        }
        Family::Host => {
            let bin = out_dir.join(exe("firmware"));
            cmd.arg("-o").arg(&bin);
            run_cmd(&mut cmd, "cc")?;
            Ok(bin)
        }
        _ => {
            let elf = out_dir.join("firmware.elf");
            cmd.arg("-o").arg(&elf);
            run_cmd(&mut cmd, board.cc)?;
            Ok(elf)
        }
    }
}

pub fn flash_firmware(firmware: &Path, board: &Board, port: &str) -> Result<String, String> {
    match board.target.family {
        Family::Host => run_host_firmware(firmware),
        Family::Avr => flash_avrdude(firmware, board, port),
        Family::Esp8266 | Family::Esp32 => flash_esptool(firmware, board, port),
        _ => {
            let flash = board.flash.replace("<PORT>", port);
            Err(format!(
                "auto-flash is not wired for {} — run:\n  {flash}",
                board.display
            ))
        }
    }
}

fn flash_avrdude(firmware: &Path, board: &Board, port: &str) -> Result<String, String> {
    let flash = board.flash.replace("<PORT>", port);
    let dude = find_flash_tool(board)
        .or_else(|| which("avrdude"))
        .ok_or("avrdude not found — install the AVR pack")?;
    let mut args: Vec<String> = Vec::new();
    if let Some(conf) = dude
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("etc/avrdude.conf"))
        .filter(|p| p.is_file())
    {
        args.push("-C".into());
        args.push(conf.display().to_string());
    }
    for part in flash.split_whitespace().skip(1) {
        if part.starts_with("flash:w:") {
            args.push(format!("flash:w:{}:i", firmware.display()));
        } else {
            args.push(part.to_string());
        }
    }
    let out = Command::new(&dude)
        .args(&args)
        .output()
        .map_err(|e| format!("avrdude: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string()
        + &String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(text);
    }
    Ok(text)
}

fn run_host_firmware(firmware: &Path) -> Result<String, String> {
    let mut child = Command::new(firmware)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {}: {e}", firmware.display()))?;
    let timeout = Duration::from_secs(4);
    let start = Instant::now();
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(st)) => break (st, false),
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let st = child
                    .wait()
                    .map_err(|e| format!("run {}: {e}", firmware.display()))?;
                break (st, true);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("run {}: {e}", firmware.display())),
        }
    };
    let mut text = String::new();
    if let Some(mut s) = child.stdout.take() {
        let _ = s.read_to_string(&mut text);
    }
    if let Some(mut s) = child.stderr.take() {
        let mut err = String::new();
        let _ = s.read_to_string(&mut err);
        text.push_str(&err);
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    if timed_out {
        text.push_str("(stopped after 4s — host programs with loop() keep running until stopped)\n");
        return Ok(text);
    }
    text.push_str(&format!(
        "ran {}  exit {}\n",
        firmware.display(),
        status.code().unwrap_or(-1)
    ));
    Ok(text)
}

fn flash_esptool(firmware: &Path, board: &Board, port: &str) -> Result<String, String> {
    if port.is_empty() || port.contains('*') {
        return Err("plug in the board and pick a serial port".into());
    }
    let tool = find_esptool()?;
    let chip = esp_chip(board);
    let image = esp_flash_image(firmware, board, &tool, chip)?;
    let mut cmd = tool.command();
    cmd.args(["--chip", chip, "-p", port, "write_flash"]);
    if chip.starts_with("esp8266") {
        cmd.args(["-fs", "4MB"]);
    }
    cmd.arg("0x0").arg(&image);
    let out = cmd
        .output()
        .map_err(|e| format!("esptool: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout).to_string()
        + &String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(if text.trim().is_empty() {
            "esptool failed".into()
        } else {
            text
        });
    }
    Ok(text)
}

fn esp_chip(board: &Board) -> &'static str {
    let mut parts = board.flash.split_whitespace();
    while let Some(p) = parts.next() {
        if p == "--chip" {
            if let Some(chip) = parts.next() {
                return chip;
            }
        }
    }
    match board.target.family {
        Family::Esp32 => "esp32",
        _ => "esp8266",
    }
}

fn esp_flash_image(
    firmware: &Path,
    board: &Board,
    tool: &EspTool,
    chip: &str,
) -> Result<PathBuf, String> {
    let is_elf = firmware
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("elf"));
    if !is_elf {
        return Ok(firmware.to_path_buf());
    }
    let bin = firmware.with_extension("bin");
    if tool.is_python {
        let mut cmd = tool.command();
        let out = cmd
            .args(["--chip", chip, "elf2image", "-o"])
            .arg(&bin)
            .arg(firmware)
            .output();
        if let Ok(out) = out {
            if out.status.success() && bin.is_file() {
                return Ok(bin);
            }
        }
    }
    objcopy_binary(board, firmware, &bin)?;
    Ok(bin)
}

fn objcopy_binary(board: &Board, elf: &Path, bin: &Path) -> Result<(), String> {
    let cc = find_cc(board).ok_or("ESP compiler not found")?;
    let name = cc.file_name().and_then(|s| s.to_str()).unwrap_or("gcc");
    let objcopy = cc.parent().map(|p| p.join(name.replace("gcc", "objcopy")));
    let objcopy = objcopy
        .filter(|p| p.is_file())
        .or_else(|| which("xtensa-lx106-elf-objcopy"))
        .or_else(|| which("xtensa-esp32-elf-objcopy"))
        .ok_or("objcopy not found next to the ESP compiler")?;
    run_cmd(
        Command::new(objcopy)
            .args(["-O", "binary"])
            .arg(elf)
            .arg(bin),
        "objcopy",
    )
}

struct EspTool {
    program: PathBuf,
    extra_args: Vec<String>,
    pythonpath: Option<PathBuf>,
    is_python: bool,
}

impl EspTool {
    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.extra_args);
        if let Some(path) = &self.pythonpath {
            cmd.env("PYTHONPATH", path);
        }
        cmd
    }
}

fn find_esptool() -> Result<EspTool, String> {
    if let Some(tool) = esptool_python(None) {
        return Ok(tool);
    }
    if let Some(p) = which("esptool.py") {
        return Ok(EspTool {
            program: p,
            extra_args: Vec::new(),
            pythonpath: None,
            is_python: true,
        });
    }
    let bundled = user_tools_dir().ok().map(|d| d.join("esptool-py"));
    if let Some(dir) = bundled.as_ref() {
        if let Some(tool) = esptool_python(Some(dir)) {
            return Ok(tool);
        }
    }
    if let (Some(py), Some(dir)) = (python_bin(), bundled) {
        let _ = std::fs::create_dir_all(&dir);
        let st = Command::new(&py)
            .args(["-m", "pip", "install", "--quiet", "--upgrade", "--target"])
            .arg(&dir)
            .arg("esptool")
            .status();
        if st.map(|s| s.success()).unwrap_or(false) {
            if let Some(tool) = esptool_python(Some(&dir)) {
                return Ok(tool);
            }
        }
    }
    Err("esptool not found — install with:  pip3 install esptool".into())
}

fn python_bin() -> Option<PathBuf> {
    which("python3").or_else(|| which("python"))
}

fn esptool_python(pythonpath: Option<&Path>) -> Option<EspTool> {
    let py = python_bin()?;
    let mut cmd = Command::new(&py);
    cmd.args(["-m", "esptool", "version"]);
    if let Some(path) = pythonpath {
        cmd.env("PYTHONPATH", path);
    }
    let ok = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return None;
    }
    Some(EspTool {
        program: py,
        extra_args: vec!["-m".into(), "esptool".into()],
        pythonpath: pythonpath.map(|p| p.to_path_buf()),
        is_python: true,
    })
}

static PACKS: &[Pack] = &[
    Pack {
        id: "avr",
        name: "AVR (UNO / Nano / Mega)",
        ships_in_installer: true,
        cc_names: &["avr-gcc", "avr-gcc.exe"],
        flash_names: &["avrdude", "avrdude.exe"],
        fetches: &[
            Fetch {
                os: OsKind::Mac,
                url: "https://downloads.arduino.cc/tools/avr-gcc-7.3.0-atmel3.6.1-arduino7-x86_64-apple-darwin14.tar.bz2",
            },
            Fetch {
                os: OsKind::Mac,
                url: "https://downloads.arduino.cc/tools/avrdude-6.3.0-arduino17-x86_64-apple-darwin12.tar.bz2",
            },
            Fetch {
                os: OsKind::Windows,
                url: "https://downloads.arduino.cc/tools/avr-gcc-7.3.0-atmel3.6.1-arduino7-i686-w64-mingw32.zip",
            },
            Fetch {
                os: OsKind::Windows,
                url: "https://downloads.arduino.cc/tools/avrdude-6.3.0-arduino17-i686-w64-mingw32.zip",
            },
            Fetch {
                os: OsKind::Linux,
                url: "https://downloads.arduino.cc/tools/avr-gcc-7.3.0-atmel3.6.1-arduino7-x86_64-pc-linux-gnu.tar.bz2",
            },
            Fetch {
                os: OsKind::Linux,
                url: "https://downloads.arduino.cc/tools/avrdude-6.3.0-arduino17-x86_64-pc-linux-gnu.tar.bz2",
            },
        ],
    },
    Pack {
        id: "arm",
        name: "ARM (Pico / STM32 / Cortex-M)",
        ships_in_installer: false,
        cc_names: &[
            "arm-none-eabi-gcc",
            "arm-none-eabi-gcc.exe",
        ],
        flash_names: &["picotool", "st-flash", "picotool.exe", "st-flash.exe"],
        fetches: &[
            Fetch {
                os: OsKind::Mac,
                url: arm_xpack_url(),
            },
            Fetch {
                os: OsKind::Windows,
                url: "https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack/releases/download/v13.2.1-1.1/xpack-arm-none-eabi-gcc-13.2.1-1.1-win32-x64.zip",
            },
            Fetch {
                os: OsKind::Linux,
                url: "https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack/releases/download/v13.2.1-1.1/xpack-arm-none-eabi-gcc-13.2.1-1.1-linux-x64.tar.gz",
            },
        ],
    },
    Pack {
        id: "esp8266",
        name: "ESP8266 / ESP8000",
        ships_in_installer: false,
        cc_names: &["xtensa-lx106-elf-gcc", "xtensa-lx106-elf-gcc.exe"],
        flash_names: &["esptool", "esptool.py", "esptool.exe"],
        fetches: &[
            Fetch {
                os: OsKind::Mac,
                url: "https://github.com/earlephilhower/esp-quick-toolchain/releases/download/3.2.0-gcc10.3/x86_64-apple-darwin14.xtensa-lx106-elf-c791b74.230224.tar.gz",
            },
            Fetch {
                os: OsKind::Windows,
                url: "https://github.com/earlephilhower/esp-quick-toolchain/releases/download/3.2.0-gcc10.3/x86_64-w64-mingw32.xtensa-lx106-elf-c791b74.230224.zip",
            },
            Fetch {
                os: OsKind::Linux,
                url: esp8266_linux_gcc_url(),
            },
        ],
    },
    Pack {
        id: "esp32",
        name: "ESP32",
        ships_in_installer: false,
        cc_names: &[
            "xtensa-esp32-elf-gcc",
            "xtensa-esp32s3-elf-gcc",
            "xtensa-esp32-elf-gcc.exe",
        ],
        flash_names: &["esptool", "esptool.py", "esptool.exe"],
        fetches: &[
            Fetch {
                os: OsKind::Mac,
                url: "https://github.com/espressif/crosstool-NG/releases/download/esp-12.2.0_20230208/xtensa-esp32-elf-12.2.0_20230208-x86_64-apple-darwin.tar.xz",
            },
            Fetch {
                os: OsKind::Windows,
                url: "https://github.com/espressif/crosstool-NG/releases/download/esp-12.2.0_20230208/xtensa-esp32-elf-12.2.0_20230208-x86_64-w64-mingw32.zip",
            },
            Fetch {
                os: OsKind::Linux,
                url: "https://github.com/espressif/crosstool-NG/releases/download/esp-12.2.0_20230208/xtensa-esp32-elf-12.2.0_20230208-x86_64-linux-gnu.tar.xz",
            },
        ],
    },
];

const fn arm_xpack_url() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack/releases/download/v13.2.1-1.1/xpack-arm-none-eabi-gcc-13.2.1-1.1-darwin-arm64.tar.gz"
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        "https://github.com/xpack-dev-tools/arm-none-eabi-gcc-xpack/releases/download/v13.2.1-1.1/xpack-arm-none-eabi-gcc-13.2.1-1.1-darwin-x64.tar.gz"
    }
}

/// ESP8266 gcc is still Intel-only on macOS (Rosetta). Linux has a native aarch64 build.
const fn esp8266_linux_gcc_url() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "https://github.com/earlephilhower/esp-quick-toolchain/releases/download/3.2.0-gcc10.3/aarch64-linux-gnu.xtensa-lx106-elf-c791b74.230224.tar.gz"
    }
    #[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
    {
        "https://github.com/earlephilhower/esp-quick-toolchain/releases/download/3.2.0-gcc10.3/x86_64-linux-gnu.xtensa-lx106-elf-c791b74.230224.tar.gz"
    }
}

fn current_os() -> OsKind {
    if cfg!(target_os = "macos") {
        OsKind::Mac
    } else if cfg!(windows) {
        OsKind::Windows
    } else {
        OsKind::Linux
    }
}

fn fetches_for_this_os(pack: &Pack) -> Vec<&'static Fetch> {
    let os = current_os();
    pack.fetches.iter().filter(|f| f.os == os).collect()
}

fn tool_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(p) = std::env::var("VOLT_TOOLS") {
        let p = p.trim();
        if !p.is_empty() {
            roots.push(PathBuf::from(p));
        }
    }
    if let Ok(ws) = workspace::ensure() {
        roots.push(ws.tools);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            roots.push(dir.join("../Resources/tools"));
            roots.push(dir.join("tools"));
            let mut walk = dir.to_path_buf();
            for _ in 0..8 {
                if walk.join("Cargo.toml").is_file() {
                    roots.push(walk.join("tools"));
                    break;
                }
                match walk.parent() {
                    Some(p) => walk = p.to_path_buf(),
                    None => break,
                }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("tools"));
        roots.push(cwd.join("../Resources/tools"));
    }
    roots
}

fn find_named(root: &Path, names: &[&str]) -> Option<PathBuf> {
    let mut stack = vec![(root.to_path_buf(), 0u32)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in entries.flatten() {
            let path = ent.path();
            if path.is_dir() {
                if depth < 8 {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            let fname = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if names.iter().any(|n| fname.eq_ignore_ascii_case(n)) {
                return Some(path);
            }
        }
    }
    None
}

fn which(name: &str) -> Option<PathBuf> {
    let Ok(path) = std::env::var("PATH") else {
        return None;
    };
    for dir in std::env::split_paths(&path) {
        let p = dir.join(exe(name));
        if p.is_file() {
            return Some(p);
        }
        #[cfg(windows)]
        {
            let p = dir.join(format!("{name}.exe"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn exe(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let curl = if cfg!(windows) { "curl.exe" } else { "curl" };
    let st = Command::new(curl)
        .args(["-L", "--fail", "--retry", "2", "-sS", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| format!("curl failed ({e}) — install curl to download compilers"))?;
    if !st.success() {
        return Err(format!("download failed: {url}"));
    }
    Ok(())
}

fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let mut cmd = Command::new("tar");
    if name.ends_with(".zip") {
        cmd.args(["-xf"]);
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        cmd.args(["-xJf"]);
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        cmd.args(["-xzf"]);
    } else {
        cmd.args(["-xjf"]);
    }
    let out = cmd
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .output()
        .map_err(|e| format!("tar failed: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!("could not unpack {}: {err}", archive.display()));
    }
    Ok(())
}

fn move_dir(src: &Path, dest: &Path) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(dest);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    copy_tree(src, dest)?;
    let _ = std::fs::remove_dir_all(src);
    Ok(())
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest)
        .map_err(|e| format!("cannot create {}: {e}", dest.display()))?;
    let entries = std::fs::read_dir(src)
        .map_err(|e| format!("cannot read {}: {e}", src.display()))?;
    for ent in entries {
        let ent = ent.map_err(|e| e.to_string())?;
        let from = ent.path();
        let to = dest.join(ent.file_name());
        let ft = ent.file_type().map_err(|e| e.to_string())?;
        if ft.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| format!("cannot copy {}: {e}", from.display()))?;
            #[cfg(unix)]
            {
                if let Ok(meta) = std::fs::metadata(&from) {
                    let _ = std::fs::set_permissions(&to, meta.permissions());
                }
            }
        }
    }
    Ok(())
}

fn archive_suffix(url: &str) -> &'static str {
    if url.ends_with(".zip") {
        ".zip"
    } else if url.contains(".tar.xz") {
        ".tar.xz"
    } else if url.contains(".tar.gz") {
        ".tar.gz"
    } else {
        ".tar.bz2"
    }
}

fn write_notice(dest: &Path, pack: &Pack) {
    let text = format!(
        "{} for Volt\n\nThese binaries come from Arduino / xPack / Espressif.\n\
         GCC is GPL; avrdude is GPL. Source is on their project pages.\n\
         Pack id: {}\n",
        pack.name, pack.id
    );
    let _ = std::fs::write(dest.join("README.txt"), text);
}

fn run_cmd(cmd: &mut Command, name: &str) -> Result<(), String> {
    let out = cmd
        .output()
        .map_err(|e| format!("{name}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(format!("{name} failed:\n{err}{stdout}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uno_uses_avr_pack() {
        let b = crate::board::find_board("uno").unwrap();
        assert_eq!(pack_id_for_board(b), Some("avr"));
        assert!(find_pack("avr").unwrap().ships_in_installer);
    }

    #[test]
    fn pico_uses_arm_pack() {
        let b = crate::board::find_board("pico").unwrap();
        assert_eq!(pack_id_for_board(b), Some("arm"));
        assert!(!find_pack("arm").unwrap().ships_in_installer);
    }

    #[test]
    fn esp8000_uses_esp8266_pack() {
        let b = crate::board::find_board("esp8000").unwrap();
        assert_eq!(pack_id_for_board(b), Some("esp8266"));
        let pack = find_pack("esp8266").unwrap();
        assert!(!pack.fetches.is_empty());
        assert!(pack.fetches.iter().all(|f| f.url.contains("3.2.0-gcc10.3")));
        assert!(pack
            .fetches
            .iter()
            .all(|f| f.url.contains("xtensa-lx106-elf-c791b74.230224")));
    }

    #[test]
    fn host_has_no_pack() {
        let b = crate::board::find_board("host").unwrap();
        assert_eq!(pack_id_for_board(b), None);
    }

    #[test]
    fn autocorrects_pack_typos() {
        assert_eq!(resolve_pack("esp826").unwrap().id, "esp8266");
        assert_eq!(resolve_pack("esp8266").unwrap().id, "esp8266");
        assert!(resolve_pack("esp").is_err());
        let st = status_for_board("unoo");
        assert_eq!(st.pack, "avr");
    }

    #[test]
    fn finds_compiler_in_fake_tree() {
        let dir = std::env::temp_dir().join(format!(
            "volt-tools-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let bin = dir.join("avr/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let gcc = bin.join("avr-gcc");
        std::fs::write(&gcc, b"").unwrap();
        let found = find_named(&dir, &["avr-gcc"]).unwrap();
        assert_eq!(found, gcc);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn esp8000_firmware_links_when_compiler_present() {
        let board = crate::board::find_board("esp8000").unwrap();
        if find_cc(board).is_none() {
            return;
        }
        let result = crate::driver::compile_source(
            "blink.volt",
            include_str!("../examples/blink.volt"),
            &crate::driver::CompileOptions {
                target: board.target.clone(),
                std_path: PathBuf::from("std"),
                include_paths: Vec::new(),
            },
        )
        .expect("compile blink for esp8000");
        let dir = std::env::temp_dir().join(format!(
            "volt-esp8000-fw-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let fw = build_firmware(&result.c_source, board, &dir).expect("link esp8000 firmware");
        assert!(fw.is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn host_flash_runs_the_binary() {
        let board = crate::board::find_board("host").unwrap();
        let result = crate::driver::compile_source(
            "add.volt",
            include_str!("../examples/add.volt"),
            &crate::driver::CompileOptions {
                target: board.target,
                std_path: PathBuf::from("std"),
                include_paths: Vec::new(),
            },
        )
        .expect("compile add");
        let dir = std::env::temp_dir().join(format!(
            "volt-host-fw-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let fw = build_firmware(&result.c_source, board, &dir).expect("link host firmware");
        let log = flash_firmware(&fw, board, "").expect("run host firmware");
        assert!(
            log.contains("exit 42"),
            "expected exit 42 from add.volt, got: {log}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn esp_flash_without_a_port_says_so() {
        let board = crate::board::find_board("esp8000").unwrap();
        let err = flash_firmware(Path::new("firmware.elf"), board, "").unwrap_err();
        assert!(err.contains("serial port"), "{err}");
    }
}
