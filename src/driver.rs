use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::codegen;
use crate::diagnostic::{self, Diagnostic};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::preproc;
use crate::sema::{Checker, Program};
use crate::source::SourceMap;
use crate::target::Target;

#[derive(Debug, Clone)]
pub struct CompileOptions {
    pub target: Target,
    pub std_path: PathBuf,
    pub include_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct CompileResult {
    pub c_source: String,
    pub report: String,
    pub program: Program,
    pub sources: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct CompileError {
    pub diagnostics: Vec<Diagnostic>,
    pub sources: SourceMap,
}

pub fn compile_file(path: &Path, opts: &CompileOptions) -> Result<CompileResult, CompileError> {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            return Err(CompileError {
                diagnostics: vec![Diagnostic::error(
                    format!("cannot read {}: {e}", path.display()),
                    crate::span::Span::dummy(),
                )],
                sources: SourceMap::new(),
            });
        }
    };
    compile_with_loader(path, &src, opts)
}

pub fn compile_source(
    name: &str,
    src: &str,
    opts: &CompileOptions,
) -> Result<CompileResult, CompileError> {
    compile_with_loader(Path::new(name), src, opts)
}

fn compile_with_loader(
    path: &Path,
    src: &str,
    opts: &CompileOptions,
) -> Result<CompileResult, CompileError> {
    let mut sources = SourceMap::new();
    let mut modules = Vec::new();
    let mut diagnostics = Vec::new();
    let mut loaded = HashSet::new();
    let mut c_includes = Vec::new();

    load_module(
        path,
        src,
        opts,
        &mut sources,
        &mut modules,
        &mut diagnostics,
        &mut loaded,
        &mut c_includes,
    );

    if diagnostics
        .iter()
        .any(|d| d.level == crate::diagnostic::Level::Error)
    {
        return Err(CompileError {
            diagnostics,
            sources,
        });
    }

    let mut checker = Checker::with_target(opts.target);
    let mut program = checker.check_modules(&modules);
    program.c_includes = dedup_c_includes(c_includes);
    diagnostics.extend(checker.diagnostics);

    if diagnostics
        .iter()
        .any(|d| d.level == crate::diagnostic::Level::Error)
    {
        return Err(CompileError {
            diagnostics,
            sources,
        });
    }

    let report = crate::report::render(&program, &opts.target);
    let c_source = codegen::emit_c(&program, &opts.target);
    Ok(CompileResult {
        c_source,
        report,
        program,
        sources,
        diagnostics,
    })
}

fn load_module(
    path: &Path,
    src: &str,
    opts: &CompileOptions,
    sources: &mut SourceMap,
    modules: &mut Vec<crate::ast::Module>,
    diagnostics: &mut Vec<Diagnostic>,
    loaded: &mut HashSet<String>,
    c_includes: &mut Vec<crate::sema::CInclude>,
) {
    let key = path.to_string_lossy().to_string();
    if !loaded.insert(key) {
        return;
    }

    let file = sources.add(path.to_string_lossy().to_string(), src.to_string());
    let (tokens, lex_diags) = tokenize(file, src);
    diagnostics.extend(lex_diags);
    let (module, parse_diags) = Parser::new(tokens).parse_module();
    diagnostics.extend(parse_diags);

    let mut volt_deps: Vec<PathBuf> = Vec::new();
    for item in &module.items {
        match item {
            crate::ast::Item::Use(u) => {
                let parts: Vec<String> = u.path.iter().map(|p| p.name.clone()).collect();
                match resolve_use(&parts, opts) {
                    Ok(dep) => volt_deps.push(dep),
                    Err(msg) => diagnostics.push(Diagnostic::error(msg, u.span)),
                }
            }
            crate::ast::Item::Include(inc) => {
                match resolve_include(&inc.path, inc.style, path, opts) {
                    Some(dep) => volt_deps.push(dep),
                    None if inc.path.ends_with(".volt") => {
                        diagnostics.push(Diagnostic::error(
                            format!("cannot find Volt library `{}`", inc.path),
                            inc.span,
                        ));
                    }
                    None => c_includes.push(crate::sema::CInclude {
                        path: inc.path.clone(),
                        angled: inc.style == crate::ast::IncludeStyle::Angle,
                    }),
                }
            }
            _ => {}
        }
    }

    modules.push(module);

    for dep in volt_deps {
        match std::fs::read_to_string(&dep) {
            Ok(dep_src) => {
                load_module(
                    &dep,
                    &dep_src,
                    opts,
                    sources,
                    modules,
                    diagnostics,
                    loaded,
                    c_includes,
                );
            }
            Err(e) => diagnostics.push(Diagnostic::error(
                format!("cannot read {}: {e}", dep.display()),
                crate::span::Span::dummy(),
            )),
        }
    }
}

fn resolve_use(parts: &[String], opts: &CompileOptions) -> Result<PathBuf, String> {
    let rel = parts.join("/") + ".volt";
    let mut search = vec![opts.std_path.clone()];
    search.extend(opts.include_paths.iter().cloned());
    for root in search {
        let candidate = root.join(&rel);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "cannot find module `{}` (looked for {rel} under {} and -I paths)",
        parts.join("."),
        opts.std_path.display()
    ))
}

fn resolve_include(
    spec: &str,
    style: crate::ast::IncludeStyle,
    current_file: &Path,
    opts: &CompileOptions,
) -> Option<PathBuf> {
    let mut dirs = Vec::new();
    match style {
        crate::ast::IncludeStyle::Quote => {
            if let Some(parent) = current_file.parent() {
                if !parent.as_os_str().is_empty() {
                    dirs.push(parent.to_path_buf());
                } else {
                    dirs.push(PathBuf::from("."));
                }
            }
            dirs.extend(opts.include_paths.iter().cloned());
            dirs.push(opts.std_path.clone());
        }
        crate::ast::IncludeStyle::Angle => {
            dirs.push(opts.std_path.clone());
            dirs.extend(opts.include_paths.iter().cloned());
        }
    }
    let mut names = vec![spec.to_string()];
    if !spec.ends_with(".volt") {
        names.push(format!("{spec}.volt"));
    }
    for dir in dirs {
        for name in &names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
            // addons/servo/servo.volt for `#include <servo>`
            if let Some(stem) = Path::new(name).file_stem() {
                let nested = dir.join(stem).join(format!("{}.volt", stem.to_string_lossy()));
                if nested.is_file() {
                    return Some(nested);
                }
            }
        }
    }
    None
}

fn dedup_c_includes(includes: Vec<crate::sema::CInclude>) -> Vec<crate::sema::CInclude> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for inc in includes {
        let key = (inc.angled, inc.path.clone());
        if seen.insert(key) {
            out.push(inc);
        }
    }
    out
}

fn tokenize(file: u32, src: &str) -> (Vec<crate::token::Token>, Vec<Diagnostic>) {
    let (tokens, mut diags) = Lexer::new(file, src).tokenize();
    let (tokens, pre_diags) = preproc::preprocess(src, tokens);
    diags.extend(pre_diags);
    (tokens, diags)
}

pub fn format_diagnostics(diags: &[Diagnostic], sources: &SourceMap) -> String {
    diagnostic::render_all(diags, sources)
}

pub fn default_std_path() -> PathBuf {
    if let Ok(p) = std::env::var("VOLT_STD") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let next_to_bin = dir.join("std");
            if next_to_bin.is_dir() {
                return next_to_bin;
            }
            let bundled = dir.join("../Resources/std");
            if bundled.is_dir() {
                return bundled;
            }
            // cargo run on Windows may be target\debug or target\<triple>\debug
            let mut walk = dir.to_path_buf();
            for _ in 0..8 {
                let Some(parent) = walk.parent() else {
                    break;
                };
                walk = parent.to_path_buf();
                let candidate = walk.join("std");
                if candidate.is_dir() && walk.join("Cargo.toml").is_file() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("std")
}

/// Parse only, for `--emit ast`.
pub fn parse_file(path: &Path) -> Result<(crate::ast::Module, SourceMap), Vec<Diagnostic>> {
    let src = std::fs::read_to_string(path).map_err(|e| {
        vec![Diagnostic::error(
            format!("cannot read {}: {e}", path.display()),
            crate::span::Span::dummy(),
        )]
    })?;
    let mut sources = SourceMap::new();
    let file = sources.add(path.to_string_lossy().to_string(), src.clone());
    let (tokens, lex_diags) = tokenize(file, &src);
    if !lex_diags.is_empty() {
        return Err(lex_diags);
    }
    let (module, parse_diags) = Parser::new(tokens).parse_module();
    if !parse_diags.is_empty() {
        return Err(parse_diags);
    }
    Ok((module, sources))
}

pub fn lex_file(path: &Path) -> Result<(Vec<crate::token::Token>, SourceMap), Vec<Diagnostic>> {
    let src = std::fs::read_to_string(path).map_err(|e| {
        vec![Diagnostic::error(
            format!("cannot read {}: {e}", path.display()),
            crate::span::Span::dummy(),
        )]
    })?;
    let mut sources = SourceMap::new();
    let file = sources.add(path.to_string_lossy().to_string(), src.clone());
    let (tokens, lex_diags) = Lexer::new(file, &src).tokenize();
    if !lex_diags.is_empty() {
        return Err(lex_diags);
    }
    Ok((tokens, sources))
}
