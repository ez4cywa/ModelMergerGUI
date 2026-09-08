use crate::{AppLanguage, RootMode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowBounds {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl WindowBounds {
    pub fn new(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    fn is_valid(self) -> bool {
        self.left.is_finite()
            && self.top.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width >= 800.0
            && self.height >= 600.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub schema_version: u32,
    #[serde(rename = "uiLanguage")]
    pub language: Option<AppLanguage>,
    pub preferred_output_directory: Option<PathBuf>,
    pub remember_output_directory: bool,
    pub check_updates_on_startup: bool,
    #[serde(rename = "rootSelectionMode")]
    pub root_mode: RootMode,
    pub window_bounds: Option<WindowBounds>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: 2,
            language: None,
            preferred_output_directory: None,
            remember_output_directory: false,
            check_updates_on_startup: false,
            root_mode: RootMode::Automatic,
            window_bounds: None,
        }
    }
}

impl AppSettings {
    fn sanitized(mut self) -> Self {
        self.schema_version = 2;
        if !self.remember_output_directory
            || self
                .preferred_output_directory
                .as_ref()
                .is_none_or(|path| !path.is_dir())
        {
            self.preferred_output_directory = None;
            self.remember_output_directory = false;
        }
        if self.window_bounds.is_some_and(|bounds| !bounds.is_valid()) {
            self.window_bounds = None;
        }
        self
    }
}

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    Json(serde_json::Error),
    MissingParent(PathBuf),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => source.fmt(formatter),
            Self::Json(source) => source.fmt(formatter),
            Self::MissingParent(path) => {
                write!(formatter, "settings path has no parent: {}", path.display())
            }
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<std::io::Error> for SettingsError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

impl From<serde_json::Error> for SettingsError {
    fn from(source: serde_json::Error) -> Self {
        Self::Json(source)
    }
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn default_for_local_app_data(local_app_data: &Path) -> Self {
        Self::new(local_app_data.join("CastModelMerger").join("settings.json"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> AppSettings {
        let backup_path = self.path.with_extension("json.backup");
        if let Some(settings) = read_settings(&self.path) {
            let _ = fs::remove_file(backup_path);
            return settings.sanitized();
        }
        if let Some(settings) = read_settings(&backup_path) {
            if !self.path.exists() {
                let _ = fs::rename(&backup_path, &self.path);
            }
            return settings.sanitized();
        }
        AppSettings::default()
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), SettingsError> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let parent = self
            .path
            .parent()
            .ok_or_else(|| SettingsError::MissingParent(self.path.clone()))?;
        fs::create_dir_all(parent)?;
        let temporary_path = self.path.with_extension(format!(
            "json.{}.{:016x}.tmp",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let backup_path = self.path.with_extension("json.backup");
        let temporary = TemporarySettings::new(temporary_path.clone());
        let bytes = serde_json::to_vec_pretty(&settings.clone().sanitized())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);

        if self.path.exists() {
            let _ = fs::remove_file(&backup_path);
            fs::rename(&self.path, &backup_path)?;
        }
        match fs::rename(&temporary_path, &self.path) {
            Ok(()) => {
                temporary.disarm();
                let _ = fs::remove_file(backup_path);
                Ok(())
            }
            Err(source) => {
                if backup_path.exists() {
                    let _ = fs::rename(&backup_path, &self.path);
                }
                Err(SettingsError::Io(source))
            }
        }
    }
}

fn read_settings(path: &Path) -> Option<AppSettings> {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

struct TemporarySettings {
    path: PathBuf,
    armed: std::cell::Cell<bool>,
}

impl TemporarySettings {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            armed: std::cell::Cell::new(true),
        }
    }

    fn disarm(&self) {
        self.armed.set(false);
    }
}

impl Drop for TemporarySettings {
    fn drop(&mut self) {
        if self.armed.get() {
            let _ = fs::remove_file(&self.path);
        }
    }
}
