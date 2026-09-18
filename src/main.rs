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
        Some("compile") | Some("check") | Some("tokens") | Some("ast") | Some("svd")
        | Some("report") | Some("ide") | Some("app") | Some("lsp") | Some("boards")
        | Some("ports") | Some("home") | Some("tools") | Some("flash") => {
            (args[0].as_str(), &args[1..])
        }
        Some(_) => ("compile", args),
        None => {
            print_help();
            return Ok(());
        }
    };

    if cmd == "svd" {
        return run_svd(rest);
    }
    if cmd == "ide" {
        return run_ide(rest);
    }
    if cmd == "app" {
        return run_app();
    }
    if cmd == "lsp" {
        return voltc::lsp::run().map_err(|e| {
            eprintln!("error: {e}");
            ExitCode::from(1)
        });
    }
    if cmd == "home" {
        match voltc::workspace::ensure() {
            Ok(ws) => {
                println!("home      {}", ws.root.display());
                println!("projects  {}", ws.projects.display());
                println!("addons    {}", ws.addons.display());
                println!("tools     {}", ws.tools.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!("error: {e}");
                return Err(ExitCode::from(1));
            }
        }
    }
    if cmd == "boards" {
        let q = rest.first().map(String::as_str).unwrap_or("").trim();
        if q.is_empty() {
            println!("name           display                      fqbn");
            println!("{}", voltc::board::known_names());
            return Ok(());
        }
        let hits = voltc::board::search_boards(q);
        if hits.is_empty() {
            eprintln!("error: {}", voltc::board::unknown_board_message(q));
            return Err(ExitCode::from(2));
        }
        println!("name           display                      fqbn");
        for b in hits {
            let fqbn = b.fqbn.unwrap_or("-");
            println!("  {:<14} {:<28} {}", b.name, b.display, fqbn);
        }
        return Ok(());
    }
    if cmd == "tools" {
        return run_tools(rest);
    }
    if cmd == "flash" {
        return run_flash(rest);
    }
    if cmd == "ports" {
        let ports = voltc::ports::list_serial_ports();
        if ports.is_empty() {
            println!(
                "no serial ports (plug in a board; on macOS look for /dev/cu.usbmodem*)"
            );
        } else {
            for p in ports {
                println!("{:<28} {}", p.address, p.label);
            }
        }
        return Ok(());
    }

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
        "report" => compile_report(path, &parsed),
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
    firmware: bool,
    port: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut out = Args {
        input: None,
        output: None,
        target: "host".into(),
        emit: "c".into(),
        include_paths: Vec::new(),
        std_path: None,
        firmware: false,
        port: None,
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-o" | "--output" => {
                i += 1;
                let p = PathBuf::from(args.get(i).ok_or("missing argument for -o")?);
                if p.extension().and_then(|s| s.to_str()) == Some("hex")
                    || p.extension().and_then(|s| s.to_str()) == Some("elf")
                {
                    out.firmware = true;
                }
                out.output = Some(p);
            }
            "--target" | "--board" => {
                i += 1;
                out.target = args
                    .get(i)
                    .ok_or("missing argument for --target / --board")?
                    .clone();
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
            "--firmware" | "--hex" => out.firmware = true,
            "--port" => {
                i += 1;
                out.port = Some(
                    args.get(i)
                        .ok_or("missing argument for --port")?
                        .clone(),
                );
            },
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
        eprintln!("error: {}", voltc::board::unknown_board_message(&args.target));
        ExitCode::from(2)
    })?;
    Ok(CompileOptions {
        target,
        std_path: args
            .std_path
            .clone()
            .unwrap_or_else(driver::default_std_path),
        include_paths: {
            let mut paths = args.include_paths.clone();
            paths.extend(voltc::workspace::addon_search_paths());
            paths
        },
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
    if args.emit == "report" {
        return compile_report(path, args);
    }
    let opts = options_from(args)?;
    match driver::compile_file(path, &opts) {
        Ok(result) => {
            if args.firmware {
                if let Some(board) = voltc::board::find_board(&args.target) {
                    let dir = args
                        .output
                        .as_ref()
                        .and_then(|p| p.parent())
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from("."));
                    match voltc::toolchain::build_firmware(&result.c_source, board, &dir) {
                        Ok(fw) => eprintln!("firmware {}", fw.display()),
                        Err(e) => {
                            eprintln!("error: {e}");
                            return Err(ExitCode::from(1));
                        }
                    }
                }
            } else if let Some(out) = &args.output {
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

fn compile_report(path: &Path, args: &Args) -> Result<(), ExitCode> {
    let opts = options_from(args)?;
    match driver::compile_file(path, &opts) {
        Ok(result) => {
            print!("{}", result.report);
            Ok(())
        }
        Err(err) => print_compile_fail(&err),
    }
}

fn run_svd(args: &[String]) -> Result<(), ExitCode> {
    let parsed = parse_args(args).map_err(|e| {
        eprintln!("error: {e}");
        ExitCode::from(2)
    })?;
    let Some(input) = parsed.input.clone() else {
        eprintln!("error: missing SVD file");
        return Err(ExitCode::from(2));
    };
    let xml = std::fs::read_to_string(&input).map_err(|e| {
        eprintln!("error: cannot read {input}: {e}");
        ExitCode::from(1)
    })?;
    match voltc::svd::svd_to_volt(&xml) {
        Ok(volt) => {
            if let Some(out) = parsed.output {
                if let Err(e) = std::fs::write(&out, &volt) {
                    eprintln!("error: cannot write {}: {e}", out.display());
                    return Err(ExitCode::from(1));
                }
            } else {
                print!("{volt}");
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(ExitCode::from(1))
        }
    }
}

fn run_ide(args: &[String]) -> Result<(), ExitCode> {
    let mut port: u16 = 8741;
    let mut host = "127.0.0.1".to_string();
    let mut open = true;
    let mut window = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                let p = args.get(i).ok_or_else(|| {
                    eprintln!("error: missing argument for --port");
                    ExitCode::from(2)
                })?;
                port = p.parse().map_err(|_| {
                    eprintln!("error: invalid --port");
                    ExitCode::from(2)
                })?;
            }
            "--bind" => {
                i += 1;
                host = args
                    .get(i)
                    .ok_or_else(|| {
                        eprintln!("error: missing argument for --bind");
                        ExitCode::from(2)
                    })?
                    .clone();
            }
            "--no-open" => open = false,
            "--open" => open = true,
            "--window" | "--app" => window = true,
            "--browser" => window = false,
            s => {
                eprintln!("error: unknown option `{s}`");
                return Err(ExitCode::from(2));
            }
        }
        i += 1;
    }
    if window {
        return run_app();
    }
    let bind = format!("{host}:{port}");
    voltc::ide_server::run(&bind, open).map_err(|e| {
        eprintln!("error: {e}");
        ExitCode::from(1)
    })
}

fn run_app() -> Result<(), ExitCode> {
    #[cfg(feature = "app")]
    {
        return voltc::app::run().map_err(|e| {
            eprintln!("error: {e}");
            ExitCode::from(1)
        });
    }
    #[cfg(not(feature = "app"))]
    {
        eprintln!(
            "error: native Volt app is not in this binary\n  cargo run --features app --bin volt\n  ./scripts/package-macos.sh\n  .\\scripts\\package-windows.cmd"
        );
        Err(ExitCode::from(2))
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
  voltc compile <file.volt> [-o out.c] [--board uno] [--firmware]
  voltc check   <file.volt>
  voltc ast     <file.volt>
  voltc tokens  <file.volt>
  voltc report  <file.volt>
  voltc svd     <file.svd> [-o out.volt]
  voltc boards  [query]
  voltc tools   [install <avr|arm|esp8266|esp32>]
  voltc flash   <file.volt> [--board uno] [--port /dev/cu.usbmodem*]
  voltc home
  voltc ports
  voltc ide     [--port 8741] [--no-open] [--window]
  voltc app
  voltc lsp

Options:
  -o, --output <path>     Write generated C (or firmware, for .hex) to a file
  --board, --target <id>  Arduino FQBN or id (`voltc boards [query]`)
  --firmware, --hex       Also run the board compiler (needs `voltc tools`)
  --emit <kind>           c | ast | tokens | report
  -I <dir>                Extra module search path
  --std-path <dir>        Standard library root (default: ./std)
  --port <n>              IDE bind port (default 8741)
  --bind <addr>           IDE bind host (default 127.0.0.1)
  --no-open               Do not open a browser for `voltc ide`
  --window, --app         Open the native Volt app window
  --browser               Force the browser UI (default without --features app)
  -h, --help              Show this help
  -V, --version           Show version

Pipeline:
  .volt  →  lexer  →  parser  →  sema  →  C  →  bundled avr-gcc / ARM / ESP
"
    );
    let _ = HOST;
}

fn run_tools(args: &[String]) -> Result<(), ExitCode> {
    let sub = args.first().map(String::as_str).unwrap_or("");
    if sub == "install" {
        let pack = args.get(1).map(String::as_str).unwrap_or("");
        if pack.is_empty() {
            eprintln!("error: voltc tools install <avr|arm|esp8266|esp32>");
            return Err(ExitCode::from(2));
        }
        eprintln!("downloading {pack} into Documents/Volt/tools …");
        match voltc::toolchain::install(pack) {
            Ok(st) => {
                println!("{}  {}", st.pack, st.note);
                if let Some(cc) = st.cc {
                    println!("cc  {cc}");
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("error: {e}");
                Err(ExitCode::from(1))
            }
        }
    } else {
        println!("id       shipped  ready  name");
        for st in voltc::toolchain::list_status() {
            println!(
                "{:<8} {:<7} {:<5} {}",
                st.pack,
                if st.ships_in_installer { "yes" } else { "no" },
                if st.installed { "yes" } else { "no" },
                st.name
            );
        }
        if let Ok(dir) = voltc::toolchain::user_tools_dir() {
            println!("dir  {}", dir.display());
        }
        Ok(())
    }
}

fn run_flash(args: &[String]) -> Result<(), ExitCode> {
    let parsed = parse_args(args).map_err(|e| {
        eprintln!("error: {e}");
        ExitCode::from(2)
    })?;
    let Some(input) = parsed.input.clone() else {
        eprintln!("error: missing input file");
        return Err(ExitCode::from(2));
    };
    let board = voltc::board::find_board(&parsed.target).ok_or_else(|| {
        eprintln!("error: {}", voltc::board::unknown_board_message(&parsed.target));
        ExitCode::from(2)
    })?;
    let opts = options_from(&parsed)?;
    let result = driver::compile_file(Path::new(&input), &opts).map_err(|err| {
        let _ = print_compile_fail(&err);
        ExitCode::from(1)
    })?;
    let out = std::env::temp_dir().join("volt-flash");
    let fw = voltc::toolchain::build_firmware(&result.c_source, board, &out).map_err(|e| {
        eprintln!("error: {e}");
        ExitCode::from(1)
    })?;
    let port = parsed
        .port
        .or_else(|| std::env::var("VOLT_PORT").ok().filter(|s| !s.is_empty()))
        .or_else(voltc::ports::preferred_port)
        .unwrap_or_default();
    match voltc::toolchain::flash_firmware(&fw, board, &port) {
        Ok(log) => {
            print!("{log}");
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(ExitCode::from(1))
        }
    }
}
