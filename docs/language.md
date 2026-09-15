# Volt language (v0)

Volt is a C++-like language for microcontrollers. The goal is familiar syntax
with MCU-first defaults: fixed-width types, no hidden allocations, first-class
memory-mapped registers, and interrupts as part of the language.

This document is the v0 surface the compiler actually implements. Features
marked *later* are planned but not parsed yet.

## Design rules

1. **What you write is what runs.** No constructors, destructors, exceptions, or
   implicit heap use in v0.
2. **Widths are explicit when it matters.** Prefer `u8` / `i16` / `u32`. `int`
   exists and maps to C `int` (size follows the target, like C++).
3. **Hardware is visible.** Registers and ISRs are syntax, not only macros.
4. **C++ where it helps.** `#include` pulls in libraries (`<volt/gpio>` or a C
   header like `<avr/io.h>`). `#define` and `const` are both supported.
   Functions that return nothing are declared with `function`
   (`void` still works as a synonym).

## Compilation model

A `.volt` file is a module. `voltc` type-checks the module (and its `use`s /
`#include`s) and emits a single C translation unit. You then compile that C
with the toolchain you already have (`avr-gcc`, `arm-none-eabi-gcc`, host `cc`, …).

```
blink.volt ──► voltc ──► blink.c ──► avr-gcc ──► firmware.hex
```

## Types

| Volt       | C equivalent | Notes                    |
|------------|--------------|--------------------------|
| `function` | `void`       | function with no return  |
| `void`     | `void`       | synonym for `function`   |
| `int`      | `int`        | target C `int`           |
| `bool`     | `bool`       | `true` / `false`         |
| `u8`   | `uint8_t`    |                          |
| `u16`  | `uint16_t`   |                          |
| `u32`  | `uint32_t`   |                          |
| `u64`  | `uint64_t`   |                          |
| `i8`   | `int8_t`     |                          |
| `i16`  | `int16_t`    |                          |
| `i32`  | `int32_t`    |                          |
| `i64`  | `int64_t`    |                          |
| `usize`| `uintptr_t`  | pointer-sized unsigned   |
| `isize`| `intptr_t`   | pointer-sized signed     |
| `f32`  | `float`      | optional on the target   |
| `f64`  | `double`     | optional on the target   |
| `T*`   | `T*`         | raw pointer              |
| `T[N]` | `T[N]`       | `N` must be a constant   |

User types: `struct` and `enum`.

## Libraries (`#include` and `use`)

```volt
#include <volt/gpio>
#include <volt/delay>
#include <volt/math>
#include <avr/io.h>
#include "helpers.volt"

use volt.gpio;    // same as #include <volt/gpio>
```

- `#include <volt/gpio>` loads `std/volt/gpio.volt` (a Volt library).
- `#include "file.volt"` searches next to the current file, then `-I`, then `std`.
- If the path is not a `.volt` file, it is passed through to the generated C
  (`#include <avr/io.h>`, `#include <stdio.h>`, …).
- `use a.b.c` still works and means the same as `#include <a/b/c>`.

### `volt/math` (C++ `<cmath>`)

`#include <volt/math>` brings in the C++ math functions and `<math.h>`.
Doubles use the C++ names; `f32` variants use the C `f` suffix. Float
literals (`1.0`) coerce to `f32` or `f64` from context.

| Group | `f64` | `f32` |
|-------|-------|-------|
| Trig | `sin` `cos` `tan` `asin` `acos` `atan` `atan2` | `sinf` `cosf` … `atan2f` |
| Hyperbolic | `sinh` `cosh` `tanh` `asinh` `acosh` `atanh` | `sinhf` … |
| Exp / log | `exp` `exp2` `expm1` `log` `log10` `log2` `log1p` | `expf` … |
| Power | `pow` `sqrt` `cbrt` `hypot` `fma` | `powf` `sqrtf` … |
| Rounding | `ceil` `floor` `trunc` `round` `nearbyint` `rint` | `ceilf` … |
| Other | `fabs` `fmod` `remainder` `fmax` `fmin` `fdim` `copysign` `ldexp` `scalbn` | `fabsf` … |
| Integer | `abs(int)` | |

Constants: `PI`, `E`, `SQRT2` (`f64`).

```volt
#include <volt/math>

int main() {
    return sqrt(16.0) as int;   // 4
}
```

## Items

### Functions

C++-style declarations, except a function that returns nothing starts with
`function` (Volt’s `void`). A trailing semicolon is a prototype; a brace body
is a definition. `extern` means “provided by the runtime / another language”.

```volt
function setup() {
    pin_mode(LED, PinMode.Output);
}

int add(int a, int b) {
    return a + b;
}

extern function delay_ms(u32 ms);
```

`void setup()` is still accepted and means the same as `function setup()`.

If a module defines `setup` and `loop` but no `main`, the compiler emits
Arduino-style `main`:

```c
int main(void) { setup(); for (;;) loop(); }
```

### `#define`

C-style macros, expanded before parse. Object-like and function-like forms:

```volt
#define LED 13
#define DELAY 500
#define MAX(a, b) ((a) > (b) ? (a) : (b))

function loop() {
    delay_ms(DELAY);
    digital_write(LED, Level.High);
}
```

A function-like macro requires `(` immediately after the name (`#define MAX(a, b)`,
not `#define MAX (a, b)`).

### Globals, const, static

```volt
const u8 LED = 13;        // module-level, folded at compile time
static u32 ticks = 0;
u16 counter = 0;          // module-level global
```

Inside a function, `const` is a C++-style local that cannot be assigned after
initialization:

```volt
function loop() {
    const int wait = 500;
    delay_ms(wait as u32);
}
```

### Registers

Memory-mapped I/O is a first-class item. The name behaves as a volatile lvalue.

```volt
reg u8 PORTB @ 0x25;
reg u8 DDRB  @ 0x24;

function setup() {
    DDRB |= 1 << 5;
}
```

Emitted C:

```c
#define PORTB (*((volatile uint8_t *)(0x25u)))
```

### Structs and enums

```volt
struct Pulse {
    u16 on_ms;
    u16 off_ms;
}

enum Level {
    Low = 0,
    High = 1,
}

function example(Pulse* p) {
    p.on_ms = 100;
    digital_write(13, Level.High);
}
```

Field access uses `.` (and `->` for pointers, C++-style). Enum variants are
`EnumName.Variant`.

### Interrupts

```volt
#[interrupt("TIMER0_OVF")]
function on_tick() {
    ticks += 1;
}
```

On AVR targets this becomes an `ISR(TIMER0_OVF_vect)` thunk that calls `on_tick`.

## Statements

```volt
int x = 1;
const int limit = 10;
x = x + 1;
if (x > 0) {
    return x;
} else if (x == 0) {
    return 0;
} else {
    return -1;
}
while (x > 0) { x -= 1; }
for (u8 i = 0; i < 10; i += 1) { ... }
loop { ... }
return x;
break;
continue;
```

## Expressions

Standard C++ operators with the usual precedence: arithmetic (`+` `-` `*` `/`),
bitwise, logical, comparisons, `++`/`--`, `&` / `*`, calls, indexing, `.` / `->`.

Comparisons:

- `a < b` — `b` is bigger (`a` is less than `b`)
- `a > b` — `a` is bigger (`a` is greater than `b`)
- `==` `!=` `<=` `>=`

Casts are explicit and written postfix:

```volt
u16 wide = (x as u16) << 8;
```

Integer literals are untyped and coerce into the type required by context
(`1 << LED` is `u8` if `LED` is `u8`). Suffixes pin a type: `1u8`, `0xFFu16`.

Hex (`0xFF`), binary (`0b1010`), character (`'A'`), and string (`"hi"`)
literals are supported. String literals *later* become `u8*` / array types;
in v0 they are only used in attributes.

## What v0 deliberately omits

These are C++ features we are not taking — either because they hide cost or
because they need a later language pass:

- classes, inheritance, virtuals, templates
- constructors / destructors / RAII
- exceptions, RTTI
- `new` / `delete` / implicit heap
- overloading, namespaces (modules replace them)
- references (pointers only, for now)

Planned next: methods, `match`, const-eval richer than integer folding,
inline assembly, and a borrow-checked pointer tier.

## Example

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
