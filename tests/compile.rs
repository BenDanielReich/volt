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
    let src = "#define LED 13\nfunction main() { u8 x = LED; }";
    let result = compile_source("def.volt", src, &opts()).expect("compile");
    assert!(result.c_source.contains("13"));
    assert!(!result.c_source.contains("LED"));
}

#[test]
fn int_type_emits_c_int() {
    let src = "int main() { int x = 1; return x; }";
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
    let src = "#include <stdio.h>\nint main() { return 0; }";
    let result = compile_source("cinc.volt", src, &opts()).expect("compile c include");
    assert!(result.c_source.contains("#include <stdio.h>"));
}

#[test]
fn if_else_emits_c() {
    let src = r#"
        int main() {
            int x = 1;
            if (x > 0) {
                return 1;
            } else {
                return 0;
            }
        }
    "#;
    let result = compile_source("if.volt", src, &opts()).expect("compile if");
    assert!(result.c_source.contains("if ((x > 0))"));
    assert!(result.c_source.contains("else"));
}

#[test]
fn const_local_emits_c_const() {
    let src = "int main() { const int n = 4; return n; }";
    let result = compile_source("const.volt", src, &opts()).expect("compile const");
    assert!(result.c_source.contains("const int n = 4"));
}

#[test]
fn cannot_assign_to_const() {
    let src = "int main() { const int n = 4; n = 5; return n; }";
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
            int a = ((10 + 2) - 4) * 3 / 6;
            if (a < 4) {
                return 1;
            }
            if (a > 4) {
                return 2;
            }
            return a;
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
            int a = ((10 + 2) - 4) * 3 / 6;
            if (3 < 5) {
                if (9 > 2) {
                    return a;
                }
            }
            return 0;
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
    let src = "void main() { u8 x = true; }";
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
    let src = "void main() { return porb; }";
    let err = compile_source("bad.volt", src, &opts()).unwrap_err();
    assert!(err
        .diagnostics
        .iter()
        .any(|d| d.message.contains("unknown identifier `porb`")));
}
