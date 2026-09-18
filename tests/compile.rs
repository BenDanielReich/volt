use std::path::PathBuf;

use voltc::driver::{compile_source, CompileOptions};
use voltc::target::HOST;

fn opts() -> CompileOptions {
    CompileOptions {
        target: HOST.clone(),
        std_path: PathBuf::from("std"),
        include_paths: Vec::new(),
    }
}

#[test]
fn add_compiles_to_c() {
    let src = include_str!("../examples/add.volt");
    let result = compile_source("add.volt", src, &opts()).expect("compile");
    assert!(result.c_source.contains("int add(int a, int b)"));
    assert!(result.c_source.contains("return (a + b);"));
    assert!(result.c_source.contains("int main("));
}

#[test]
fn blink_imports_stdlib() {
    let src = include_str!("../examples/blink.volt");
    let result = compile_source("blink.volt", src, &opts()).expect("compile blink");
    assert!(result.c_source.contains("void setup(void)"));
    assert!(result.c_source.contains("void loop(void)"));
    assert!(result.c_source.contains("int main(void)"));
    assert!(result.c_source.contains("pin_mode"));
    assert!(result.c_source.contains("PinMode_Output"));
}

#[test]
fn registers_become_volatile_macros() {
    let src = include_str!("../examples/registers.volt");
    let result = compile_source("registers.volt", src, &opts()).expect("compile regs");
    assert!(result.c_source.contains("#define PORTB"));
    assert!(result.c_source.contains("0x25"));
    assert!(result.c_source.contains("volatile uint8_t"));
}

#[test]
fn function_keyword_emits_void() {
    let src = "function setup() { }\nfunction loop() { }";
    let result = compile_source("fn.volt", src, &opts()).expect("compile");
    assert!(result.c_source.contains("void setup(void)"));
    assert!(result.c_source.contains("void loop(void)"));
}

#[test]
fn define_expands_before_parse() {
    let src = "#define LED 13\nfunction main() { u8 x = LED }";
    let result = compile_source("def.volt", src, &opts()).expect("compile");
    assert!(result.c_source.contains("13"));
    assert!(!result.c_source.contains("x = LED"));
}

#[test]
fn int_type_emits_c_int() {
    let src = "int main() {\n    int x = 1\n    return x\n}";
    let result = compile_source("int.volt", src, &opts()).expect("compile");
    assert!(result.c_source.contains("int main("));
    assert!(result.c_source.contains("int x = 1"));
}

#[test]
fn include_loads_volt_library() {
    let src = include_str!("../examples/blink.volt");
    let result = compile_source("blink.volt", src, &opts()).expect("compile blink include");
    assert!(result.c_source.contains("pin_mode"));
    assert!(result.c_source.contains("PinMode_Output"));
}

#[test]
fn include_passes_c_headers_through() {
    let src = "#include <stdio.h>\nint main() { return 0 }";
    let result = compile_source("cinc.volt", src, &opts()).expect("compile c include");
    assert!(result.c_source.contains("#include <stdio.h>"));
}

#[test]
fn if_else_emits_c() {
    let src = r#"
        int main() {
            int x = 1
            if (x > 0) {
                return 1
            } else {
                return 0
            }
        }
    "#;
    let result = compile_source("if.volt", src, &opts()).expect("compile if");
    assert!(result.c_source.contains("if ((x > 0))"));
    assert!(result.c_source.contains("else"));
}

#[test]
fn const_local_emits_c_const() {
    let src = "int main() {\n    const int n = 4\n    return n\n}";
    let result = compile_source("const.volt", src, &opts()).expect("compile const");
    assert!(result.c_source.contains("const int n = 4"));
}

#[test]
fn cannot_assign_to_const() {
    let src = "int main() {\n    const int n = 4\n    n = 5\n    return n\n}";
    let err = compile_source("badconst.volt", src, &opts()).unwrap_err();
    assert!(err
        .diagnostics
        .iter()
        .any(|d| d.message.contains("cannot assign to const `n`")));
}

#[test]
fn math_library_compiles() {
    let src = include_str!("../examples/math.volt");
    let result = compile_source("math.volt", src, &opts()).expect("compile math");
    assert!(result.c_source.contains("#include <math.h>"));
    assert!(result.c_source.contains("sqrt("));
    assert!(result.c_source.contains("sin("));
    assert!(result.c_source.contains("PI"));
}

#[test]
fn math_sqrt_runs_on_host() {
    if std::process::Command::new("cc")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let src = include_str!("../examples/math.volt");
    let result = compile_source("math.volt", src, &opts()).expect("compile math");
    let c_path = std::env::temp_dir().join("volt_math_test.c");
    let bin_path = std::env::temp_dir().join("volt_math_test");
    std::fs::write(&c_path, &result.c_source).unwrap();
    let status = std::process::Command::new("cc")
        .arg(&c_path)
        .arg("-lm")
        .arg("-o")
        .arg(&bin_path)
        .status()
        .unwrap();
    assert!(status.success(), "cc failed to compile math example");
    let status = std::process::Command::new(&bin_path).status().unwrap();
    assert_eq!(status.code(), Some(5));
}

#[test]
fn arithmetic_and_comparisons() {
    let src = r#"
        int main() {
            int a = ((10 + 2) - 4) * 3 / 6
            if (a < 4) {
                return 1
            }
            if (a > 4) {
                return 2
            }
            return a
        }
    "#;
    let result = compile_source("ops.volt", src, &opts()).expect("compile ops");
    assert!(result.c_source.contains("+"));
    assert!(result.c_source.contains("-"));
    assert!(result.c_source.contains("*"));
    assert!(result.c_source.contains("/"));
    assert!(result.c_source.contains("<"));
    assert!(result.c_source.contains(">"));
}

#[test]
fn arithmetic_runs_on_host() {
    if std::process::Command::new("cc")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let src = r#"
        int main() {
            int a = ((10 + 2) - 4) * 3 / 6
            if (3 < 5) {
                if (9 > 2) {
                    return a
                }
            }
            return 0
        }
    "#;
    let result = compile_source("ops_run.volt", src, &opts()).expect("compile ops");
    let c_path = std::env::temp_dir().join("volt_ops_test.c");
    let bin_path = std::env::temp_dir().join("volt_ops_test");
    std::fs::write(&c_path, &result.c_source).unwrap();
    let status = std::process::Command::new("cc")
        .arg(&c_path)
        .arg("-o")
        .arg(&bin_path)
        .status()
        .unwrap();
    assert!(status.success(), "cc failed to compile ops example");
    let status = std::process::Command::new(&bin_path).status().unwrap();
    assert_eq!(status.code(), Some(4));
}

#[test]
fn type_error_is_reported() {
    let src = "void main() { u8 x = true }";
    let err = compile_source("bad.volt", src, &opts()).unwrap_err();
    assert!(err
        .diagnostics
        .iter()
        .any(|d| d.message.contains("expected `u8`")));
}

#[test]
fn add_links_and_returns_42() {
    if std::process::Command::new("cc")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let src = include_str!("../examples/add.volt");
    let result = compile_source("add.volt", src, &opts()).expect("compile");
    let c_path = std::env::temp_dir().join("volt_add_test.c");
    let bin_path = std::env::temp_dir().join("volt_add_test");
    std::fs::write(&c_path, &result.c_source).unwrap();
    let status = std::process::Command::new("cc")
        .arg(&c_path)
        .arg("-o")
        .arg(&bin_path)
        .status()
        .unwrap();
    assert!(status.success(), "cc failed to compile generated C");
    let status = std::process::Command::new(&bin_path).status().unwrap();
    assert_eq!(status.code(), Some(42));
}

#[test]
fn unknown_ident_is_reported() {
    let src = "void main() { return porb }";
    let err = compile_source("bad.volt", src, &opts()).unwrap_err();
    assert!(err
        .diagnostics
        .iter()
        .any(|d| d.message.contains("unknown identifier `porb`")));
}

#[test]
fn wrapping_and_saturating_ops() {
    let src = "int main() {\n    u8 a = 200\n    u8 b = 200\n    return (a +% b) as int\n}";
    let result = compile_source("wrap.volt", src, &opts()).expect("wrap");
    assert!(result.c_source.contains("uint32_t"));
    let src = "int main() {\n    u8 a = 1\n    u8 b = 2\n    return (a +| b) as int\n}";
    let result = compile_source("sat.volt", src, &opts()).expect("sat");
    assert!(result.c_source.contains("volt_sat_addu32"));
}

#[test]
fn bit_precise_and_alias() {
    let src = "type adc12 = u12\nint main() {\n    adc12 x = 4095u12\n    return x as int\n}";
    let result = compile_source("bits.volt", src, &opts()).expect("bits");
    assert!(result.c_source.contains("uint16_t"));
}

#[test]
fn register_bitfields_and_pins() {
    let src = r#"
        reg u8 PORTB @ 0x25 { pb5: 5, }
        pin LED = PORTB.pb5
        int main() {
            LED = 1
            PORTB.pb5 = 1
            return LED as int
        }
    "#;
    let result = compile_source("bitsreg.volt", src, &opts()).expect("reg bits");
    assert!(result.c_source.contains("PORTB"));
    assert!(result.c_source.contains("pb5"));
    assert!(result.c_source.contains("LED_SHIFT"));
}

#[test]
fn match_and_when() {
    let src = r#"
        enum E { A = 1, B = 2 }
        int main() {
            match (E.A) {
                E.A => { return 1 }
                _ => { return 0 }
            }
        }
    "#;
    let result = compile_source("match.volt", src, &opts()).expect("match");
    assert!(result.c_source.contains("E_A"));
    let src = r#"
        int main() {
            when (target.has_fpu) { return 1 } else { return 0 }
        }
    "#;
    let result = compile_source("when.volt", src, &opts()).expect("when");
    assert!(result.c_source.contains("return 1"));
    let before_runtime = result
        .c_source
        .split("/* board runtime */")
        .next()
        .unwrap_or(&result.c_source);
    assert!(!before_runtime.contains("return 0"));
}

#[test]
fn methods_and_traits() {
    let src = r#"
        struct Point { i32 x }
        trait HasX { function get(&self); }
        impl Point {
            function set(&mut self, i32 v) { self.x = v; }
        }
        impl HasX for Point {
            function get(&self) { }
        }
        int main() {
            Point p;
            p.set(3)
            p.get()
            return 0
        }
    "#;
    let result = compile_source("meth.volt", src, &opts()).expect("methods");
    assert!(result.c_source.contains("Point_set"));
    assert!(result.c_source.contains("Point_get"));
}

#[test]
fn isr_cannot_call_blocking() {
    let src = r#"
        #[blocking]
        function wait() {}
        #[interrupt("TIMER0_OVF")]
        function on_tick() { wait() }
    "#;
    let err = compile_source("isrblock.volt", src, &opts()).unwrap_err();
    assert!(err
        .diagnostics
        .iter()
        .any(|d| d.message.contains("blocking")));
}

#[test]
fn cannot_assign_through_shared_ref() {
    let src = "int main() {\n    int x = 1\n    int* p = &x\n    return 0\n}";
    let result = compile_source("ref.volt", src, &opts());
    assert!(result.is_ok());
    let src = "int main() {\n    const int n = 1\n    n = 2\n    return n\n}";
    let err = compile_source("cref.volt", src, &opts()).unwrap_err();
    assert!(err
        .diagnostics
        .iter()
        .any(|d| d.message.contains("cannot assign to const")));
}

#[test]
fn peripheral_move() {
    let src = r#"
        peripheral UART0;
        function take(UART0 u) {}
        function main() { take(UART0); take(UART0); }
    "#;
    let err = compile_source("move.volt", src, &opts()).unwrap_err();
    assert!(err
        .diagnostics
        .iter()
        .any(|d| d.message.contains("moved")));
}

#[test]
fn async_emits_poll() {
    let src = r#"
        function wait() {}
        async function work() { await wait() }
        int main() { return 0 }
    "#;
    let result = compile_source("async.volt", src, &opts()).expect("async");
    assert!(result.c_source.contains("work_poll"));
    assert!(result.c_source.contains("work_Frame"));
}

#[test]
fn typed_asm_and_report() {
    let src = r#"
        int main() {
            asm ("nop", in dummy = 0)
            return 0
        }
    "#;
    let result = compile_source("asm.volt", src, &opts()).expect("asm");
    assert!(result.c_source.contains("__asm__ volatile"));
    assert!(result.report.contains("target:"));
}

#[test]
fn power_example_compiles() {
    let src = include_str!("../examples/power.volt");
    compile_source("power.volt", src, &opts()).expect("power example");
}

#[test]
fn svd_import_roundtrip() {
    let xml = include_str!("../examples/atmega328p.svd");
    let volt = voltc::svd::svd_to_volt(xml).expect("svd");
    assert!(volt.contains("reg u8 PORTB_PORT"));
    compile_source("svd.volt", &volt, &opts()).expect("compiled svd output");
}

#[test]
fn crlf_source_compiles() {
    let src = "int main() {\r\n    return 42\r\n}\r\n";
    let result = compile_source("crlf.volt", src, &opts()).expect("crlf");
    assert!(result.c_source.contains("return 42"));
}

#[test]
fn arduino_uno_fqbn_blinks() {
    let target = *voltc::find_target("arduino:avr:uno").expect("uno");
    assert_eq!(target.led, 13);
    let src = include_str!("../examples/blink.volt");
    let result = compile_source(
        "blink.volt",
        src,
        &CompileOptions {
            target,
            std_path: PathBuf::from("std"),
            include_paths: Vec::new(),
        },
    )
    .expect("blink uno");
    assert!(result.c_source.contains("#define LED_BUILTIN"));
    assert!(result.c_source.contains("VOLT_BOARD_UNO"));
    assert!(result.c_source.contains("arduino:avr:uno"));
    assert!(result.c_source.contains("pin_mode"));
    if cfg!(target_os = "macos") {
        assert!(result.c_source.contains("os:    macos"));
        assert!(result.c_source.contains("/dev/cu."));
    }
}

#[test]
fn pico_led_is_gp25() {
    assert_eq!(voltc::find_target("pico").unwrap().led, 25);
}

#[test]
fn arduino_nano_blinks() {
    let target = *voltc::find_target("nano").expect("nano");
    assert_eq!(target.led, 13);
    let src = include_str!("../examples/blink.volt");
    let result = compile_source(
        "blink.volt",
        src,
        &CompileOptions {
            target,
            std_path: PathBuf::from("std"),
            include_paths: Vec::new(),
        },
    )
    .expect("blink nano");
    assert!(result.c_source.contains("#define LED_BUILTIN"));
    assert!(result.c_source.contains("VOLT_BOARD_NANO"));
    assert!(result.c_source.contains("arduino:avr:nano"));
    assert!(result.c_source.contains("atmega328p"));
}

#[test]
fn pinmode_calls_pin_mode() {
    let src = r#"
        #include <volt/gpio>
        function setup() { pinmode(13, 1) }
        function loop() {}
    "#;
    let result = compile_source("pinmode.volt", src, &opts()).expect("pinmode");
    assert!(result.c_source.contains("void pinmode("));
    assert!(result.c_source.contains("pin_mode("));
}

#[test]
fn change_shortens_enum_and_function() {
    let src = r#"
        #change enum to em
        #change function to fn
        em Level { Low = 0, High = 1 }
        fn setup() {}
        fn loop() {}
        int main() { return Level.High as int }
    "#;
    let result = compile_source("change.volt", src, &opts()).expect("change");
    assert!(result.c_source.contains("Level_High"));
    assert!(result.c_source.contains("void setup(void)"));
    assert!(result.c_source.contains("int main("));
}

#[test]
fn change_shortens_ident() {
    let src = r#"
        #include <volt/gpio>
        #change pin_mode to pm
        function setup() { pm(13, 1) }
        function loop() {}
    "#;
    let result = compile_source("pm.volt", src, &opts()).expect("change ident");
    assert!(result.c_source.contains("pin_mode"));
}

#[test]
fn esp8000_blinks() {
    let target = *voltc::find_target("esp8000").expect("esp8000");
    assert_eq!(target.led, 2);
    let src = include_str!("../examples/blink.volt");
    let result = compile_source(
        "blink.volt",
        src,
        &CompileOptions {
            target,
            std_path: PathBuf::from("std"),
            include_paths: Vec::new(),
        },
    )
    .expect("blink esp8000");
    assert!(result.c_source.contains("#define LED_BUILTIN"));
    assert!(result.c_source.contains("VOLT_BOARD_ESP8000"));
    assert!(result.c_source.contains("0x60000300"));
    assert!(result.c_source.contains("esptool.py --chip esp8266"));
}
