use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

pub fn install_panic_hook() -> io::Result<PathBuf> {
    let directory = log_directory();
    fs::create_dir_all(&directory)?;
    let _ = LOG_DIRECTORY.set(directory.clone());
    std::panic::set_hook(Box::new(|panic_info| {
        let details = format!("unhandled panic: {panic_info}");
        let _ = append("panic", &details);
    }));
    append("startup", "application launch requested")?;
    Ok(directory)
}

pub fn record_startup_error(error: &str) {
    let _ = append("startup-error", error);
}

pub fn record_runtime_error(area: &str, error: &str) {
    let _ = append(area, error);
}

pub fn log_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("CastModelMerger")
        .join("logs")
}

fn append(kind: &str, message: &str) -> io::Result<()> {
    let directory = LOG_DIRECTORY.get().cloned().unwrap_or_else(log_directory);
    fs::create_dir_all(&directory)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("CastModelMerger.log"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    writeln!(file, "[{timestamp}] {kind}: {message}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_directory_has_a_stable_product_suffix() {
        assert!(log_directory().ends_with(PathBuf::from("CastModelMerger").join("logs")));
    }
}
