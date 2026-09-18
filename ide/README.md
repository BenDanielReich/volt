# Volt IDE

Volt ships a native desktop app on macOS and Windows, plus a browser UI
and a language server.

## Native apps

```bash
# macOS — builds dist/Volt.app (double-click in Finder)
./scripts/package-macos.sh
open dist/Volt.app

# Windows — builds dist\Volt\Volt.exe
.\scripts\package-windows.cmd
dist\Volt\Volt.exe
```

Dev run without packaging:

```bash
cargo run --features app --bin volt
```

The window is WKWebView (macOS) or WebView2 (Windows 11 / Edge runtime).
`std/` and `examples/` ship next to the binary (inside `Contents/Resources`
on macOS). AVR gcc ships in the **installer** (`./scripts/make-installer.sh` → `dist/Volt.pkg`). Pico/ESP: **Get compiler** in the IDE.

## Web IDE (`voltc ide`)

A local Monaco editor with live diagnostics, completions, hover, generated C, and the resource report.

```bash
cargo run -- ide
# opens http://127.0.0.1:8741
cargo run -- ide --port 9000 --no-open
```

Compile with **Ctrl+Enter** (⌘↵ on macOS). **Ctrl+S** / **⌘S** saves into `Documents/Volt/projects`. Extra libraries go in `Documents/Volt/addons`. The left sidebar has three collapsible groups: **Libraries**, **Examples**, and **Files**. The header has a **board search** (name, FQBN, MCU, or alias such as `nano` / `esp8000`) and a serial-port list (`/dev/cu.usbmodem*` on a Mac).

### macOS

```bash
cargo run -- ide
target/debug/voltc ide
open ide/macos/volt-ide.command
```

`voltc ide` binds `127.0.0.1:8741` and runs `open <url>` (Safari if that fails). `voltc ports` lists `/dev/cu.usbmodem*` and `/dev/cu.usbserial*`, skipping Bluetooth. Pico appears as `/Volumes/RPI-RP2` after BOOTSEL.

### Windows

```bat
cargo run -- ide
target\debug\voltc.exe ide
ide\windows\volt-ide.cmd
```

PowerShell:

```powershell
.\ide\windows\volt-ide.ps1
```

`voltc ide` binds `127.0.0.1:8741` and opens the default browser via `cmd /C start "" <url>` (the empty title is required so `start` does not swallow the URL). Pass `--no-open` and visit the printed URL if a policy blocks the browser.

Set `VOLT_STD` if the standard library is not next to the binary or in the repo `std\` folder.

## Language server (`voltc lsp`)

JSON-RPC over stdio. Point any LSP client at `voltc lsp` (on Windows: `voltc.exe lsp`).

Stdio is switched to binary mode on Windows so `Content-Length` is not inflated by `\n` → `\r\n`. `file:///C:/...` URIs are decoded before diagnostics.

## Cursor / VS Code extension

Folder: `ide/vscode/`

```bash
# Cursor: Extensions → Install from Location → ide/vscode
# or: ln -s "$(pwd)/ide/vscode" ~/.cursor/extensions/volt
```

On Windows, install from that folder the same way, or copy it to `%USERPROFILE%\.cursor\extensions\volt`. On macOS the same Install from Location path works; the binary is `voltc`, not `voltc.exe`.

The extension highlights `.volt` files, runs `voltc check` on save, and offers **Volt: Open IDE**.
