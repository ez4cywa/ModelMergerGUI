use flate2::Compression;
use flate2::write::GzEncoder;
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must set CARGO_MANIFEST_DIR"),
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must set OUT_DIR"));
    embed_font(&manifest, &output).expect("MiSans Medium font compression failed");
    let icon_path = manifest.join("assets/icon.ico");
    println!("cargo:rerun-if-changed={}", icon_path.display());
    let icon = fs::read(&icon_path).expect("could not read assets/icon.ico");
    let png = ui_icon_png(&icon).expect("assets/icon.ico must contain a 256×256 PNG icon");
    fs::write(output.join("app-icon.png"), png).expect("could not write the shared UI icon");
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

fn ui_icon_png(ico: &[u8]) -> Option<&[u8]> {
    // ICO represents a 256×256 image with zero in both size bytes.
    const HEADER_SIZE: usize = 6;
    const ENTRY_SIZE: usize = 16;
    const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    let header = ico.get(..HEADER_SIZE)?;
    if header[..4] != [0, 0, 1, 0] {
        return None;
    }
    let image_count = usize::from(u16::from_le_bytes(header[4..6].try_into().ok()?));
    let directory_end = HEADER_SIZE + image_count * ENTRY_SIZE;
    let entries = ico.get(HEADER_SIZE..directory_end)?;
    entries.chunks_exact(ENTRY_SIZE).find_map(|entry| {
        if entry[..2] != [0, 0] {
            return None;
        }
        let length = usize::try_from(u32::from_le_bytes(entry[8..12].try_into().ok()?)).ok()?;
        let start = usize::try_from(u32::from_le_bytes(entry[12..16].try_into().ok()?)).ok()?;
        if start < directory_end {
            return None;
        }
        let image = ico.get(start..start.checked_add(length)?)?;
        image.starts_with(PNG_SIGNATURE).then_some(image)
    })
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
