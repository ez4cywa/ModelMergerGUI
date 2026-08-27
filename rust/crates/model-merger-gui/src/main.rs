#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use model_merger_gui::{NativeApp, startup_viewport};

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("application icon should be valid PNG data");
    let options = eframe::NativeOptions {
        viewport: startup_viewport()
            .with_app_id("io.github.echo000.cast-model-merger")
            .with_icon(icon),
        depth_buffer: 24,
        multisampling: 4,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Cast Model Merger",
        options,
        Box::new(|creation| Ok(Box::new(NativeApp::new(&creation.egui_ctx)))),
    )
}
