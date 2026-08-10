fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/keykoff.ico");
        // FileVersion/ProductVersion are filled from CARGO_PKG_VERSION automatically.
        res.set("ProductName", "keykoff");
        res.set("FileDescription", "keykoff quick launcher");
        res.set("CompanyName", "Lars Wallenborn");
        res.set("LegalCopyright", "Copyright (c) 2026 Lars Wallenborn");
        res.set("OriginalFilename", "keykoff.exe");
        res.set("InternalName", "keykoff");
        if let Err(e) = res.compile() {
            panic!("failed to compile Windows resources: {e}");
        }
    }
}
