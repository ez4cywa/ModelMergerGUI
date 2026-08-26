use serde::Serialize;
use serde_json::json;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

pub const PROTOCOL_VERSION: u32 = 1;
pub type WorkerOutput = Mutex<BufWriter<std::io::Stdout>>;

pub fn emit_error(output: &WorkerOutput, code: &str, message: &str) {
    emit(
        output,
        &json!({
            "protocol": PROTOCOL_VERSION,
            "event": "error",
            "code": code,
            "message": message,
        }),
    );
}

pub fn emit<T: Serialize>(output: &WorkerOutput, event: &T) {
    let mut output = output
        .lock()
        .expect("worker output lock should not be poisoned");
    serde_json::to_writer(&mut *output, event).expect("worker event should serialize");
    writeln!(output).expect("worker event should write");
    output.flush().expect("worker event should flush");
}
