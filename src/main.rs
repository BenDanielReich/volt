use std::path::{Path, PathBuf};
use std::process::ExitCode;

use voltc::diagnostic::Diagnostic;
use voltc::driver::{self, CompileOptions};
use voltc::target::{self, HOST};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|a| a == "-V" || a == "--version")
        || args.first().map(String::as_str) == Some("version")
    {
        println!("voltc {VERSION}");
        return ExitCode::SUCCESS;
    }

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

fn run(args: &[String]) -> Result<(), ExitCode> {
    let (cmd, rest) = match args.first().map(String::as_str) {
        Some("compile") | Some("check") | Some("tokens") | Some("ast") => {
            (args[0].as_str(), &args[1..])
        }
        Some(_) => ("compile", args),
        None => {
            print_help();
            return Ok(());
        }
    };

    let parsed = parse_args(rest).map_err(|e| {
        eprintln!("error: {e}");
        ExitCode::from(2)
    })?;

    let Some(input) = parsed.input.clone() else {
        eprintln!("error: missing input file");
        return Err(ExitCode::from(2));
    };
    let path = Path::new(&input);

    match cmd {
        "tokens" => dump_tokens(path),
        "ast" => dump_ast(path),
        "check" => check(path, &parsed),
        _ => compile(path, &parsed),
    }
}

struct Args {
    input: Option<String>,
    output: Option<PathBuf>,
    target: String,
    emit: String,
    include_paths: Vec<PathBuf>,
    std_path: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut out = Args {
        input: None,
        output: None,
        target: "host".into(),
        emit: "c".into(),
        include_paths: Vec::new(),
        std_path: None,
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-o" | "--output" => {
                i += 1;
                out.output = Some(PathBuf::from(args.get(i).ok_or("missing argument for -o")?));
            }
            "--target" => {
                i += 1;
                out.target = args.get(i).ok_or("missing argument for --target")?.clone();
            }
            "--emit" => {
                i += 1;
                out.emit = args.get(i).ok_or("missing argument for --emit")?.clone();
            }
            "-I" => {
                i += 1;
                out.include_paths
                    .push(PathBuf::from(args.get(i).ok_or("missing argument for -I")?));
            }
            "--std-path" => {
                i += 1;
                out.std_path = Some(PathBuf::from(
                    args.get(i).ok_or("missing argument for --std-path")?,
                ));
            }
            s if s.starts_with('-') => return Err(format!("unknown option `{s}`")),
            s => {
                if out.input.is_some() {
                    return Err(format!("unexpected argument `{s}`"));
                }
                out.input = Some(s.to_string());
            }
        }
        i += 1;
    }
    Ok(out)
}

fn options_from(args: &Args) -> Result<CompileOptions, ExitCode> {
    let target = target::find_target(&args.target).cloned().ok_or_else(|| {
        eprintln!(
            "error: unknown target `{}` (try host, avr-atmega328p, cortex-m0)",
            args.target
        );
        ExitCode::from(2)
    })?;
    Ok(CompileOptions {
        target,
        std_path: args
            .std_path
            .clone()
            .unwrap_or_else(driver::default_std_path),
        include_paths: args.include_paths.clone(),
    })
}

fn dump_tokens(path: &Path) -> Result<(), ExitCode> {
    match driver::lex_file(path) {
        Ok((tokens, _)) => {
            for t in tokens {
                println!("{:<12} {}", format!("{:?}", t.kind), t.lexeme);
            }
            Ok(())
        }
        Err(diags) => print_fail(&diags, None),
    }
}

fn dump_ast(path: &Path) -> Result<(), ExitCode> {
    match driver::parse_file(path) {
        Ok((module, _)) => {
            println!("{module:#?}");
            Ok(())
        }
        Err(diags) => print_fail(&diags, None),
    }
}

fn check(path: &Path, args: &Args) -> Result<(), ExitCode> {
    let opts = options_from(args)?;
    match driver::compile_file(path, &opts) {
        Ok(_) => {
            eprintln!("ok");
            Ok(())
        }
        Err(err) => print_compile_fail(&err),
    }
}

fn compile(path: &Path, args: &Args) -> Result<(), ExitCode> {
    if args.emit == "tokens" {
        return dump_tokens(path);
    }
    if args.emit == "ast" {
        return dump_ast(path);
    }
    let opts = options_from(args)?;
    match driver::compile_file(path, &opts) {
        Ok(result) => {
            if let Some(out) = &args.output {
                if let Err(e) = std::fs::write(out, &result.c_source) {
                    eprintln!("error: cannot write {}: {e}", out.display());
                    return Err(ExitCode::from(1));
                }
            } else {
                print!("{}", result.c_source);
            }
            Ok(())
        }
        Err(err) => print_compile_fail(&err),
    }
}

fn print_compile_fail(err: &voltc::CompileError) -> Result<(), ExitCode> {
    eprint!(
        "{}",
        driver::format_diagnostics(&err.diagnostics, &err.sources)
    );
    Err(ExitCode::from(1))
}

fn print_fail(diags: &[Diagnostic], path: Option<&Path>) -> Result<(), ExitCode> {
    if let Some(path) = path {
        if let Ok(src) = std::fs::read_to_string(path) {
            let mut sources = voltc::source::SourceMap::new();
            sources.add(path.to_string_lossy().to_string(), src);
            eprint!("{}", driver::format_diagnostics(diags, &sources));
            return Err(ExitCode::from(1));
        }
    }
    let sources = voltc::source::SourceMap::new();
    eprint!("{}", driver::format_diagnostics(diags, &sources));
    Err(ExitCode::from(1))
}

fn print_help() {
    println!(
        "\
voltc {VERSION} — Volt compiler (C++-like language for microcontrollers)

Usage:
  voltc compile <file.volt> [-o out.c] [--target host]
  voltc check   <file.volt>
  voltc ast     <file.volt>
  voltc tokens  <file.volt>

Options:
  -o, --output <path>     Write generated C to a file (default: stdout)
  --target <name>         host | avr-atmega328p | cortex-m0
  --emit <kind>           c | ast | tokens
  -I <dir>                Extra module search path
  --std-path <dir>        Standard library root (default: ./std)
  -h, --help              Show this help
  -V, --version           Show version

Pipeline:
  .volt  →  lexer  →  parser  →  sema  →  C  →  your MCU toolchain
"
    );
    let _ = HOST;
}
