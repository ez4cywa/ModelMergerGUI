use std::backtrace::Backtrace;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
static LOG_WRITE: Mutex<()> = Mutex::new(());

pub fn install_panic_hook() -> io::Result<PathBuf> {
    let preferred = log_directory();
    let directory = if fs::create_dir_all(&preferred).is_ok() {
        preferred
    } else {
        std::env::temp_dir().join("CastModelMerger").join("logs")
    };
    install_in(&directory)?;
    Ok(directory)
}

fn install_in(directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let _ = LOG_DIRECTORY.set(directory.to_path_buf());
    std::panic::set_hook(Box::new(|panic_info| {
        let thread = std::thread::current();
        let details = format!(
            "unhandled panic: {panic_info}\nthread={:?} name={:?}\nbacktrace:\n{}",
            thread.id(),
            thread.name(),
            Backtrace::force_capture()
        );
        let _ = append("panic", &details);
    }));
    #[cfg(windows)]
    native_crash::install(&directory.join("CastModelMerger.log"))?;
    append(
        "startup",
        &format!(
            "version={} commit={} os={} arch={} executable={:?}",
            env!("CARGO_PKG_VERSION"),
            env!("CAST_BUILD_COMMIT"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::env::current_exe().ok()
        ),
    )
}

pub fn record_startup_error(error: &str) {
    let _ = append("startup-error", error);
}

pub fn record_runtime_error(area: &str, error: &str) {
    let _ = append(area, error);
}

pub fn record_event(area: &str, detail: &str) {
    // Unit tests do not pollute the user's application log unless explicitly initialized.
    if LOG_DIRECTORY.get().is_some() {
        let _ = append(area, detail);
    }
}

pub fn log_directory() -> PathBuf {
    if let Some(directory) = LOG_DIRECTORY.get() {
        return directory.clone();
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("CastModelMerger")
        .join("logs")
}

fn append(kind: &str, message: &str) -> io::Result<()> {
    let directory = LOG_DIRECTORY.get().cloned().unwrap_or_else(log_directory);
    fs::create_dir_all(&directory)?;
    // Do not deadlock if a panic occurs while another thread is writing a log.
    let guard = LOG_WRITE.try_lock();
    let file_name = if guard.is_ok() {
        "CastModelMerger.log"
    } else {
        "CastModelMerger-concurrent.log"
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join(file_name))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let record = format!(
        "[{timestamp}] pid={} {kind}: {message}\n",
        std::process::id()
    );
    file.write_all(record.as_bytes())?;
    file.flush()?;
    if matches!(kind, "panic" | "fatal" | "startup-error") {
        file.sync_data()?;
    }
    Ok(())
}

#[cfg(windows)]
mod native_crash {
    use super::*;
    use std::ffi::c_void;
    use std::fmt::Write as _;
    use std::os::windows::io::IntoRawHandle;
    use std::sync::atomic::{AtomicPtr, Ordering};

    static LOG_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

    #[repr(C)]
    struct ExceptionRecord {
        code: u32,
        flags: u32,
        next: *mut ExceptionRecord,
        address: *mut c_void,
        parameter_count: u32,
        parameters: [usize; 15],
    }
    #[repr(C)]
    struct ExceptionPointers {
        record: *const ExceptionRecord,
        context: *const c_void,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetUnhandledExceptionFilter(
            filter: Option<unsafe extern "system" fn(*const ExceptionPointers) -> i32>,
        ) -> *const c_void;
        fn WriteFile(
            handle: *mut c_void,
            buffer: *const u8,
            size: u32,
            written: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn FlushFileBuffers(handle: *mut c_void) -> i32;
        fn GetCurrentThreadId() -> u32;
        fn GetCurrentProcessId() -> u32;
    }

    struct StackMessage {
        bytes: [u8; 256],
        len: usize,
    }
    impl std::fmt::Write for StackMessage {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let count = value.len().min(self.bytes.len() - self.len);
            self.bytes[self.len..self.len + count].copy_from_slice(&value.as_bytes()[..count]);
            self.len += count;
            Ok(())
        }
    }

    unsafe extern "system" fn on_exception(info: *const ExceptionPointers) -> i32 {
        let handle = LOG_HANDLE.load(Ordering::Relaxed);
        if !handle.is_null() && !info.is_null() {
            // SAFETY: Windows supplies valid exception pointers during this callback.
            let record = unsafe { (*info).record };
            if !record.is_null() {
                let mut message = StackMessage {
                    bytes: [0; 256],
                    len: 0,
                };
                // No heap allocation, Rust locks, or backtrace unwinding on a damaged process.
                unsafe {
                    let _ = writeln!(
                        &mut message,
                        "\nnative-exception pid={} thread={} code=0x{:08X} address={:p}",
                        GetCurrentProcessId(),
                        GetCurrentThreadId(),
                        (*record).code,
                        (*record).address
                    );
                    let mut written = 0;
                    WriteFile(
                        handle,
                        message.bytes.as_ptr(),
                        message.len as u32,
                        &mut written,
                        std::ptr::null_mut(),
                    );
                    FlushFileBuffers(handle);
                }
            }
        }
        0 // EXCEPTION_CONTINUE_SEARCH: preserve Windows' normal crash handling.
    }

    pub(super) fn install(path: &Path) -> io::Result<()> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        // Kept open until process exit so fatal exceptions need not open or allocate anything.
        LOG_HANDLE.store(file.into_raw_handle(), Ordering::Relaxed);
        // SAFETY: the callback and its pre-opened handle live for the process lifetime.
        unsafe {
            SetUnhandledExceptionFilter(Some(on_exception));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_directory_has_a_stable_product_suffix() {
        assert!(log_directory().ends_with(PathBuf::from("CastModelMerger").join("logs")));
    }

    #[test]
    fn crash_child_writes_context_and_panic_backtrace() {
        let log = run_child_probe("panic");
        assert!(log.contains("startup: version="));
        assert!(log.contains("preview-open: id=7"));
        assert!(log.contains("diagnostic panic probe"));
        assert!(log.contains("backtrace:"));
        assert!(!log.contains("disabled backtrace"));
        assert!(log.contains("pid="));
    }

    #[cfg(windows)]
    #[test]
    fn crash_child_writes_native_exception_without_rust_unwind() {
        let log = run_child_probe("native");
        assert!(log.contains("native-exception pid="));
        assert!(log.contains("code=0xE0424D4D"));
        assert!(log.contains("address=0x"));
        assert!(log.contains("preview-open: id=7"));
    }

    fn run_child_probe(mode: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("cast-log-probe-{}-{unique}", std::process::id()));
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "diagnostics::tests::child_log_probe",
                "--ignored",
                "--nocapture",
            ])
            .env("CAST_LOG_PROBE_MODE", mode)
            .env("CAST_LOG_PROBE_DIRECTORY", &directory)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let started = std::time::Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if started.elapsed() > std::time::Duration::from_secs(15) {
                child.kill().unwrap();
                child.wait().unwrap();
                let _ = fs::remove_dir_all(&directory);
                panic!("crash logging child timed out");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let log = fs::read_to_string(directory.join("CastModelMerger.log")).unwrap();
        fs::remove_dir_all(&directory).unwrap();
        assert!(!status.success(), "probe must really terminate abnormally");
        log
    }

    #[test]
    #[ignore = "subprocess-only crash probe; invoked by the parent regression tests"]
    fn child_log_probe() {
        let directory = PathBuf::from(
            std::env::var_os("CAST_LOG_PROBE_DIRECTORY").expect("child probe directory"),
        );
        install_in(&directory).unwrap();
        record_event("preview-open", "id=7 path=\"test.cast\"");
        let mode = std::env::var("CAST_LOG_PROBE_MODE").unwrap();
        #[cfg(windows)]
        if mode == "native" {
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn SetErrorMode(mode: u32) -> u32;
                fn RaiseException(code: u32, flags: u32, count: u32, arguments: *const usize);
            }
            // SAFETY: intentionally terminate only this isolated test subprocess.
            unsafe {
                SetErrorMode(3); // No system crash dialog in the unattended test.
                RaiseException(0xE0424D4D, 1, 0, std::ptr::null());
            }
            panic!("native probe unexpectedly returned");
        }
        assert_eq!(mode, "panic");
        panic!("diagnostic panic probe");
    }
}
