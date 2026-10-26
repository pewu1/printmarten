fn main() {
    println!("cargo:rerun-if-changed=packaging/printmarten.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("packaging/printmarten.ico")
            .set("ProductName", "PrintMarten")
            .set("FileDescription", "PrintMarten photo printing")
            .compile()
            .expect("Could not embed PrintMarten's Windows icon");
    }
}
