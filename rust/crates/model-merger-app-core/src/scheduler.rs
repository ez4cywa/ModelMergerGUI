use crate::paths::identity_key;
use model_merger_engine::{
    MergeError, MergeObserver, MergeRequest, MergeResult, MergeStage, MergeValidationCode,
    PreparedMerge,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const MAXIMUM_CONCURRENCY: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl TaskState {
    fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskProgress {
    pub stage: MergeStage,
    pub current: usize,
    pub total: usize,
    pub item: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskSnapshot {
    pub id: TaskId,
    pub state: TaskState,
    pub progress: Option<TaskProgress>,
    pub result: Option<MergeResult>,
    pub error: Option<TaskError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    Cancelled,
    OutputConflict(PathBuf),
    Validation {
        code: MergeValidationCode,
        message: String,
        path: Option<PathBuf>,
    },
    Io {
        path: PathBuf,
        message: String,
    },
    Codec(String),
    ModelRead {
        path: PathBuf,
        message: String,
    },
    InvalidModel(String),
    Backend(String),
    SchedulerStopped,
    InvalidConcurrency,
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("merge task was cancelled"),
            Self::OutputConflict(path) => {
                write!(formatter, "another task is writing to {}", path.display())
            }
            Self::Validation { message, .. }
            | Self::Codec(message)
            | Self::InvalidModel(message)
            | Self::Backend(message) => formatter.write_str(message),
            Self::Io { path, message } | Self::ModelRead { path, message } => {
                write!(formatter, "{}: {message}", path.display())
            }
            Self::SchedulerStopped => formatter.write_str("merge scheduler has stopped"),
            Self::InvalidConcurrency => {
                write!(
                    formatter,
                    "maximum concurrency must be 1 to {MAXIMUM_CONCURRENCY}"
                )
            }
        }
    }
}

impl std::error::Error for TaskError {}

#[derive(Clone)]
pub struct TaskContext {
    id: TaskId,
    cancelled: Arc<AtomicBool>,
    shared: Arc<Shared>,
}

impl TaskContext {
    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn report(&self, stage: MergeStage, current: usize, total: usize, item: Option<&str>) {
        let mut inner = self.shared.inner.lock().unwrap();
        if let Some(task) = inner.tasks.get_mut(&self.id) {
            task.progress = Some(TaskProgress {
                stage,
                current,
                total,
                item: item.map(str::to_owned),
            });
        }
        self.shared.changed.notify_all();
    }
}

pub trait PreparedTask: Send {
    fn output_path(&self) -> &Path;

    fn execute(self: Box<Self>, context: &TaskContext) -> Result<MergeResult, TaskError>;
}

pub trait MergeBackend: Send + Sync + 'static {
    fn prepare(
        &self,
        request: MergeRequest,
        context: &TaskContext,
    ) -> Result<Box<dyn PreparedTask>, TaskError>;
}

#[derive(Default)]
pub struct NativeMergeBackend;

impl MergeBackend for NativeMergeBackend {
    fn prepare(
        &self,
        request: MergeRequest,
        context: &TaskContext,
    ) -> Result<Box<dyn PreparedTask>, TaskError> {
        let observer = NativeObserver { context };
        model_merger_engine::prepare(request, &observer)
            .map(|prepared| Box::new(NativePrepared(Some(prepared))) as Box<dyn PreparedTask>)
            .map_err(map_engine_error)
    }
}

struct NativePrepared(Option<PreparedMerge>);

impl PreparedTask for NativePrepared {
    fn output_path(&self) -> &Path {
        self.0
            .as_ref()
            .expect("native prepared task should not be consumed")
            .output_path()
    }

    fn execute(mut self: Box<Self>, context: &TaskContext) -> Result<MergeResult, TaskError> {
        let observer = NativeObserver { context };
        self.0
            .take()
            .expect("native prepared task should execute once")
            .execute(&observer)
            .map_err(map_engine_error)
    }
}

struct NativeObserver<'a> {
    context: &'a TaskContext,
}

impl MergeObserver for NativeObserver<'_> {
    fn is_cancelled(&self) -> bool {
        self.context.is_cancelled()
    }

    fn on_progress(&self, stage: MergeStage, current: usize, total: usize, item: Option<&str>) {
        self.context.report(stage, current, total, item);
    }
}

fn map_engine_error(error: MergeError) -> TaskError {
    match error {
        MergeError::Cancelled => TaskError::Cancelled,
        MergeError::Validation {
            code,
            message,
            path,
        } => TaskError::Validation {
            code,
            message,
            path,
        },
        MergeError::Io { path, source } => TaskError::Io {
            path,
            message: source.to_string(),
        },
        MergeError::Codec(source) => TaskError::Codec(source.to_string()),
        MergeError::ModelRead { path, message } => TaskError::ModelRead { path, message },
        MergeError::InvalidModel(message) => TaskError::InvalidModel(message),
    }
}

pub struct TaskScheduler {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
    worker_done: mpsc::Receiver<()>,
    maximum_concurrency: usize,
}

impl TaskScheduler {
    pub fn new<B>(backend: Arc<B>, maximum_concurrency: usize) -> Result<Self, TaskError>
    where
        B: MergeBackend,
    {
        if !(1..=MAXIMUM_CONCURRENCY).contains(&maximum_concurrency) {
            return Err(TaskError::InvalidConcurrency);
        }
        let backend: Arc<dyn MergeBackend> = backend;
        let shared = Arc::new(Shared {
            backend,
            inner: Mutex::new(Inner::default()),
            available: Condvar::new(),
            changed: Condvar::new(),
        });
        let (done_sender, worker_done) = mpsc::channel();
        let workers = (0..maximum_concurrency)
            .map(|_| {
                let shared = Arc::clone(&shared);
                let done_sender = done_sender.clone();
                std::thread::spawn(move || {
                    worker_loop(shared);
                    let _ = done_sender.send(());
                })
            })
            .collect();
        drop(done_sender);
        Ok(Self {
            shared,
            workers,
            worker_done,
            maximum_concurrency,
        })
    }

    pub fn native(maximum_concurrency: usize) -> Result<Self, TaskError> {
        Self::new(Arc::new(NativeMergeBackend), maximum_concurrency)
    }

    pub fn maximum_concurrency(&self) -> usize {
        self.maximum_concurrency
    }

    pub fn schedule(&self, request: MergeRequest) -> Result<TaskId, TaskError> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed));
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut inner = self.shared.inner.lock().unwrap();
        if inner.stopping {
            return Err(TaskError::SchedulerStopped);
        }
        inner.tasks.insert(
            id,
            TaskRecord {
                state: TaskState::Queued,
                cancellation: Arc::clone(&cancellation),
                progress: None,
                result: None,
                error: None,
            },
        );
        inner.queue.push_back(Job {
            id,
            request,
            cancellation,
        });
        self.shared.available.notify_one();
        Ok(id)
    }

    pub fn cancel(&self, id: TaskId) -> bool {
        let mut inner = self.shared.inner.lock().unwrap();
        let Some(task) = inner.tasks.get_mut(&id) else {
            return false;
        };
        if task.state.is_terminal() {
            return false;
        }
        task.cancellation.store(true, Ordering::Release);
        if task.state == TaskState::Queued {
            task.state = TaskState::Cancelled;
            task.error = Some(TaskError::Cancelled);
        }
        self.shared.available.notify_all();
        self.shared.changed.notify_all();
        true
    }

    pub fn snapshot(&self, id: TaskId) -> Option<TaskSnapshot> {
        let inner = self.shared.inner.lock().unwrap();
        inner.tasks.get(&id).map(|task| task.snapshot(id))
    }

    pub fn snapshots(&self) -> Vec<TaskSnapshot> {
        let inner = self.shared.inner.lock().unwrap();
        let mut snapshots: Vec<_> = inner
            .tasks
            .iter()
            .map(|(id, task)| task.snapshot(*id))
            .collect();
        snapshots.sort_by_key(|snapshot| snapshot.id.0);
        snapshots
    }

    pub fn wait(&self, id: TaskId) -> Option<TaskSnapshot> {
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            let task = inner.tasks.get(&id)?;
            if task.state.is_terminal() {
                return Some(task.snapshot(id));
            }
            inner = self.shared.changed.wait(inner).unwrap();
        }
    }

    pub fn wait_for_any(&self, ids: &[TaskId], timeout: Duration) -> Option<TaskSnapshot> {
        let deadline = Instant::now() + timeout;
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            if let Some(snapshot) = ids.iter().find_map(|id| {
                inner
                    .tasks
                    .get(id)
                    .filter(|task| task.state.is_terminal())
                    .map(|task| task.snapshot(*id))
            }) {
                return Some(snapshot);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return None;
            }
            let (guard, wait) = self.shared.changed.wait_timeout(inner, remaining).unwrap();
            inner = guard;
            if wait.timed_out() {
                return ids.iter().find_map(|id| {
                    inner
                        .tasks
                        .get(id)
                        .filter(|task| task.state.is_terminal())
                        .map(|task| task.snapshot(*id))
                });
            }
        }
    }
}

impl Drop for TaskScheduler {
    fn drop(&mut self) {
        {
            let mut inner = self.shared.inner.lock().unwrap();
            inner.stopping = true;
            for task in inner.tasks.values_mut() {
                if !task.state.is_terminal() {
                    task.cancellation.store(true, Ordering::Release);
                    if task.state == TaskState::Queued {
                        task.state = TaskState::Cancelled;
                        task.error = Some(TaskError::Cancelled);
                    }
                }
            }
        }
        self.shared.available.notify_all();
        self.shared.changed.notify_all();
        let deadline = Instant::now() + Duration::from_secs(2);
        for _ in 0..self.workers.len() {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            if self.worker_done.recv_timeout(remaining).is_err() {
                break;
            }
        }
        while let Some(worker) = self.workers.pop() {
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}

struct Shared {
    backend: Arc<dyn MergeBackend>,
    inner: Mutex<Inner>,
    available: Condvar,
    changed: Condvar,
}

#[derive(Default)]
struct Inner {
    queue: VecDeque<Job>,
    tasks: HashMap<TaskId, TaskRecord>,
    claimed_outputs: HashSet<String>,
    stopping: bool,
}

struct Job {
    id: TaskId,
    request: MergeRequest,
    cancellation: Arc<AtomicBool>,
}

struct TaskRecord {
    state: TaskState,
    cancellation: Arc<AtomicBool>,
    progress: Option<TaskProgress>,
    result: Option<MergeResult>,
    error: Option<TaskError>,
}

impl TaskRecord {
    fn snapshot(&self, id: TaskId) -> TaskSnapshot {
        TaskSnapshot {
            id,
            state: self.state,
            progress: self.progress.clone(),
            result: self.result.clone(),
            error: self.error.clone(),
        }
    }
}

fn worker_loop(shared: Arc<Shared>) {
    loop {
        let Some(job) = next_job(&shared) else {
            return;
        };
        let id = job.id;
        if !mark_running(&shared, &job) {
            continue;
        }
        let result = catch_unwind(AssertUnwindSafe(|| execute_job(&shared, job)))
            .unwrap_or_else(|_| Err(TaskError::Backend("merge backend panicked".to_owned())));
        finish_task(&shared, id, result);
    }
}

fn mark_running(shared: &Shared, job: &Job) -> bool {
    let mut inner = shared.inner.lock().unwrap();
    let Some(task) = inner.tasks.get_mut(&job.id) else {
        return false;
    };
    if task.state == TaskState::Cancelled || job.cancellation.load(Ordering::Acquire) {
        task.state = TaskState::Cancelled;
        task.error = Some(TaskError::Cancelled);
        shared.changed.notify_all();
        return false;
    }
    task.state = TaskState::Running;
    shared.changed.notify_all();
    true
}

fn execute_job(shared: &Arc<Shared>, job: Job) -> Result<MergeResult, TaskError> {
    let context = TaskContext {
        id: job.id,
        cancelled: job.cancellation,
        shared: Arc::clone(shared),
    };
    let prepared = shared.backend.prepare(job.request, &context)?;
    if context.is_cancelled() {
        return Err(TaskError::Cancelled);
    }
    let output_path = prepared.output_path().to_path_buf();
    let _claim = OutputClaim::acquire(Arc::clone(shared), output_path)?;
    prepared.execute(&context)
}

struct OutputClaim {
    shared: Arc<Shared>,
    key: String,
}

impl OutputClaim {
    fn acquire(shared: Arc<Shared>, output_path: PathBuf) -> Result<Self, TaskError> {
        let key = output_key(&output_path);
        {
            let mut inner = shared.inner.lock().unwrap();
            if !inner.claimed_outputs.insert(key.clone()) {
                return Err(TaskError::OutputConflict(output_path));
            }
        }
        Ok(Self { shared, key })
    }
}

impl Drop for OutputClaim {
    fn drop(&mut self) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.claimed_outputs.remove(&self.key);
    }
}

fn next_job(shared: &Shared) -> Option<Job> {
    let mut inner = shared.inner.lock().unwrap();
    loop {
        if let Some(job) = inner.queue.pop_front() {
            return Some(job);
        }
        if inner.stopping {
            return None;
        }
        inner = shared.available.wait(inner).unwrap();
    }
}

fn finish_task(shared: &Shared, id: TaskId, result: Result<MergeResult, TaskError>) {
    let mut inner = shared.inner.lock().unwrap();
    if let Some(task) = inner.tasks.get_mut(&id) {
        match result {
            Ok(result) => {
                task.state = TaskState::Succeeded;
                task.result = Some(result);
                task.error = None;
            }
            Err(TaskError::Cancelled) => {
                task.state = TaskState::Cancelled;
                task.error = Some(TaskError::Cancelled);
            }
            Err(error) => {
                task.state = TaskState::Failed;
                task.error = Some(error);
            }
        }
    }
    shared.changed.notify_all();
}

fn output_key(path: &Path) -> String {
    identity_key(path)
}
