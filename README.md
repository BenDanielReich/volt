# Volt

A C++-like language for boards. This repo is the compiler: it reads `.volt`,
checks it, and writes C you can flash with `avr-gcc` / `avrdude` / etc.

It keeps the easy parts (functions, numbered types, `#include`) and skips the
heavy C++ stuff (exceptions, hidden heap). A function that returns nothing is
`function`. No semicolons.

`pinmode(LED_BUILTIN, PinMode.Output)` sets the LED pin as an output.
Put `#change enum to em` at the top of a file if you want a shorter word.

```volt
module blink

#include <volt/board>

function setup() {
    pinmode(LED_BUILTIN, PinMode.Output)
}

function loop() {
    digital_write(LED_BUILTIN, Level.High)
    delay_ms(500)
    digital_write(LED_BUILTIN, Level.Low)
    delay_ms(500)
}
```

`voltc` type-checks that and emits C. You finish the build with the toolchain
you already have (`avr-gcc`, `arm-none-eabi-gcc`, host `cc`).

```
blink.volt ──► voltc ──► blink.c ──► avr-gcc ──► firmware.hex
```

## Status

v0 is a working pipeline, not a finished language.

| Stage        | What it does                                      |
|--------------|---------------------------------------------------|
| Lexer        | Keywords, numbers, wrapping/saturating ops        |
| Parser       | Decls, control flow, methods, match, regs, async  |
| Sema         | Types, effects, moves, comptime `when`/`target`   |
| Codegen      | Readable C, ISR thunks, async poll machines       |
| Report       | Stack estimate, ISRs, peripherals                 |
| SVD          | Import chip registers from CMSIS-SVD              |
| Targets      | Arduino AVR boards + Pico / STM32 / ESP32         |

See [docs/language.md](docs/language.md) for the language and
[docs/compiler.md](docs/compiler.md) for how to extend the compiler.

## Build

Needs a Rust toolchain.

```bash
cargo build
cargo test
cargo run -- compile examples/add.volt
cargo run -- compile examples/blink.volt --board uno
cargo run -- boards
cargo run -- compile examples/power.volt
cargo run -- report examples/power.volt
cargo run -- svd examples/atmega328p.svd
cargo run -- ide
```

The IDE is a local Monaco editor with live diagnostics, completions, hover, generated C, and the resource report. Native apps:

```bash
./scripts/package-macos.sh          # dist/Volt.app
# Windows (from a machine with Rust):
.\scripts\package-windows.cmd       # dist\Volt\Volt.exe
```

On first run Volt creates two folders under `Documents/Volt`:

- `projects/` — your programs (Files in the IDE sidebar)
- `addons/` — extra libraries (`#include <servo>` looks here; Libraries in the sidebar)

The IDE sidebar also lists Examples. Each group folds up.

`voltc home` prints those paths. `voltc ide` still opens the browser UI. `voltc lsp` is a stdio language server. See [ide/README.md](ide/README.md).

## macOS

Needs [Rust](https://rustup.rs/). Xcode CLT (`xcode-select --install`) gives `cc`. For AVR boards, Homebrew:

```bash
brew install avr-gcc avrdude
# optional: brew install arduino-cli
```

```bash
cargo build
cargo test
cargo run -- ide
cargo run -- ports
cargo run -- compile examples/blink.volt --board uno
```

The native app is `dist/Volt.app` after `./scripts/package-macos.sh` (or `cargo run --features app --bin volt`). `voltc ide` still opens the browser via `open`. Double-click `ide/macos/volt-ide.command` for the browser launcher.

Plug in an UNO/Nano/Mega and `voltc ports` should list `/dev/cu.usbmodem*` (native USB) or `/dev/cu.usbserial*` (FTDI/CH340). Flash with that port:

```bash
avrdude -c arduino -p atmega328p -b 115200 -P /dev/cu.usbmodem14101 -U flash:w:firmware.hex:i
```

Pico: hold BOOTSEL, plug in, then `cp firmware.uf2 /Volumes/RPI-RP2/`.

Cursor/VS Code: **Extensions → Install from Location** on `ide/vscode`, or symlink into `~/.cursor/extensions`. Set `volt.path` to `target/debug/voltc` if needed.

## Windows

Needs [Rust](https://rustup.rs/) (MSVC toolchain is the usual choice: `rustup default stable-msvc`).

```bat
cargo build
cargo test
cargo run -- compile examples\add.volt
cargo run -- ide
```

`voltc ide` opens `http://127.0.0.1:8741` in the default browser. The native app:

```bat
.\scripts\package-windows.cmd
dist\Volt\Volt.exe
```

That needs the [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/) (already on Windows 11). Browser launcher: `ide\windows\volt-ide.cmd` or `.\ide\windows\volt-ide.ps1`.

`voltc lsp` uses binary stdio so Content-Length is not corrupted by CRLF translation. Point Cursor/VS Code at `target\debug\voltc.exe` via the `volt.path` setting, or install `ide\vscode`.

Compile the `add` example all the way to a host binary:

```bash
cargo run -- compile examples/add.volt -o /tmp/add.c
cc /tmp/add.c -o /tmp/add
/tmp/add; echo $?    # 42
```

On Windows (Developer Command Prompt or any `cl`/`clang` on PATH):

```bat
cargo run -- compile examples\add.volt -o %TEMP%\add.c
cl %TEMP%\add.c /Fe:%TEMP%\add.exe
%TEMP%\add.exe
```

## Layout

```
src/           compiler stages (one module per stage)
std/volt/      standard library (loaded by `#include` / `use`)
runtime/       C helpers per target (delay, GPIO)
examples/      blink, branch, math, registers, ISR, host add, power, SVD
ide/           web IDE, native app packagers, Cursor/VS Code extension
scripts/       package-macos.sh / package-windows.cmd
docs/          language spec + compiler map
```

## Adding something

**Language feature** — token (if needed) → AST → parse → check → emit C → test.

**MCU target** — add a `Board` in `src/board.rs` (Arduino FQBN, USB VID/PID, pin map) and C under `runtime/<family>/`.

## Boards

Arduino IDE 2 installs the `arduino:avr` core on first launch. Volt ships those boards (from [ArduinoCore-avr `boards.txt`](https://github.com/arduino/ArduinoCore-avr/blob/master/boards.txt)) so an UNO/Nano/Mega/Leonardo works without Boards Manager:

```
voltc boards
voltc compile examples/blink.volt --board uno
voltc compile examples/blink.volt --board nano
voltc compile examples/blink.volt --board nano-old
voltc compile examples/blink.volt --board nano-esp32
voltc compile examples/blink.volt --board esp8000
voltc compile examples/blink.volt --board arduino:avr:nano
```

`#include <volt/board>` gives `LED_BUILTIN`, `A0`…, `pinmode`, and `delay_ms`. Generated C embeds the pin-map runtime and an `avrdude` / `arduino-cli` flash line. USB VID/PID matches Arduino IDE auto-detect (`0x2341:0x0043` is an UNO).
