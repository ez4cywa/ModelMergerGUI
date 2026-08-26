use model_merger_app_core::{
    MergeBackend, PreparedTask, TaskContext, TaskError, TaskScheduler, TaskState,
};
use model_merger_engine::{MergeRequest, MergeResult, MergeValidationCode, RootSelection};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[test]
fn scheduler_runs_at_most_two_groups_and_starts_the_queued_group_after_release() {
    let backend = Arc::new(BlockingBackend::default());
    let scheduler = TaskScheduler::new(backend.clone(), 2).unwrap();
    let first = scheduler.schedule(request("first.cast")).unwrap();
    let second = scheduler.schedule(request("second.cast")).unwrap();
    let third = scheduler.schedule(request("third.cast")).unwrap();
    backend.wait_for_started(2);

    assert_eq!(2, backend.state.maximum_active.load(Ordering::Acquire));
    assert_eq!(TaskState::Queued, scheduler.snapshot(third).unwrap().state);
    backend.state.release.store(true, Ordering::Release);
    backend.wake_all();

    assert_eq!(TaskState::Succeeded, scheduler.wait(first).unwrap().state);
    assert_eq!(TaskState::Succeeded, scheduler.wait(second).unwrap().state);
    assert_eq!(TaskState::Succeeded, scheduler.wait(third).unwrap().state);
}

#[test]
fn product_scheduler_rejects_more_than_two_workers() {
    assert!(matches!(
        TaskScheduler::new(Arc::new(BlockingBackend::default()), 3),
        Err(TaskError::InvalidConcurrency)
    ));
}

#[test]
fn scheduler_shutdown_is_bounded_when_a_backend_ignores_cancellation() {
    let backend = Arc::new(BlockingBackend::default());
    let scheduler = TaskScheduler::new(backend.clone(), 1).unwrap();
    scheduler.schedule(request("blocked.cast")).unwrap();
    backend.wait_for_started(1);

    let started = Instant::now();
    drop(scheduler);

    assert!(started.elapsed() < Duration::from_secs(3));
    backend.state.release.store(true, Ordering::Release);
    backend.wake_all();
}

#[test]
fn backend_panic_fails_one_task_releases_its_claim_and_keeps_worker_alive() {
    let backend = Arc::new(PanicOnceBackend::default());
    let scheduler = TaskScheduler::new(backend, 1).unwrap();
    let first = scheduler.schedule(request("panic.cast")).unwrap();
    let second = scheduler.schedule(request("panic.cast")).unwrap();

    let first = scheduler.wait(first).unwrap();
    let second = scheduler.wait(second).unwrap();

    assert_eq!(TaskState::Failed, first.state);
    assert!(matches!(first.error, Some(TaskError::Backend(_))));
    assert_eq!(TaskState::Succeeded, second.state);
}

#[test]
fn queued_task_can_be_cancelled_without_entering_the_backend() {
    let backend = Arc::new(BlockingBackend::default());
    let scheduler = TaskScheduler::new(backend.clone(), 1).unwrap();
    let running = scheduler.schedule(request("running.cast")).unwrap();
    let queued = scheduler.schedule(request("queued.cast")).unwrap();
    backend.wait_for_started(1);

    assert!(scheduler.cancel(queued));
    assert_eq!(TaskState::Cancelled, scheduler.wait(queued).unwrap().state);
    assert_eq!(1, backend.state.started.load(Ordering::Acquire));
    backend.state.release.store(true, Ordering::Release);
    backend.wake_all();
    assert_eq!(TaskState::Succeeded, scheduler.wait(running).unwrap().state);
}

#[test]
fn concurrent_tasks_cannot_claim_the_same_output_path() {
    let backend = Arc::new(BlockingBackend::default());
    let scheduler = TaskScheduler::new(backend.clone(), 2).unwrap();
    let first = scheduler.schedule(request("shared.cast")).unwrap();
    let second = scheduler.schedule(request("shared.cast")).unwrap();
    backend.wait_for_started(1);
    for _ in 0..1_000 {
        if [first, second].into_iter().any(|id| {
            scheduler
                .snapshot(id)
                .is_some_and(|snapshot| snapshot.state == TaskState::Failed)
        }) {
            break;
        }
        std::thread::yield_now();
    }
    backend.state.release.store(true, Ordering::Release);
    backend.wake_all();

    let states = [
        scheduler.wait(first).unwrap(),
        scheduler.wait(second).unwrap(),
    ];
    assert_eq!(
        1,
        states
            .iter()
            .filter(|task| task.state == TaskState::Succeeded)
            .count()
    );
    let failed = states
        .iter()
        .find(|task| task.state == TaskState::Failed)
        .expect("one same-output task should fail");
    assert!(matches!(failed.error, Some(TaskError::OutputConflict(_))));
}

#[test]
fn aliased_output_paths_share_one_claim() {
    let directory = TestDirectory::new();
    let nested = directory.path.join("nested");
    std::fs::create_dir(&nested).unwrap();
    let backend = Arc::new(BlockingBackend::default());
    let scheduler = TaskScheduler::new(backend.clone(), 2).unwrap();
    let mut direct = request("shared.cast");
    direct.output_directory = directory.path.clone();
    let mut aliased = request("shared.cast");
    aliased.output_directory = nested.join("..");
    let first = scheduler.schedule(direct).unwrap();
    let second = scheduler.schedule(aliased).unwrap();
    backend.wait_for_started(1);
    for _ in 0..1_000 {
        if [first, second].into_iter().any(|id| {
            scheduler
                .snapshot(id)
                .is_some_and(|snapshot| snapshot.state == TaskState::Failed)
        }) {
            break;
        }
        std::thread::yield_now();
    }
    backend.state.release.store(true, Ordering::Release);
    backend.wake_all();

    let states = [
        scheduler.wait(first).unwrap(),
        scheduler.wait(second).unwrap(),
    ];
    assert_eq!(
        1,
        states
            .iter()
            .filter(|task| task.state == TaskState::Succeeded)
            .count()
    );
    assert_eq!(
        1,
        states
            .iter()
            .filter(|task| task.state == TaskState::Failed)
            .count()
    );
}

#[test]
fn native_backend_executes_a_golden_two_part_merge() {
    let directory = TestDirectory::new();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/rust-migration/golden-small");
    let scheduler = TaskScheduler::native(2).unwrap();
    let id = scheduler
        .schedule(MergeRequest {
            input_files: vec![fixtures.join("part-00.cast"), fixtures.join("part-01.cast")],
            output_directory: directory.path.clone(),
            output_file_name: Some("native.cast".to_owned()),
            root_selection: RootSelection::Automatic,
            overwrite: false,
        })
        .unwrap();

    let snapshot = scheduler.wait(id).unwrap();

    assert_eq!(TaskState::Succeeded, snapshot.state);
    let result = snapshot.result.unwrap();
    assert_eq!(2, result.part_count);
    assert!(directory.path.join("native.cast").is_file());
}

#[test]
fn native_backend_preserves_structured_validation_errors() {
    let scheduler = TaskScheduler::native(2).unwrap();
    let mut invalid = request("invalid.cast");
    invalid.input_files.truncate(1);
    let id = scheduler.schedule(invalid).unwrap();

    let snapshot = scheduler.wait(id).unwrap();

    assert_eq!(TaskState::Failed, snapshot.state);
    assert!(matches!(
        snapshot.error,
        Some(TaskError::Validation {
            code: MergeValidationCode::InvalidPartCount,
            ..
        })
    ));
}

fn request(name: &str) -> MergeRequest {
    MergeRequest {
        input_files: vec![PathBuf::from("a.cast"), PathBuf::from("b.cast")],
        output_directory: std::env::temp_dir(),
        output_file_name: Some(name.to_owned()),
        root_selection: RootSelection::Automatic,
        overwrite: true,
    }
}

#[derive(Clone, Default)]
struct BlockingBackend {
    state: Arc<BackendState>,
}

#[derive(Default)]
struct BackendState {
    active: AtomicUsize,
    maximum_active: AtomicUsize,
    started: AtomicUsize,
    release: AtomicBool,
    gate: (Mutex<()>, Condvar),
}

impl BlockingBackend {
    fn wait_for_started(&self, count: usize) {
        let (lock, condition) = &self.state.gate;
        let guard = lock.lock().unwrap();
        let _guard = condition
            .wait_timeout_while(guard, Duration::from_secs(3), |_| {
                self.state.started.load(Ordering::Acquire) < count
            })
            .unwrap();
        assert!(self.state.started.load(Ordering::Acquire) >= count);
    }

    fn wake_all(&self) {
        self.state.gate.1.notify_all();
    }
}

impl MergeBackend for BlockingBackend {
    fn prepare(
        &self,
        request: MergeRequest,
        _context: &TaskContext,
    ) -> Result<Box<dyn PreparedTask>, TaskError> {
        Ok(Box::new(BlockingPrepared {
            backend: Arc::clone(&self.state),
            output_path: request
                .output_directory
                .join(request.output_file_name.unwrap()),
        }))
    }
}

struct BlockingPrepared {
    backend: Arc<BackendState>,
    output_path: PathBuf,
}

impl PreparedTask for BlockingPrepared {
    fn output_path(&self) -> &Path {
        &self.output_path
    }

    fn execute(self: Box<Self>, context: &TaskContext) -> Result<MergeResult, TaskError> {
        let active = self.backend.active.fetch_add(1, Ordering::AcqRel) + 1;
        self.backend
            .maximum_active
            .fetch_max(active, Ordering::AcqRel);
        self.backend.started.fetch_add(1, Ordering::AcqRel);
        self.backend.gate.1.notify_all();
        let (lock, condition) = &self.backend.gate;
        let mut guard = lock.lock().unwrap();
        while !self.backend.release.load(Ordering::Acquire) && !context.is_cancelled() {
            guard = condition.wait(guard).unwrap();
        }
        self.backend.active.fetch_sub(1, Ordering::AcqRel);
        if context.is_cancelled() {
            return Err(TaskError::Cancelled);
        }
        Ok(MergeResult {
            output_path: self.output_path.clone(),
            root_model_name: "root".to_owned(),
            part_count: 2,
            bone_count: 1,
            mesh_count: 1,
            warnings: Vec::new(),
        })
    }
}

#[derive(Default)]
struct PanicOnceBackend {
    should_panic: AtomicBool,
}

impl MergeBackend for PanicOnceBackend {
    fn prepare(
        &self,
        request: MergeRequest,
        _context: &TaskContext,
    ) -> Result<Box<dyn PreparedTask>, TaskError> {
        let should_panic = !self.should_panic.swap(true, Ordering::AcqRel);
        Ok(Box::new(PanicPrepared {
            should_panic,
            output_path: request
                .output_directory
                .join(request.output_file_name.unwrap()),
        }))
    }
}

struct PanicPrepared {
    should_panic: bool,
    output_path: PathBuf,
}

impl PreparedTask for PanicPrepared {
    fn output_path(&self) -> &Path {
        &self.output_path
    }

    fn execute(self: Box<Self>, _context: &TaskContext) -> Result<MergeResult, TaskError> {
        assert!(!self.should_panic, "intentional backend panic");
        Ok(MergeResult {
            output_path: self.output_path.clone(),
            root_model_name: "root".to_owned(),
            part_count: 2,
            bone_count: 1,
            mesh_count: 1,
            warnings: Vec::new(),
        })
    }
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        use std::sync::atomic::AtomicU64;
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "model-merger-app-core-scheduler-{}-{}",
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
