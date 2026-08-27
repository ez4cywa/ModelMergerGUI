use model_merger_app_core::{AddPartStatus, AppLanguage, AppSettings};
use model_merger_gui::{NativeAppState, slot_columns};
use std::path::PathBuf;

#[test]
fn workspace_starts_with_one_group_and_preserves_state_across_language_switches() {
    let directory = TestDirectory::new();
    let first = directory.cast("body.cast");
    let second = directory.cast("head.cast");
    let mut state = NativeAppState::new(AppSettings::default());

    assert_eq!(1, state.groups().len());
    assert_eq!(AddPartStatus::Added, state.add_part(0, &first).status);
    assert_eq!(AddPartStatus::Added, state.add_part(0, &second).status);
    state.set_language(AppLanguage::French);

    assert_eq!(AppLanguage::French, state.language());
    assert_eq!(
        vec![first, second],
        state.groups()[0].plan.state().part_files
    );
    assert!(state.groups()[0].plan.state().is_ready);
}

#[test]
fn groups_can_be_added_collapsed_and_deleted_without_affecting_other_groups() {
    let mut state = NativeAppState::new(AppSettings::default());
    let second = state.add_group();
    state.set_collapsed(0, true);

    assert_eq!(1, second);
    assert!(state.groups()[0].collapsed);
    assert!(state.delete_group(1));
    assert_eq!(1, state.groups().len());
    assert!(!state.delete_group(0));
}

#[test]
fn slot_grid_reflows_before_horizontal_scrolling_is_needed() {
    assert_eq!(5, slot_columns(760.0));
    assert_eq!(3, slot_columns(560.0));
    assert_eq!(2, slot_columns(360.0));
}

#[test]
fn batch_import_preserves_selection_order_and_respects_the_fifteen_part_limit() {
    let directory = TestDirectory::new();
    let paths: Vec<_> = (1..=16)
        .map(|index| directory.cast(&format!("part-{index:02}.cast")))
        .collect();
    let mut state = NativeAppState::new(AppSettings::default());

    let results = state.add_parts(0, paths.iter());

    assert_eq!(16, results.len());
    assert!(
        results[..15]
            .iter()
            .all(|result| result.status == AddPartStatus::Added)
    );
    assert_eq!(AddPartStatus::Full, results[15].status);
    assert_eq!(paths[..15], state.groups()[0].plan.state().part_files);
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "model-merger-native-state-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn cast(&self, name: &str) -> PathBuf {
        let path = self.path.join(name);
        std::fs::write(&path, b"cast").unwrap();
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
