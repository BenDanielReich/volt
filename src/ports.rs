//! Serial ports the way Arduino IDE lists them on each OS.
//!
//! macOS: `/dev/cu.usbmodem*`, `/dev/cu.usbserial*`, `/dev/cu.wchusbserial*`
//! (skip Bluetooth and debug consoles). Windows: `COM1`…`COM32`. Linux:
//! `/dev/ttyACM*` / `/dev/ttyUSB*`.

pub struct SerialPort {
    pub address: String,
    pub label: String,
}

pub fn list_serial_ports() -> Vec<SerialPort> {
    let mut ports = inner_list();
    ports.sort_by(|a, b| a.address.cmp(&b.address));
    ports
}

pub fn preferred_port() -> Option<String> {
    list_serial_ports().into_iter().next().map(|p| p.address)
}

/// Example port token for flash recipes when nothing is plugged in.
pub fn port_placeholder() -> &'static str {
    if cfg!(windows) {
        "COM3"
    } else if cfg!(target_os = "macos") {
        "/dev/cu.usbmodem*"
    } else {
        "/dev/ttyACM0"
    }
}

pub fn os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}

fn inner_list() -> Vec<SerialPort> {
    #[cfg(target_os = "macos")]
    {
        return list_macos();
    }
    #[cfg(windows)]
    {
        return list_windows();
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        list_unix()
    }
}

#[cfg(target_os = "macos")]
fn list_macos() -> Vec<SerialPort> {
    let mut out = Vec::new();
    let Ok(dir) = std::fs::read_dir("/dev") else {
        return out;
    };
    for ent in dir.flatten() {
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("cu.") {
            continue;
        }
        if skip_macos_cu(&name) {
            continue;
        }
        let address = format!("/dev/{name}");
        let label = mac_label(&name);
        out.push(SerialPort { address, label });
    }
    out
}

#[cfg(target_os = "macos")]
fn skip_macos_cu(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("bluetooth")
        || n.contains("incoming")
        || n == "cu.debug-console"
        || n == "cu.soc"
        || n == "cu.mals"
        || n.contains("wlan")
        || n.contains("iphone")
        || n.contains("ipad")
}

#[cfg(target_os = "macos")]
fn mac_label(cu_name: &str) -> String {
    let rest = cu_name.strip_prefix("cu.").unwrap_or(cu_name);
    if rest.to_ascii_lowercase().starts_with("usbmodem") {
        format!("{cu_name} (USB)")
    } else if rest.to_ascii_lowercase().contains("usbserial")
        || rest.to_ascii_lowercase().contains("wchusb")
        || rest.to_ascii_lowercase().contains("slab")
    {
        format!("{cu_name} (USB serial)")
    } else {
        cu_name.to_string()
    }
}

#[cfg(windows)]
fn list_windows() -> Vec<SerialPort> {
    let mut out = Vec::new();
    for i in 1..=32u8 {
        let address = format!("COM{i}");
        let path = format!(r"\\.\{address}");
        if std::path::Path::new(&path).exists() {
            out.push(SerialPort {
                address,
                label: format!("COM{i} (Serial)"),
            });
        }
    }
    out
}

#[cfg(not(any(target_os = "macos", windows)))]
fn list_unix() -> Vec<SerialPort> {
    let mut out = Vec::new();
    for pattern in ["/dev/ttyACM", "/dev/ttyUSB", "/dev/ttyAMA"] {
        for i in 0..16 {
            let address = format!("{pattern}{i}");
            if std::path::Path::new(&address).exists() {
                out.push(SerialPort {
                    address: address.clone(),
                    label: address,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_matches_os() {
        let p = port_placeholder();
        if cfg!(target_os = "macos") {
            assert!(p.contains("/dev/cu."));
        } else if cfg!(windows) {
            assert!(p.starts_with("COM"));
        } else {
            assert!(p.contains("/dev/"));
        }
    }

    #[test]
    fn list_does_not_panic() {
        let _ = list_serial_ports();
    }
}
