use model_merger_app_core::{AppLanguage, AppSettings, RootMode, SettingsStore, WindowBounds};
use std::path::PathBuf;

#[test]
fn settings_round_trip_uses_schema_two_and_leaves_no_temporary_file() {
    let directory = TestDirectory::new();
    let output = directory.path.join("output");
    std::fs::create_dir(&output).unwrap();
    let path = directory.path.join("settings.json");
    let store = SettingsStore::new(&path);
    let settings = AppSettings {
        language: Some(AppLanguage::French),
        preferred_output_directory: Some(output.clone()),
        remember_output_directory: true,
        root_mode: RootMode::Manual,
        window_bounds: Some(WindowBounds::new(120.0, 80.0, 1166.0, 854.0)),
        ..AppSettings::default()
    };

    store.save(&settings).unwrap();
    let loaded = store.load();

    assert_eq!(2, loaded.schema_version);
    assert_eq!(settings, loaded);
    assert_eq!(1, std::fs::read_dir(&directory.path).unwrap().count() - 1);

    let replacement = AppSettings {
        language: Some(AppLanguage::English),
        ..settings
    };
    store.save(&replacement).unwrap();
    assert_eq!(replacement, store.load());
    assert!(!directory.path.join("settings.json.backup").exists());
}

#[test]
fn corrupt_or_invalid_settings_fall_back_to_sanitized_defaults() {
    let directory = TestDirectory::new();
    let path = directory.path.join("settings.json");
    std::fs::write(&path, b"not json").unwrap();
    let store = SettingsStore::new(&path);
    assert_eq!(AppSettings::default(), store.load());

    std::fs::write(
        &path,
        br#"{"schemaVersion":99,"uiLanguage":4,"preferredOutputDirectory":"missing","rememberOutputDirectory":true,"rootSelectionMode":0,"windowBounds":{"left":0,"top":0,"width":200,"height":100}}"#,
    )
    .unwrap();
    let loaded = store.load();
    assert_eq!(2, loaded.schema_version);
    assert_eq!(Some(AppLanguage::Spanish), loaded.language);
    assert!(!loaded.remember_output_directory);
    assert!(loaded.preferred_output_directory.is_none());
    assert!(loaded.window_bounds.is_none());
}

#[test]
fn load_recovers_the_previous_settings_if_replacement_was_interrupted() {
    let directory = TestDirectory::new();
    let path = directory.path.join("settings.json");
    let backup = directory.path.join("settings.json.backup");
    let store = SettingsStore::new(&path);
    let settings = AppSettings {
        language: Some(AppLanguage::Russian),
        ..AppSettings::default()
    };
    store.save(&settings).unwrap();
    std::fs::rename(&path, &backup).unwrap();

    assert_eq!(settings, store.load());
    assert!(path.is_file());
    assert!(!backup.exists());
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "model-merger-settings-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
