//! Shared IDE / LSP analysis over the Volt compiler.

use std::path::PathBuf;

use crate::diagnostic::Level;
use crate::driver::{self, CompileError, CompileOptions, CompileResult};
use crate::lexer::Lexer;
use crate::source::SourceMap;
use crate::target::{self, Target};
use crate::token::TokenKind;

#[derive(Debug, Clone, serde::Serialize)]
pub struct IdeDiagnostic {
    pub level: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Completion {
    pub label: String,
    pub kind: String,
    pub detail: String,
    pub insert: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Hover {
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalyzeResult {
    pub ok: bool,
    pub diagnostics: Vec<IdeDiagnostic>,
    pub c_source: Option<String>,
    pub report: Option<String>,
}

pub fn options_for(target_name: &str, std_path: Option<PathBuf>) -> Result<CompileOptions, String> {
    let target = target::find_target(target_name)
        .cloned()
        .ok_or_else(|| crate::board::unknown_board_message(target_name))?;
    Ok(CompileOptions {
        target,
        std_path: std_path.unwrap_or_else(driver::default_std_path),
        include_paths: crate::workspace::addon_search_paths(),
    })
}

pub fn analyze(name: &str, src: &str, opts: &CompileOptions) -> AnalyzeResult {
    match driver::compile_source(name, src, opts) {
        Ok(result) => AnalyzeResult {
            ok: true,
            diagnostics: convert_diags(&result.diagnostics, &result.sources),
            c_source: Some(result.c_source),
            report: Some(result.report),
        },
        Err(err) => AnalyzeResult {
            ok: false,
            diagnostics: convert_diags(&err.diagnostics, &err.sources),
            c_source: None,
            report: None,
        },
    }
}

pub fn analyze_file(path: &std::path::Path, opts: &CompileOptions) -> AnalyzeResult {
    match driver::compile_file(path, opts) {
        Ok(result) => from_ok(result),
        Err(err) => from_err(err),
    }
}

fn from_ok(result: CompileResult) -> AnalyzeResult {
    AnalyzeResult {
        ok: true,
        diagnostics: convert_diags(&result.diagnostics, &result.sources),
        c_source: Some(result.c_source),
        report: Some(result.report),
    }
}

fn from_err(err: CompileError) -> AnalyzeResult {
    AnalyzeResult {
        ok: false,
        diagnostics: convert_diags(&err.diagnostics, &err.sources),
        c_source: None,
        report: None,
    }
}

pub fn convert_diags(
    diags: &[crate::diagnostic::Diagnostic],
    sources: &SourceMap,
) -> Vec<IdeDiagnostic> {
    diags
        .iter()
        .map(|d| {
            let level = match d.level {
                Level::Error => "error",
                Level::Warning => "warning",
                Level::Note => "note",
            };
            let file = if d.span.is_dummy() || (d.span.file as usize) >= sources.files().len() {
                String::new()
            } else {
                sources.get(d.span.file).name.clone()
            };
            let ((line, col), (end_line, end_col)) = if file.is_empty() {
                ((1, 1), (1, 1))
            } else {
                sources.span_range(d.span)
            };
            IdeDiagnostic {
                level: level.into(),
                message: d.message.clone(),
                file,
                line,
                col,
                end_line,
                end_col,
            }
        })
        .collect()
}

pub fn completions(src: &str, offset: usize) -> Vec<Completion> {
    let prefix = ident_prefix(src, offset);
    let mut out = builtin_completions();
    let (tokens, _) = Lexer::new(0, src).tokenize();
    let mut seen = std::collections::HashSet::new();
    for tok in tokens {
        if tok.kind == TokenKind::Ident && seen.insert(tok.lexeme.clone()) {
            out.push(Completion {
                label: tok.lexeme.clone(),
                kind: "identifier".into(),
                detail: "identifier in file".into(),
                insert: tok.lexeme,
            });
        }
    }
    if prefix.is_empty() {
        return out;
    }
    out.retain(|c| {
        c.label.starts_with(&prefix) || c.insert.starts_with(&prefix)
    });
    out
}

pub fn hover(src: &str, offset: usize, target: Target) -> Option<Hover> {
    let word = ident_at(src, offset)?;
    if let Some(detail) = keyword_doc(&word) {
        return Some(Hover {
            title: word,
            detail: detail.into(),
        });
    }
    let opts = CompileOptions {
        target,
        std_path: driver::default_std_path(),
        include_paths: Vec::new(),
    };
    if let Ok(result) = driver::compile_source("hover.volt", src, &opts) {
        if let Some(f) = result.program.funcs.iter().find(|f| f.name == word) {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|(n, ty)| format!("{} {}", ty.name(), n))
                .collect();
            return Some(Hover {
                title: format!("{}({})", f.c_name, params.join(", ")),
                detail: format!("returns {}", f.return_ty.name()),
            });
        }
        if let Some(r) = result.program.regs.iter().find(|r| r.name == word) {
            return Some(Hover {
                title: format!("reg {} @ 0x{:X}", r.name, r.address),
                detail: r.ty.name(),
            });
        }
        if result.program.structs.iter().any(|s| s.name == word) {
            return Some(Hover {
                title: format!("struct {word}"),
                detail: "a bundle of fields that stay together".into(),
            });
        }
        if result.program.enums.iter().any(|e| e.name == word) {
            return Some(Hover {
                title: format!("enum {word}"),
                detail: "PinMode.Output means the pin sends a signal".into(),
            });
        }
    }
    Some(Hover {
        title: word,
        detail: "identifier".into(),
    })
}

pub fn builtin_completions() -> Vec<Completion> {
    let mut items: Vec<Completion> = KEYWORDS
        .iter()
        .map(|(label, detail)| Completion {
            label: (*label).into(),
            kind: "keyword".into(),
            detail: (*detail).into(),
            insert: (*label).into(),
        })
        .collect();
    items.extend([
        Completion {
            label: "setup/loop".into(),
            kind: "snippet".into(),
            detail: "Arduino-style setup + loop".into(),
            insert: "function setup() {\n    \n}\n\nfunction loop() {\n    \n}\n".into(),
        },
        Completion {
            label: "reg".into(),
            kind: "snippet".into(),
            detail: "a chip register".into(),
            insert: "reg u8 NAME @ 0x00 {\n    bit0: 0,\n}\n".into(),
        },
        Completion {
            label: "interrupt".into(),
            kind: "snippet".into(),
            detail: "run this when the chip interrupts".into(),
            insert: "#[interrupt(\"TIMER0_OVF\")]\nfunction on_tick() {\n    \n}\n".into(),
        },
        Completion {
            label: "match".into(),
            kind: "snippet".into(),
            detail: "pick a branch".into(),
            insert: "match (x) {\n    _ => { }\n}\n".into(),
        },
        Completion {
            label: "pinmode".into(),
            kind: "function".into(),
            detail: "set a pin as input or output".into(),
            insert: "pinmode(LED_BUILTIN, PinMode.Output)".into(),
        },
        Completion {
            label: "#change".into(),
            kind: "snippet".into(),
            detail: "make a word shorter".into(),
            insert: "#change enum to em\n".into(),
        },
    ]);
    items
}

const KEYWORDS: &[(&str, &str)] = &[
    ("module", "name of this file"),
    ("use", "pull in a Volt library"),
    ("struct", "a bundle of fields that stay together"),
    ("enum", "a list of names, like PinMode.Input / PinMode.Output"),
    ("const", "a name for a value that never changes"),
    ("static", "a variable that lives for the whole program"),
    ("reg", "a hardware register on the chip"),
    ("extern", "this function is provided by the board / C"),
    ("if", "do this only when the test is true"),
    ("else", "the other branch of an if"),
    ("while", "keep looping while this is true"),
    ("for", "loop with a counter"),
    ("return", "leave this function, maybe with a value"),
    ("break", "stop the loop"),
    ("continue", "skip to the next time around the loop"),
    ("as", "treat this number as a different size"),
    ("true", "yes / on"),
    ("false", "no / off"),
    ("function", "a chunk of code (returns nothing)"),
    ("match", "pick a branch"),
    ("impl", "add functions onto a type"),
    ("trait", "a checklist of functions a type must have"),
    ("type", "a nickname for a type"),
    ("export", "visible to the rest of the program"),
    ("internal", "only this file can use it"),
    ("comptime", "runs while compiling, not on the chip"),
    ("async", "a function that can pause and come back"),
    ("await", "pause here until that finishes"),
    ("when", "if, decided while compiling (usually the board)"),
    ("asm", "raw assembly for this chip"),
    ("mut", "allowed to change"),
    ("self", "this object, inside a function on a type"),
    ("pin", "a name for one bit on a port"),
    ("peripheral", "a piece of hardware you hand off once"),
    ("void", "same as function — returns nothing"),
    ("int", "a plain integer"),
    ("bool", "true or false"),
    ("u8", "a number 0–255"),
    ("u16", "a number 0–65535"),
    ("u32", "a 32-bit unsigned number"),
    ("u64", "a 64-bit unsigned number"),
    ("i8", "a signed 8-bit number"),
    ("i16", "a signed 16-bit number"),
    ("i32", "a signed 32-bit number"),
    ("i64", "a signed 64-bit number"),
    ("usize", "an unsigned number the size of a pointer"),
    ("isize", "a signed number the size of a pointer"),
    ("f32", "a 32-bit float"),
    ("f64", "a 64-bit float"),
];

fn keyword_doc(word: &str) -> Option<&'static str> {
    KEYWORDS
        .iter()
        .chain(EXTRA_HOVER.iter())
        .find(|(k, _)| *k == word)
        .map(|(_, d)| *d)
}

const EXTRA_HOVER: &[(&str, &str)] = &[
    (
        "pinmode",
        "set a pin as input or output: pinmode(LED_BUILTIN, PinMode.Output)",
    ),
    (
        "PinMode",
        "Input, Output, or InputPullup — pass this to pinmode",
    ),
];

fn ident_prefix(src: &str, offset: usize) -> String {
    let bytes = src.as_bytes();
    let mut i = offset.min(bytes.len());
    while i > 0 {
        let c = bytes[i - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            i -= 1;
        } else {
            break;
        }
    }
    src[i..offset.min(src.len())].to_string()
}

fn ident_at(src: &str, offset: usize) -> Option<String> {
    let bytes = src.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut i = offset.min(bytes.len());
    if i > 0 && !(bytes[i.saturating_sub(1)] as char).is_ascii_alphanumeric() && i < bytes.len() {
        // cursor at start of ident
    } else if i == bytes.len() || !(bytes.get(i).copied().unwrap_or(0) as char).is_ascii_alphanumeric()
    {
        i = i.saturating_sub(1);
    }
    let mut start = i.min(bytes.len().saturating_sub(1));
    let mut end = start;
    while start > 0 {
        let c = bytes[start - 1] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            start -= 1;
        } else {
            break;
        }
    }
    while end < bytes.len() {
        let c = bytes[end] as char;
        if c.is_ascii_alphanumeric() || c == '_' {
            end += 1;
        } else {
            break;
        }
    }
    if start >= end {
        return None;
    }
    Some(src[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::HOST;

    fn host_opts() -> CompileOptions {
        CompileOptions {
            target: HOST,
            std_path: PathBuf::from("std"),
            include_paths: Vec::new(),
        }
    }

    #[test]
    fn type_error_has_line() {
        let r = analyze("bad.volt", "void main() { u8 x = true }", &host_opts());
        assert!(!r.ok);
        assert!(r.diagnostics.iter().any(|d| d.line >= 1 && d.message.contains("expected `u8`")));
    }

    #[test]
    fn completions_include_function() {
        let c = completions("fun", 3);
        assert!(c.iter().any(|c| c.label == "function"));
    }

    #[test]
    fn hover_keyword() {
        let h = hover("function setup() {}", 0, HOST).unwrap();
        assert_eq!(h.title, "function");
    }

    #[test]
    fn hover_pinmode() {
        let src = "pinmode(LED_BUILTIN, PinMode.Output)";
        let h = hover(src, 2, HOST).unwrap();
        assert_eq!(h.title, "pinmode");
        assert!(h.detail.contains("PinMode"));
        let h = hover(src, src.find("PinMode").unwrap(), HOST).unwrap();
        assert_eq!(h.title, "PinMode");
    }

    #[test]
    fn unknown_board_is_a_compiler_error() {
        let err = options_for("unoo", None).unwrap_err();
        assert!(err.contains("unknown board `unoo`"));
        assert!(err.contains("did you mean:"));
        assert!(err.contains("uno"));
    }
}
