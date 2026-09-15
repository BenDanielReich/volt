//! Volt compiler framework.
//!
//! Pipeline: source → lexer → parser → sema → C codegen.
//! See `docs/compiler.md` for the stage map and how to extend it.

pub mod ast;
pub mod codegen;
pub mod diagnostic;
pub mod driver;
pub mod lexer;
pub mod parser;
pub mod preproc;
pub mod sema;
pub mod source;
pub mod span;
pub mod target;
pub mod token;

pub use driver::{compile_file, compile_source, CompileError, CompileOptions, CompileResult};
pub use target::{find_target, Target, HOST};

use std::path::PathBuf;

/// Compile a Volt source string to C for the host target, using `./std`.
pub fn compile_to_c(name: &str, src: &str) -> Result<String, Vec<diagnostic::Diagnostic>> {
    let opts = CompileOptions {
        target: HOST.clone(),
        std_path: PathBuf::from("std"),
        include_paths: Vec::new(),
    };
    compile_source(name, src, &opts)
        .map(|r| r.c_source)
        .map_err(|e| e.diagnostics)
}
