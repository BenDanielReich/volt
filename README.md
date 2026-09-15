# Volt

A C++-like language for microcontrollers. This repo is the **compiler framework**:
lexer → parser → type checker → C backend, plus a small standard library and
examples.

Volt is deliberately not a new C++. It keeps the parts that feel right on a
chip (functions, structs, enums, pointers, bitwise ops, `#define`, `#include`)
and drops the parts that hide cost (exceptions, implicit heap). Hardware
registers and interrupts are syntax. A function that returns nothing is
declared with `function` (Volt’s `void`).

```volt
module blink;

#include <volt/gpio>
#include <volt/delay>

#define LED 13
#define DELAY 500

function setup() {
    pin_mode(LED, PinMode.Output);
}

function loop() {
    digital_write(LED, Level.High);
    delay_ms(DELAY);
    digital_write(LED, Level.Low);
    delay_ms(DELAY);
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
| Lexer        | Keywords, numbers, operators, comments            |
| Parser       | C++-style decls, control flow, structs, enums     |
| Sema         | Names, types, integer const-eval                  |
| Codegen      | Readable C (`stdint.h`, volatile register macros) |
| Targets      | `host`, `avr-atmega328p`, `cortex-m0`             |

See [docs/language.md](docs/language.md) for the language and
[docs/compiler.md](docs/compiler.md) for how to extend the compiler.

## Build

Needs a Rust toolchain.

```bash
cargo build
cargo test
cargo run -- compile examples/add.volt
cargo run -- compile examples/blink.volt --target avr-atmega328p
```

Compile the `add` example all the way to a host binary:

```bash
cargo run -- compile examples/add.volt -o /tmp/add.c
cc /tmp/add.c -o /tmp/add
/tmp/add; echo $?    # 42
```

## Layout

```
src/           compiler stages (one module per stage)
std/volt/      standard library (loaded by `#include` / `use`)
runtime/       C helpers per target (delay, GPIO)
examples/      blink, branch, math, registers, ISR, host add
docs/          language spec + compiler map
```

## Adding something

**Language feature** — token (if needed) → AST → parse → check → emit C → test.

**MCU target** — a `Target` in `src/target.rs` plus C under `runtime/<name>/`.
