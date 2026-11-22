use flate2::Compression;
use flate2::write::GzEncoder;
use std::env;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must set CARGO_MANIFEST_DIR"),
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"));
    embed_font(&manifest, &output).expect("MiSans Medium font compression failed");
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

    let icon_path = manifest.join("assets/icon.ico");
    println!("cargo:rerun-if-changed={}", icon_path.display());

    let icon_path = icon_path
        .to_str()
        .expect("Windows icon path must contain valid Unicode");
    winresource::WindowsResource::new()
        .set_icon(icon_path)
        .set("ProductName", "Cast Assembly")
        .set("FileDescription", "CAST 装配台")
        .compile()
        .expect("Windows application resources should compile");
}

fn embed_font(manifest: &Path, output: &Path) -> io::Result<()> {
    let source = manifest.join("assets/fonts/MiSans-Medium.ttf");
    println!("cargo:rerun-if-changed={}", source.display());
    let mut font = File::open(source)?;
    let mut encoder = GzEncoder::new(
        File::create(output.join("MiSans-Medium.ttf.gz"))?,
        Compression::best(),
    );
    io::copy(&mut font, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}
