fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon.ico");
            res.set("ProductName", "Klick");
            res.set(
                "FileDescription",
                "Klick — Precision Desktop Auto-Clicker",
            );
            res.set("LegalCopyright", "GNU General Public License v3.0");
            if let Err(err) = res.compile() {
                println!("cargo:warning=Windows resource embedding failed: {err}");
            }
        }
    }
}
