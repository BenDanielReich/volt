fn main() {
    println!("cargo:rerun-if-changed=ide/windows/Volt.ico");
    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new("ide/windows/Volt.ico").exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon("ide/windows/Volt.ico");
            res.set("ProductName", "Volt");
            res.set("FileDescription", "Volt IDE");
            let _ = res.compile();
        }
    }
}
