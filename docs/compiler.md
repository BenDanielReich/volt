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
  Preproc      `#define` expansion
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
| `preproc`     | `#define` object- and function-like macros          |
| `ast`         | Untyped syntax tree                                 |
| `parser`      | Recursive-descent, with backtrack for declarations  |
| `sema`        | Name resolution, type checking, const eval          |
| `codegen`     | Pretty-printed C                                    |
| `target`      | MCU / host descriptions                             |
| `driver`      | Glue: load → compile → emit                         |

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

`target.rs` is a data definition: pointer width, C includes, ISR flavor.
Codegen reads that. Runtime C for the target lives in `runtime/<name>/`.

Current targets:

- `host` — desktop `cc`, for tests and the REPL-ish `--run` path
- `avr-atmega328p` — Arduino Uno class (ISR macros, `avr/io.h`)
- `cortex-m0` — Thumb-2 bare metal (CMSIS-style stubs)

## Testing the pipeline

```
cargo test
cargo run -- compile examples/add.volt --emit ast
cargo run -- compile examples/add.volt -o /tmp/add.c --target host
```

`compile_source` in `lib.rs` is the stable entry point for tests: feed a
string, get C or diagnostics. The CLI is a thin wrapper around `driver`.
