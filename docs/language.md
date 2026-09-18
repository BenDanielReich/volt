# Volt language (v0)

Volt is a small language for boards: Arduino Nano, UNO, ESP, Pico, and friends.
You write something that looks like Arduino C, `voltc` turns it into C, then
your usual chip tools (`avr-gcc`, …) burn it.

No semicolons. A function that returns nothing is written `function`.

```volt
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

`pinmode(LED_BUILTIN, PinMode.Output)` sets the LED pin as an output.
That's the first thing `setup` does. `PinMode.Input` means the pin reads.

## Words Volt uses

| Word | Means |
|------|--------|
| `function` | a chunk of code (returns nothing) |
| `int` / `u8` | numbers (`u8` is 0–255) |
| `pinmode` | set a pin as input or output (`#include <volt/board>`) |
| `PinMode` | `Input` or `Output` — pass this to `pinmode` |
| `struct` | a bundle of fields that stay together |
| `const` | a name for a value that never changes |
| `reg` | a hardware register on the chip |
| `#include` | pull in a library |
| `#change` | make a word shorter (`#change enum to em`) |

What you write is what runs. No hidden heap, no exceptions.

## Compilation model

A `.volt` file is one program (a module). `voltc` checks it and prints C.
You compile that C with the toolchain you already have.

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

User types: `struct` (a bundle of fields). `PinMode` is Input or Output for `pinmode`.

## Libraries (`#include` and `use`)

```volt
#include <volt/gpio>
#include <volt/delay>
#include <volt/math>
#include <avr/io.h>
#include "helpers.volt"

use volt.gpio    // same as #include <volt/gpio>
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
    return sqrt(16.0) as int   // 4
}
```

## Items

### Functions

C++-style declarations, except a function that returns nothing starts with
`function` (Volt’s `void`). A prototype has no body; a brace body is a
definition. `extern` means “provided by the runtime / another language”.

```volt
function setup() {
    pinmode(LED, PinMode.Output)
}

int add(int a, int b) {
    return a + b
}

extern function delay_ms(u32 ms)
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
    delay_ms(DELAY)
    digital_write(LED, Level.High)
}
```

A function-like macro requires `(` immediately after the name (`#define MAX(a, b)`,
not `#define MAX (a, b)`).

### `#change`

Pick a shorter name for a command or word. The compiler rewrites it before parse,
so `em` below is the same as `enum`:

```volt
#change enum to em
#change function to fn

em Level {
    Low = 0,
    High = 1,
}

fn setup() {
    pinmode(LED_BUILTIN, PinMode.Output)
}
```

`#change pin_mode to pm` works the same way for identifiers.

### Globals, const, static

```volt
const u8 LED = 13        // module-level, folded at compile time
static u32 ticks = 0
u16 counter = 0          // module-level global
```

Inside a function, `const` is a C++-style local that cannot be assigned after
initialization:

```volt
function loop() {
    const int wait = 500
    delay_ms(wait as u32)
}
```

### Registers

Memory-mapped I/O is a first-class item. The name behaves as a volatile lvalue.

```volt
reg u8 PORTB @ 0x25
reg u8 DDRB  @ 0x24

function setup() {
    DDRB |= 1 << 5
}
```

Emitted C:

```c
#define PORTB (*((volatile uint8_t *)(0x25u)))
```

### Bundles (`struct`) and pin modes

A **struct** is a handful of values that travel together (on-time and off-time).

`pinmode` takes a `PinMode`: `Output` means the pin sends a signal, `Input`
means it reads one.

```volt
struct Pulse {
    u16 on_ms
    u16 off_ms
}

function example(Pulse* p) {
    p.on_ms = 100
    pinmode(13, PinMode.Output)
}
```

Use `.` to pick a field (`p.on_ms`) or a pin mode (`PinMode.Output`). Pointers
can also use `->`.

### Interrupts

```volt
#[interrupt("TIMER0_OVF")]
function on_tick() {
    ticks += 1
}
```

On AVR targets this becomes an `ISR(TIMER0_OVF_vect)` thunk that calls `on_tick`.

## Statements

```volt
int x = 1
const int limit = 10
x = x + 1
if (x > 0) {
    return x
} else if (x == 0) {
    return 0
} else {
    return -1
}
while (x > 0) { x -= 1 }
for (u8 i = 0, i < 10, i += 1) { ... }
loop { ... }
return x
break
continue
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
u16 wide = (x as u16) << 8
```

Integer literals are untyped and coerce into the type required by context
(`1 << LED` is `u8` if `LED` is `u8`). Suffixes pin a type: `1u8`, `0xFFu16`,
`4095u12`.

Hex (`0xFF`), binary (`0b1010`), character (`'A'`), and string (`"hi"`)
literals are supported.

There is **no C integer promotion**: `u8 + u8` stays `u8`. Mixing widths
requires `as`. Overflow is defined via operators:

| Op | Meaning |
|----|---------|
| `+%` `-%` `*%` | wrapping |
| `+|` `-|` `*|` | saturating |

## Bit-precise integers and aliases

`u1`–`u64` and `i1`–`i64` (beyond the usual 8/16/32/64 names) are types.
Store width is the next power-of-two C integer; values are masked.

```volt
type adc12 = u12
adc12 sample = 4095u12
```

## Registers, bitfields, and pins

```volt
reg u8 PORTB @ 0x25 {
    pb5: 5,
    adps: 0..2,   // bits 0 inclusive through 2
}

pin LED = PORTB.pb5

function setup() {
    PORTB.pb5 = 1
    LED = 1
    u8 v = LED
}
```

`voltc svd chip.svd -o regs.volt` imports CMSIS-SVD peripherals into `reg` blocks.

## Methods, traits, match

```volt
struct Pulse { u16 on_ms }

trait Writer {
    function write(&mut self, u8 byte)
}

impl Pulse {
    function start(&mut self, u16 ms) { self.on_ms = ms }
}

impl Writer for Pulse {
    function write(&mut self, u8 byte) { self.on_ms = byte as u16 }
}

function demo(Pulse p, Phase ph) {
    p.start(10)
    p.write(3)
    match (ph) {
        Phase.Idle => { return }
        Phase.Run => { }
        _ => { }
    }
}
```

Calls `p.start(...)` lower to `Pulse_start(&p, ...)`.

## References, peripherals, effects

`&T` / `&mut T` are references (emitted as pointers). Assignment through `&T`
is an error. Two overlapping `&mut` borrows of the same local are rejected.

```volt
peripheral UART0          // unique token; passing it moves it
function take(UART0 u) {}
#[blocking] function wait()
#[interrupt("TIMER0_OVF")]
function on_tick() { wait() }   // error: blocking from an ISR
```

`internal function f()` emits `static` C. `export` is the default for
`main` / `setup` / `loop` interop.

## Target, when, comptime, async, asm

`target.has_fpu`, `target.led`, `target.avr`, `target.pointer_width`, `target.ram`, `target.f_cpu` are
compile-time values. `when` keeps only the taken branch:

```volt
when (target.has_fpu) {
    f32 x = 1.0
} else {
    int x = 1
}

async function blink_once() {
    LED = 1
    await wait()
    LED = 0
}

asm ("sbi {port}, {bit}", in port = 0x25, in bit = 5)
```

Async functions become a `_Frame` struct plus `_poll` that returns whether
the machine finished. `voltc report file.volt` prints stack estimates, ISRs,
and peripherals (also embedded as a comment in generated C).

## What Volt still omits

These C++ features we are not taking — they hide cost or need a later pass:

- classes, inheritance, virtuals, templates
- constructors / destructors / RAII
- exceptions, RTTI
- `new` / `delete` / implicit heap
- overloading, namespaces (modules replace them)

## Example

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
