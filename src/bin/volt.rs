//! Volt IDE — native desktop app for macOS and Windows.
//!
//! Build with: `cargo build --release --features app --bin volt`

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(e) = voltc::app::run() {
        eprintln!("error: {e}");
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/C", "start", "", "cmd", "/K", "echo Volt IDE failed to start. & echo error: ", &e])
                .status();
        }
        std::process::exit(1);
    }
}
