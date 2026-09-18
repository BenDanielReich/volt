# Volt compiler framework

`voltc` is a multi-stage compiler. Each stage is a module with a narrow
interface so we can grow the language without rewriting the pipeline.

```
SourceMap
    │
    ▼
  Lexer        source text → Token[]
    │
    ▼
  Preproc      `#define` / `#change`
    │
    ▼
  Parser       Token[]     → ast::Module
    │
    ▼
  Loader       follows `use` / `#include`, parses imported files
    │
    ▼
  Sema         AST         → typed HIR-ish Program
    │
    ▼
  Codegen      Program     → C text
    │
    ▼
  Target       C text      → (optional) cc / avr-gcc / …
```

## Crates / modules

| Module        | Responsibility                                      |
|---------------|-----------------------------------------------------|
| `source`      | File table, line/column lookup                      |
| `span`        | Byte offsets into a source file                     |
| `diagnostic`  | Errors and notes, rendered with source snippets     |
| `token`       | Token kinds                                         |
| `lexer`       | Hand-written scanner                                |
| `preproc`     | `#define` macros and `#change` short names          |
| `ast`         | Untyped syntax tree                                 |
| `parser`      | Recursive-descent, with backtrack for declarations  |
| `sema`        | Name resolution, type checking, const eval          |
| `codegen`     | Pretty-printed C                                    |
| `target`      | MCU / host descriptions                             |
| `board`       | Out-of-box Arduino FQBNs, USB ids, runtimes         |
| `toolchain`   | AVR-in-installer + on-demand Pico/ESP downloads     |
| `driver`      | Glue: load → compile → emit                         |
| `report`      | Stack / ISR / peripheral resource report            |
| `svd`         | CMSIS-SVD XML → Volt `reg` source                   |

There is no LLVM (yet) and no custom assembler. Emitting C is the framework
choice that gets Volt onto real chips on day one: every MCU vendor already
ships a C toolchain.

## Adding a language feature

1. Token / keyword in `token.rs` + `lexer.rs` if needed.
2. AST node in `ast.rs`.
3. Parse it in `parser.rs`.
4. Check it in `sema.rs` (names, types, constness).
5. Emit it in `codegen.rs`.
6. A `.volt` file under `tests/` or `examples/`.

## Adding a target

Add a `Board` in `src/board.rs` (Arduino FQBN, USB VID/PID, `LED_BUILTIN`) and
runtime C under `runtime/<family>/`. `target.rs` is the chip description
(`pointer_width`, ISR style, `f_cpu`).

Current boards (`voltc boards`): Arduino AVR (UNO, Nano / Nano old bootloader, Mega, Leonardo, Micro, …), Arduino Nano ESP32, ESP8000 (ESP8266), Pico, Blue Pill, Nucleo-F401RE, ESP32 DevKit, and `host`.

## Testing the pipeline

```
cargo test
cargo run -- compile examples/add.volt --emit ast
cargo run -- compile examples/add.volt -o /tmp/add.c --target host
cargo run -- report examples/power.volt
cargo run -- svd examples/atmega328p.svd -o /tmp/portb.volt
cargo run -- boards
cargo run -- ports
cargo run -- ide
```

`compile_source` in `lib.rs` is the stable entry point for tests: feed a
string, get C or diagnostics. The CLI is a thin wrapper around `driver`.

The IDE (`voltc ide`) and language server (`voltc lsp`) are part of the
same binary. On Windows, the LSP sets stdin/stdout to binary mode; `voltc
ide` opens the browser with `cmd /C start "" <url>`.

The native desktop apps (`Volt.app` / `Volt.exe`) are the `volt` binary,
built with `--features app` (WKWebView on macOS, WebView2 on Windows).
`./scripts/package-macos.sh` and `.\scripts\package-windows.cmd` copy the
binary, `voltc`, and `std/` into `dist/`.
