use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn prepare_then_execute_streams_progress_and_a_result() {
    let fixtures = fixture_directory();
    let output = TestDirectory::new();
    let mut worker = Worker::start();
    worker.send(json!({
        "protocol": 1,
        "command": "prepare",
        "request": {
            "input_files": [fixtures.join("part-00.cast"), fixtures.join("part-01.cast")],
            "output_directory": output.path,
            "output_file_name": "worker.cast",
            "root_selection_mode": "automatic",
            "manual_root_file": null,
            "overwrite": false
        }
    }));
    let prepare_events = worker.read_until("prepared");
    assert!(
        prepare_events
            .iter()
            .any(|event| event["event"] == "progress")
    );
    assert_eq!(
        output.path.join("worker.cast").to_string_lossy(),
        prepare_events.last().unwrap()["output_path"]
            .as_str()
            .unwrap()
    );

    worker.send(json!({ "protocol": 1, "command": "execute" }));
    let execute_events = worker.read_until("result");
    let result = execute_events.last().unwrap();
    assert_eq!("part-00", result["root_model_name"]);
    assert_eq!(2, result["part_count"]);
    assert_eq!(2, result["bone_count"]);
    assert_eq!(2, result["mesh_count"]);
    assert!(output.path.join("worker.cast").exists());
    worker.wait_success();
}

#[test]
fn invalid_protocol_is_returned_as_a_structured_error() {
    let mut worker = Worker::start();
    worker.send(json!({ "protocol": 99, "command": "execute" }));
    let events = worker.read_until("error");
    assert_eq!("protocol", events.last().unwrap()["code"]);
    worker.wait_failure();
}

#[test]
fn cancel_after_prepare_returns_a_structured_cancellation() {
    let fixtures = fixture_directory();
    let output = TestDirectory::new();
    let mut worker = Worker::start();
    worker.send(json!({
        "protocol": 1,
        "command": "prepare",
        "request": {
            "input_files": [fixtures.join("part-00.cast"), fixtures.join("part-01.cast")],
            "output_directory": output.path,
            "output_file_name": "cancelled.cast",
            "root_selection_mode": "automatic",
            "manual_root_file": null,
            "overwrite": false
        }
    }));
    worker.read_until("prepared");

    worker.send(json!({ "protocol": 1, "command": "cancel" }));
    let events = worker.read_until("error");

    assert_eq!("cancelled", events.last().unwrap()["code"]);
    assert!(!output.path.join("cancelled.cast").exists());
    worker.wait_failure();
}

#[test]
fn preview_returns_compact_geometry_payload_and_deletes_it_after_release() {
    let fixtures = fixture_directory();
    let mut worker = Worker::start();
    worker.send(json!({
        "protocol": 1,
        "command": "preview",
        "request": {
            "file_path": fixtures.join("part-00.cast"),
            "triangle_limit": 2
        }
    }));

    let events = worker.read_until("preview_result");
    let result = events.last().unwrap();
    assert_eq!("part-00", result["model_name"]);
    assert_eq!(1, result["source_mesh_count"]);
    assert_eq!(12, result["source_vertex_count"]);
    assert_eq!(4, result["source_triangle_count"]);
    assert_eq!(2, result["displayed_triangle_count"]);
    assert_eq!(true, result["is_simplified"]);
    let payload_path = PathBuf::from(result["payload_path"].as_str().unwrap());
    let payload = std::fs::read(&payload_path).expect("preview payload should exist");
    assert_eq!(MAGIC, &payload[0..4]);
    assert_eq!(
        VERSION,
        u32::from_le_bytes(payload[4..8].try_into().unwrap())
    );
    assert_eq!(1, u32::from_le_bytes(payload[8..12].try_into().unwrap()));
    assert_eq!(payload.len() as u64, result["payload_length"]);

    worker.send(json!({ "protocol": 1, "command": "release_preview" }));
    worker.wait_success();
    assert!(!payload_path.exists());
}

#[test]
fn preview_payload_is_deleted_after_cancel() {
    let mut worker = Worker::start();
    let payload_path = start_preview_and_get_payload(&mut worker);

    worker.send(json!({ "protocol": 1, "command": "cancel" }));
    let events = worker.read_until("error");
    assert_eq!("cancelled", events.last().unwrap()["code"]);
    worker.wait_failure();
    assert!(!payload_path.exists());
}

#[test]
fn preview_payload_is_deleted_when_input_pipe_closes() {
    let mut worker = Worker::start();
    let payload_path = start_preview_and_get_payload(&mut worker);

    worker.close_input();
    worker.wait_success();
    assert!(!payload_path.exists());
}

fn start_preview_and_get_payload(worker: &mut Worker) -> PathBuf {
    let fixtures = fixture_directory();
    worker.send(json!({
        "protocol": 1,
        "command": "preview",
        "request": {
            "file_path": fixtures.join("part-00.cast"),
            "triangle_limit": 2
        }
    }));
    let events = worker.read_until("preview_result");
    PathBuf::from(events.last().unwrap()["payload_path"].as_str().unwrap())
}

fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/rust-migration/golden-small")
}

struct Worker {
    child: Child,
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
}

impl Worker {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_model-merger-worker"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("worker should start");
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input: Some(input),
            output,
        }
    }

    fn send(&mut self, value: Value) {
        let input = self.input.as_mut().expect("worker input should be open");
        serde_json::to_writer(&mut *input, &value).unwrap();
        writeln!(input).unwrap();
        input.flush().unwrap();
    }

    fn close_input(&mut self) {
        self.input.take();
    }

    fn read_until(&mut self, expected_event: &str) -> Vec<Value> {
        let mut result = Vec::new();
        loop {
            let mut line = String::new();
            assert_ne!(
                0,
                self.output.read_line(&mut line).unwrap(),
                "worker closed stdout"
            );
            let event: Value = serde_json::from_str(&line).expect("worker output should be JSON");
            let done = event["event"] == expected_event;
            result.push(event);
            if done {
                return result;
            }
        }
    }

    fn wait_success(&mut self) {
        assert!(self.child.wait().unwrap().success());
    }

    fn wait_failure(&mut self) {
        assert!(!self.child.wait().unwrap().success());
    }
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "model-merger-worker-test-{}-{}",
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
use model_merger_worker::preview_payload::{MAGIC, VERSION};
