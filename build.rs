fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=assets/DesktopLayoutManager.ico");
        if let Err(error) = winres::WindowsResource::new()
            .set_icon("assets/DesktopLayoutManager.ico")
            .set("FileDescription", "Desktop Layout Manager")
            .set("ProductName", "Desktop Layout Manager")
            .compile()
        {
            panic!("Failed to embed the application icon into the EXE: {error}");
        }
    }
}
