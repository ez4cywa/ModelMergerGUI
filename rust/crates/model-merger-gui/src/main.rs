#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use model_merger_gui::{NativeApp, startup_viewport};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::panic::AssertUnwindSafe;
use std::path::Path;

fn main() {
    let log_directory = model_merger_gui::diagnostics::install_panic_hook()
        .unwrap_or_else(|_| model_merger_gui::diagnostics::log_directory());
    let result = std::panic::catch_unwind(AssertUnwindSafe(run));
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => report_startup_failure(&log_directory, &error.to_string()),
        Err(_) => report_startup_failure(
            &log_directory,
            "The application stopped unexpectedly during startup.",
        ),
    }
}

fn run() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("application icon should be valid PNG data");
    let options = eframe::NativeOptions {
        viewport: startup_viewport()
            .with_app_id("io.github.echo000.cast-model-merger")
            .with_icon(icon),
        depth_buffer: 24,
        multisampling: model_merger_gui::GPU_SAMPLE_COUNT,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Cast Model Merger",
        options,
        Box::new(|creation| Ok(Box::new(NativeApp::new(creation)))),
    )
}

fn report_startup_failure(log_directory: &Path, details: &str) {
    model_merger_gui::diagnostics::record_startup_error(details);
    let message = format!(
        "Cast 模型合并器无法启动。\n请更新显卡驱动后重试。是否打开诊断日志文件夹？\n\nCast Model Merger could not start.\nUpdate the graphics driver and try again. Open the diagnostics folder?\n\n{}",
        log_directory.display()
    );
    let result = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("Cast Model Merger")
        .set_description(message)
        .set_buttons(MessageButtons::YesNo)
        .show();
    if result == MessageDialogResult::Yes {
        let _ = std::process::Command::new("explorer.exe")
            .arg(log_directory)
            .spawn();
    }
}
