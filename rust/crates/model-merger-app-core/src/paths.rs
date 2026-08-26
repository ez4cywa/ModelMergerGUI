use std::path::{Path, PathBuf};

pub(crate) fn identity_key(path: &Path) -> String {
    let resolved = std::fs::canonicalize(path)
        .or_else(|_| canonicalize_parent(path))
        .unwrap_or_else(|_| absolute(path));
    resolved.to_string_lossy().to_ascii_lowercase()
}

fn canonicalize_parent(path: &Path) -> Result<PathBuf, std::io::Error> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name().unwrap_or_default();
    std::fs::canonicalize(parent).map(|parent| parent.join(file_name))
}

fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|directory| directory.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}
