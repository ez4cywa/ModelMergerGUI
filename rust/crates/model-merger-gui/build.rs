use std::env;
use std::path::PathBuf;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon_path =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("assets/icon.ico");
    println!("cargo:rerun-if-changed={}", icon_path.display());

    let icon_path = icon_path
        .to_str()
        .expect("Windows icon path must contain valid Unicode");
    winresource::WindowsResource::new()
        .set_icon(icon_path)
        .compile()
        .expect("Windows application resources should compile");
}
