use std::env;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repository = manifest.join("../../..");
    let revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&repository)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| value.len() == 40)
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=CAST_BUILD_COMMIT={revision}");
    println!(
        "cargo:rerun-if-changed={}",
        repository.join(".git/HEAD").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        repository.join(".git/refs/heads").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        repository.join(".git/packed-refs").display()
    );
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
