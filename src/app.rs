//! Native Volt IDE window (macOS WKWebView / Windows WebView2).
//!
//! Built with `--features app`. The `volt` binary is the GUI; `voltc` stays the CLI.

use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

pub fn run() -> Result<(), String> {
    prepare_paths();
    let url = crate::ide_server::spawn("127.0.0.1:0")?;
    let url = format!("{}?v={}", url.trim_end_matches('/'), env!("CARGO_PKG_VERSION"));
    open_window(&url)
}

fn prepare_paths() {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let resources = dir.join("../Resources");
            if resources.join("std").is_dir() {
                if let Ok(res) = resources.canonicalize() {
                    if std::env::var_os("VOLT_STD").is_none() {
                        std::env::set_var("VOLT_STD", res.join("std"));
                    }
                    let _ = std::env::set_current_dir(&res);
                    return;
                }
            }
            if dir.join("std").is_dir() {
                if std::env::var_os("VOLT_STD").is_none() {
                    std::env::set_var("VOLT_STD", dir.join("std"));
                }
                let _ = std::env::set_current_dir(dir);
            }
        }
    }
}

fn open_window(url: &str) -> Result<(), String> {
    let mut event_loop = EventLoopBuilder::new().build();
    #[cfg(target_os = "macos")]
    {
        use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
        event_loop.set_activation_policy(ActivationPolicy::Regular);
    }
    let window = WindowBuilder::new()
        .with_title("Volt")
        .with_inner_size(LogicalSize::new(1280.0, 840.0))
        .with_min_inner_size(LogicalSize::new(720.0, 480.0))
        .build(&event_loop)
        .map_err(|e| format!("cannot create window: {e}"))?;

    let _webview = WebViewBuilder::new()
        .with_url(url)
        .with_devtools(cfg!(debug_assertions))
        .build(&window)
        .map_err(|e| format!("cannot create webview: {e}"))?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
