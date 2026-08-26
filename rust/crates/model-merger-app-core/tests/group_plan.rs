use model_merger_app_core::{AddPartStatus, GroupPlan, RootMode};
use std::path::{Path, PathBuf};

#[test]
fn adding_parts_remembers_the_directory_and_builds_the_default_output() {
    let directory = TestDirectory::new();
    let first = directory.cast("body.cast");
    let second = directory.cast("head.cast");
    let mut plan = GroupPlan::default();

    assert_eq!(AddPartStatus::Added, plan.add_part(&first).status);
    assert_eq!(AddPartStatus::Added, plan.add_part(&second).status);

    let state = plan.state();
    assert_eq!(
        Some(directory.path.as_path()),
        state.recent_input_directory.as_deref()
    );
    assert_eq!(directory.path.join("Merged Models"), state.output_directory);
    assert!(state.is_ready);
}

#[test]
fn duplicate_and_sixteenth_parts_are_rejected_without_changing_accepted_slots() {
    let directory = TestDirectory::new();
    let mut plan = GroupPlan::default();
    let first = directory.cast("part-00.cast");
    assert_eq!(AddPartStatus::Added, plan.add_part(&first).status);
    assert_eq!(AddPartStatus::Duplicate, plan.add_part(&first).status);
    for index in 1..15 {
        assert_eq!(
            AddPartStatus::Added,
            plan.add_part(directory.cast(&format!("part-{index:02}.cast")))
                .status
        );
    }

    assert_eq!(
        AddPartStatus::Full,
        plan.add_part(directory.cast("part-15.cast")).status
    );
    assert_eq!(15, plan.state().part_files.len());
}

#[test]
fn replacing_a_manual_root_keeps_the_replacement_as_root() {
    let directory = TestDirectory::new();
    let mut plan = GroupPlan::default();
    let first = directory.cast("body.cast");
    let second = directory.cast("head.cast");
    let replacement = directory.cast("head-v2.cast");
    plan.add_part(&first);
    plan.add_part(&second);
    assert!(plan.set_manual_root(1));

    assert_eq!(
        AddPartStatus::Added,
        plan.replace_part(1, &replacement).status
    );

    let state = plan.state();
    assert_eq!(RootMode::Manual, state.root_mode);
    assert_eq!(
        Some(replacement.as_path()),
        state.manual_root_file.as_deref()
    );
}

#[test]
fn path_alias_cannot_add_the_same_part_twice() {
    let directory = TestDirectory::new();
    let nested = directory.path.join("nested");
    std::fs::create_dir(&nested).unwrap();
    let part = directory.cast("body.cast");
    let alias = nested.join("..").join("body.cast");
    let mut plan = GroupPlan::default();

    assert_eq!(AddPartStatus::Added, plan.add_part(&part).status);
    assert_eq!(AddPartStatus::Duplicate, plan.add_part(&alias).status);
    assert_eq!(1, plan.state().part_files.len());
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "model-merger-group-plan-{}-{}",
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

fn _assert_path(_: &Path) {}
