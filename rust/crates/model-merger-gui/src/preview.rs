use crate::{preview_gpu, theme};
use eframe::egui::{self, RichText, Sense};
use model_merger_app_core::{Catalog, TextKey};
use model_merger_engine::PreviewError;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::JoinHandle;

const PREVIEW_TRIANGLE_LIMIT: usize = 250_000;

pub struct PreviewSession {
    pub id: u64,
    pub open: bool,
    pub title: String,
    state: PreviewLoadState,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    show_grid: bool,
    show_info: bool,
    cancelled: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

enum PreviewLoadState {
    Loading(Receiver<Result<LoadedPreview, PreviewError>>),
    Ready(LoadedPreview),
    Failed(PreviewFailure),
}

enum PreviewFailure {
    Engine(PreviewError),
    WorkerStopped,
}

struct LoadedPreview {
    summary: PreviewSummary,
    gpu_model: Arc<preview_gpu::PreviewModel>,
}

struct PreviewSummary {
    model_name: String,
    source_mesh_count: usize,
    source_vertex_count: usize,
    source_triangle_count: usize,
    dimensions: [f32; 3],
    displayed_triangle_count: usize,
    is_simplified: bool,
}

impl PreviewSession {
    pub fn load(id: u64, path: &Path) -> Self {
        let title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let path = path.to_path_buf();
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        let worker = std::thread::spawn(move || {
            let result = model_merger_engine::load_preview(&path, PREVIEW_TRIANGLE_LIMIT, || {
                worker_cancelled.load(Ordering::Acquire)
            })
            .map(|data| {
                let gpu_model = Arc::new(preview_gpu::PreviewModel::from_preview(&data));
                let summary = PreviewSummary {
                    model_name: data.model_name,
                    source_mesh_count: data.source_mesh_count,
                    source_vertex_count: data.source_vertex_count,
                    source_triangle_count: data.source_triangle_count,
                    dimensions: std::array::from_fn(|index| {
                        data.bounds.maximum[index] - data.bounds.minimum[index]
                    }),
                    displayed_triangle_count: data.displayed_triangle_count,
                    is_simplified: data.is_simplified,
                };
                LoadedPreview { summary, gpu_model }
            });
            let _ = sender.send(result);
        });
        Self {
            id,
            open: true,
            title,
            state: PreviewLoadState::Loading(receiver),
            yaw: -0.55,
            pitch: -0.35,
            zoom: 1.0,
            show_grid: true,
            show_info: false,
            cancelled,
            worker: Some(worker),
        }
    }

    pub fn path_title(path: &Path, prefix: &str) -> String {
        format!("{prefix} — {}", short_name(path))
    }

    pub fn show(&mut self, ui: &mut egui::Ui, catalog: Catalog) {
        if ui.input(|input| {
            input.viewport().close_requested() || input.key_pressed(egui::Key::Escape)
        }) {
            self.open = false;
            return;
        }
        self.receive_preview();
        crate::chrome::show_with_title(ui, catalog.language(), catalog.text(TextKey::Preview));
        let palette = theme::palette(ui);
        egui::Panel::top(egui::Id::new(("preview-name", self.id)))
            .frame(
                egui::Frame::new()
                    .fill(palette.preview_canvas)
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(catalog.text(TextKey::PreviewName))
                            .size(13.0)
                            .color(palette.primary),
                    );
                    ui.add(egui::Label::new(RichText::new(&self.title).size(13.0)).truncate())
                        .on_hover_text(&self.title);
                });
            });
        let ready = matches!(self.state, PreviewLoadState::Ready(_));
        egui::Panel::bottom(egui::Id::new(("preview-tabs", self.id)))
            .frame(
                egui::Frame::new()
                    .fill(palette.toolbar)
                    .stroke(egui::Stroke::new(1.0, palette.border))
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                ui.spacing_mut().interact_size.y = 28.0;
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            !self.show_info,
                            RichText::new(catalog.text(TextKey::PreviewViewport)).color(
                                if self.show_info {
                                    palette.foreground
                                } else {
                                    palette.on_primary
                                },
                            ),
                        )
                        .clicked()
                    {
                        self.show_info = false;
                    }
                    if ui
                        .add_enabled(
                            ready,
                            egui::Button::selectable(
                                self.show_info,
                                RichText::new(catalog.text(TextKey::PreviewInfo)).color(
                                    if self.show_info {
                                        palette.on_primary
                                    } else {
                                        palette.foreground
                                    },
                                ),
                            ),
                        )
                        .clicked()
                    {
                        self.show_info = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(catalog.text(TextKey::Close)).clicked() {
                            self.open = false;
                        }
                        if let PreviewLoadState::Ready(loaded) = &self.state {
                            let status = format!(
                                "{} {} · {} {}",
                                loaded.summary.source_mesh_count,
                                catalog.text(TextKey::Meshes),
                                loaded.summary.displayed_triangle_count,
                                catalog.text(TextKey::Triangles)
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&status).size(12.0).color(palette.secondary),
                                )
                                .truncate(),
                            )
                            .on_hover_text(status);
                        }
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(palette.preview_canvas))
            .show(ui, |ui| {
                if self.show_info && ready {
                    self.model_information(ui, catalog);
                    return;
                }
                let (response, painter) = ui.allocate_painter(
                    ui.available_size().max(egui::vec2(1.0, 1.0)),
                    Sense::click_and_drag(),
                );
                painter.rect_filled(response.rect, 0.0, palette.preview_canvas);
                match &self.state {
                    PreviewLoadState::Ready(loaded) => {
                        let model = Arc::clone(&loaded.gpu_model);
                        let simplified = loaded.summary.is_simplified;
                        self.interact_with_view(ui, &response);
                        painter.add(preview_gpu::paint_callback(
                            response.rect,
                            self.id,
                            model,
                            preview_gpu::ViewSettings {
                                yaw: self.yaw,
                                pitch: self.pitch,
                                zoom: self.zoom,
                                show_grid: self.show_grid,
                                model_color: palette.preview_model,
                                grid_color: palette.secondary,
                                axis_color: palette.primary,
                            },
                        ));
                        if simplified {
                            let text = catalog.text(TextKey::PreviewSimplified);
                            let top = egui::Rect::from_min_size(
                                response.rect.left_top() + egui::vec2(16.0, 0.0),
                                egui::vec2((response.rect.width() - 32.0).max(1.0), 50.0),
                            );
                            ui.scope_builder(egui::UiBuilder::new().max_rect(top), |ui| {
                                ui.label(RichText::new(text).size(12.0).color(palette.secondary));
                            });
                        }
                    }
                    PreviewLoadState::Loading(_) => {
                        self.state_message(
                            ui,
                            response.rect,
                            catalog.text(TextKey::PreviewLoading),
                            true,
                            false,
                        );
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(30));
                    }
                    PreviewLoadState::Failed(failure) => {
                        let message = match failure {
                            PreviewFailure::Engine(error) => {
                                crate::messages::preview_error(catalog, error)
                            }
                            PreviewFailure::WorkerStopped => {
                                catalog.text(TextKey::PreviewWorkerStopped).to_owned()
                            }
                        };
                        self.state_message(ui, response.rect, &message, false, true);
                    }
                }
                self.view_tools(ui, response.rect, catalog, ready);
            });
        crate::chrome::resize_edges(ui);
    }

    fn reset_view(&mut self) {
        self.yaw = -0.55;
        self.pitch = -0.35;
        self.zoom = 1.0;
    }

    fn interact_with_view(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.clicked() {
            response.request_focus();
        }
        if response.dragged() {
            let delta = ui.input(|input| input.pointer.delta());
            self.yaw += delta.x * 0.01;
            self.pitch = (self.pitch + delta.y * 0.01).clamp(-1.45, 1.45);
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                self.zoom = (self.zoom * (1.0 + scroll * 0.0015)).clamp(0.2, 8.0);
            }
        }
        // Focused buttons keep their arrow-key navigation; the viewport gets camera keys.
        if !response.has_focus() && ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        ui.input(|input| {
            if input.key_pressed(egui::Key::ArrowLeft) {
                self.yaw -= 0.12;
            }
            if input.key_pressed(egui::Key::ArrowRight) {
                self.yaw += 0.12;
            }
            if input.key_pressed(egui::Key::ArrowUp) {
                self.pitch = (self.pitch - 0.12).max(-1.45);
            }
            if input.key_pressed(egui::Key::ArrowDown) {
                self.pitch = (self.pitch + 0.12).min(1.45);
            }
            if input.key_pressed(egui::Key::Plus) || input.key_pressed(egui::Key::Equals) {
                self.zoom = (self.zoom * 1.15).min(8.0);
            }
            if input.key_pressed(egui::Key::Minus) {
                self.zoom = (self.zoom / 1.15).max(0.2);
            }
            if input.key_pressed(egui::Key::R) {
                self.reset_view();
            }
            if input.key_pressed(egui::Key::G) {
                self.show_grid = !self.show_grid;
            }
        });
    }

    fn view_tools(&mut self, ui: &mut egui::Ui, rect: egui::Rect, catalog: Catalog, ready: bool) {
        use crate::preview_controls::{Icon, button};
        let palette = theme::palette(ui);
        egui::Area::new(egui::Id::new(("preview-tools", self.id)))
            .order(egui::Order::Foreground)
            .movable(false)
            .fixed_pos(egui::pos2(rect.left() + 12.0, rect.bottom() - 54.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(palette.toolbar)
                    .stroke(egui::Stroke::new(1.0, palette.border))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.horizontal(|ui| {
                            ui.add_enabled_ui(ready, |ui| {
                                ui.horizontal(|ui| {
                                    if button(
                                        ui,
                                        Icon::Reset,
                                        catalog.text(TextKey::ResetView),
                                        false,
                                    )
                                    .clicked()
                                    {
                                        self.reset_view();
                                    }
                                    if button(
                                        ui,
                                        Icon::Grid,
                                        catalog.text(TextKey::PreviewGrid),
                                        self.show_grid,
                                    )
                                    .clicked()
                                    {
                                        self.show_grid = !self.show_grid;
                                    }
                                    ui.separator();
                                    if button(
                                        ui,
                                        Icon::Left,
                                        catalog.text(TextKey::RotateLeft),
                                        false,
                                    )
                                    .clicked()
                                    {
                                        self.yaw -= 0.18;
                                    }
                                    if button(
                                        ui,
                                        Icon::Right,
                                        catalog.text(TextKey::RotateRight),
                                        false,
                                    )
                                    .clicked()
                                    {
                                        self.yaw += 0.18;
                                    }
                                    ui.separator();
                                    if button(
                                        ui,
                                        Icon::ZoomOut,
                                        catalog.text(TextKey::ZoomOut),
                                        false,
                                    )
                                    .clicked()
                                    {
                                        self.zoom = (self.zoom / 1.15).max(0.2);
                                    }
                                    if button(
                                        ui,
                                        Icon::ZoomIn,
                                        catalog.text(TextKey::ZoomIn),
                                        false,
                                    )
                                    .clicked()
                                    {
                                        self.zoom = (self.zoom * 1.15).min(8.0);
                                    }
                                });
                            });
                        });
                    });
            });
    }

    fn state_message(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        message: &str,
        loading: bool,
        failed: bool,
    ) {
        let palette = theme::palette(ui);
        let width = (rect.width() - 64.0).clamp(1.0, 480.0);
        let message_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(width, 100.0));
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(message_rect)
                .layout(egui::Layout::top_down(egui::Align::Center)),
            |ui| {
                if loading {
                    ui.spinner();
                }
                ui.label(RichText::new(message).size(15.0).color(if failed {
                    palette.destructive
                } else {
                    palette.secondary
                }));
            },
        );
    }

    fn model_information(&self, ui: &mut egui::Ui, catalog: Catalog) {
        let PreviewLoadState::Ready(loaded) = &self.state else {
            return;
        };
        let palette = theme::palette(ui);
        egui::Frame::new()
            .inner_margin(egui::Margin::same(20))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new(catalog.text(TextKey::PreviewInfo)).size(17.0));
                    ui.add_space(16.0);
                    let summary = &loaded.summary;
                    for (label, value) in [
                        (TextKey::PreviewName, summary.model_name.clone()),
                        (TextKey::PreviewFile, self.title.clone()),
                        (TextKey::Meshes, summary.source_mesh_count.to_string()),
                        (
                            TextKey::PreviewVertices,
                            summary.source_vertex_count.to_string(),
                        ),
                        (
                            TextKey::Triangles,
                            summary.source_triangle_count.to_string(),
                        ),
                        (
                            TextKey::PreviewDisplayedTriangles,
                            summary.displayed_triangle_count.to_string(),
                        ),
                        (
                            TextKey::PreviewDimensions,
                            format!(
                                "X {:.3} · Y {:.3} · Z {:.3}",
                                summary.dimensions[0], summary.dimensions[1], summary.dimensions[2]
                            ),
                        ),
                    ] {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(catalog.text(label)).color(palette.secondary));
                            ui.label(value);
                        });
                        ui.add_space(8.0);
                    }
                    ui.separator();
                    ui.label(
                        RichText::new(catalog.text(TextKey::PreviewInstructions))
                            .size(13.0)
                            .color(palette.secondary),
                    );
                    if summary.is_simplified {
                        ui.label(
                            RichText::new(catalog.text(TextKey::PreviewSimplified))
                                .size(13.0)
                                .color(palette.secondary),
                        );
                    }
                });
            });
    }

    fn receive_preview(&mut self) {
        let PreviewLoadState::Loading(receiver) = &self.state else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(data)) => {
                self.finish_worker();
                self.state = PreviewLoadState::Ready(data);
            }
            Ok(Err(error)) => {
                self.finish_worker();
                let detail = error.to_string();
                crate::diagnostics::record_runtime_error("preview-error", &detail);
                self.state = PreviewLoadState::Failed(PreviewFailure::Engine(error));
            }
            Err(TryRecvError::Disconnected) => {
                self.finish_worker();
                self.state = PreviewLoadState::Failed(PreviewFailure::WorkerStopped);
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn finish_worker(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for PreviewSession {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        // Dropping a JoinHandle detaches the cancellable loader. Never wait for disk I/O or
        // decoding on the GUI thread when a preview window is dismissed.
        self.worker.take();
    }
}

fn short_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_session() -> PreviewSession {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/rust-migration/golden-small/part-00.cast");
        let data =
            model_merger_engine::load_preview(&path, PREVIEW_TRIANGLE_LIMIT, || false).unwrap();
        PreviewSession {
            id: 1,
            open: true,
            title: "long_sample_model_name_".repeat(12) + ".cast",
            state: PreviewLoadState::Ready(LoadedPreview {
                gpu_model: Arc::new(preview_gpu::PreviewModel::from_preview(&data)),
                summary: PreviewSummary {
                    model_name: data.model_name,
                    source_mesh_count: data.source_mesh_count,
                    source_vertex_count: data.source_vertex_count,
                    source_triangle_count: data.source_triangle_count,
                    displayed_triangle_count: data.displayed_triangle_count,
                    is_simplified: false,
                    dimensions: std::array::from_fn(|index| {
                        data.bounds.maximum[index] - data.bounds.minimum[index]
                    }),
                },
            }),
            yaw: -0.55,
            pitch: -0.35,
            zoom: 1.0,
            show_grid: true,
            show_info: false,
            cancelled: Arc::new(AtomicBool::new(false)),
            worker: None,
        }
    }

    fn frame(
        context: &egui::Context,
        preview: &mut PreviewSession,
        catalog: Catalog,
        size: egui::Vec2,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ui| preview.show(ui, catalog),
        )
    }

    #[test]
    fn localized_preview_layout_and_controls_fit_all_states_and_themes() {
        for language in model_merger_app_core::AppLanguage::ALL {
            for dark in [false, true] {
                for size in [egui::vec2(640.0, 480.0), egui::vec2(900.0, 700.0)] {
                    for state in 0..4 {
                        let context = egui::Context::default();
                        theme::configure(&context, language);
                        context.set_theme(if dark {
                            egui::ThemePreference::Dark
                        } else {
                            egui::ThemePreference::Light
                        });
                        let catalog = Catalog::new(language);
                        let mut preview = ready_session();
                        let (_sender, receiver) = mpsc::channel();
                        match state {
                            1 => preview.show_info = true,
                            2 => {
                                preview.state =
                                    PreviewLoadState::Failed(PreviewFailure::WorkerStopped)
                            }
                            3 => preview.state = PreviewLoadState::Loading(receiver),
                            _ => {}
                        }
                        for _ in 0..3 {
                            frame(&context, &mut preview, catalog, size, vec![])
                                .drop_without_applying_deltas();
                        }
                        context.enable_accesskit();
                        let mut output = frame(&context, &mut preview, catalog, size, vec![]);
                        output.textures_delta.clear();
                        for (rect, text, clip) in theme::review_text(&output.shapes) {
                            if clip.intersects(rect) {
                                assert!(
                                    rect.left() >= -1.0 && rect.right() <= size.x + 1.0,
                                    "{language:?} state {state}: overflow {text}"
                                );
                            }
                        }
                        let nodes = &output
                            .platform_output
                            .accesskit_update
                            .as_ref()
                            .unwrap()
                            .nodes;
                        let keys: &[TextKey] = if state == 1 {
                            &[TextKey::Close]
                        } else {
                            &[
                                TextKey::ResetView,
                                TextKey::PreviewGrid,
                                TextKey::RotateLeft,
                                TextKey::RotateRight,
                                TextKey::ZoomIn,
                                TextKey::ZoomOut,
                                TextKey::Close,
                            ]
                        };
                        for key in keys {
                            let (_, node) = nodes
                                .iter()
                                .find(|(_, node)| node.label() == Some(catalog.text(*key)))
                                .unwrap_or_else(|| {
                                    panic!("Missing accessible control: {language:?} {key:?}")
                                });
                            let rect = node.bounds().unwrap();
                            assert!(
                                rect.x0 >= 0.0
                                    && rect.x1 <= f64::from(size.x)
                                    && rect.y0 >= 0.0
                                    && rect.y1 <= f64::from(size.y),
                                "Control clipped: {key:?}"
                            );
                        }
                        output.drop_without_applying_deltas();
                    }
                }
            }
        }
    }

    #[test]
    fn preview_tools_and_tabs_have_real_pointer_actions() {
        let context = egui::Context::default();
        let catalog = Catalog::new(model_merger_app_core::AppLanguage::English);
        theme::configure(&context, catalog.language());
        context.enable_accesskit();
        let mut preview = ready_session();
        let size = egui::vec2(640.0, 480.0);
        let mut bounds = std::collections::HashMap::<String, egui::Pos2>::new();
        let mut tick = |preview: &mut PreviewSession, events: Vec<egui::Event>| {
            let output = frame(&context, preview, catalog, size, events);
            if let Some(update) = &output.platform_output.accesskit_update {
                for (_, node) in &update.nodes {
                    if let (Some(label), Some(rect)) = (node.label(), node.bounds()) {
                        // Prefer bottom Close over the title-bar Close for this test.
                        let position = egui::pos2(
                            ((rect.x0 + rect.x1) / 2.0) as f32,
                            ((rect.y0 + rect.y1) / 2.0) as f32,
                        );
                        if label != catalog.text(TextKey::Close)
                            || bounds
                                .get(label)
                                .is_none_or(|previous| previous.y < position.y)
                        {
                            bounds.insert(label.to_owned(), position);
                        }
                    }
                }
            }
            output.drop_without_applying_deltas();
            bounds.clone()
        };
        for _ in 0..3 {
            tick(&mut preview, vec![]);
        }
        let mut click = |preview: &mut PreviewSession, key: TextKey| {
            let bounds = tick(preview, vec![]);
            let position = *bounds
                .get(catalog.text(key))
                .unwrap_or_else(|| panic!("No target: {key:?}"));
            tick(preview, vec![egui::Event::PointerMoved(position)]);
            for pressed in [true, false] {
                tick(
                    preview,
                    vec![egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    }],
                );
            }
        };
        click(&mut preview, TextKey::PreviewGrid);
        assert!(!preview.show_grid);
        click(&mut preview, TextKey::RotateLeft);
        assert!(preview.yaw < -0.55);
        click(&mut preview, TextKey::ZoomIn);
        assert!(preview.zoom > 1.0);
        click(&mut preview, TextKey::ResetView);
        assert_eq!(
            (preview.yaw, preview.pitch, preview.zoom),
            (-0.55, -0.35, 1.0)
        );
        click(&mut preview, TextKey::PreviewInfo);
        assert!(preview.show_info);
        click(&mut preview, TextKey::PreviewViewport);
        assert!(!preview.show_info);
        click(&mut preview, TextKey::Close);
        assert!(!preview.open);
    }

    #[test]
    fn preview_title_keeps_the_selected_file_name() {
        assert_eq!(
            "Preview — body.cast",
            PreviewSession::path_title(Path::new("C:/models/body.cast"), "Preview")
        );
    }

    #[test]
    fn dropping_a_loading_preview_never_waits_for_the_worker() {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(400));
            drop(sender);
        });
        let session = PreviewSession {
            id: 1,
            open: true,
            title: "slow.cast".to_owned(),
            state: PreviewLoadState::Loading(receiver),
            yaw: 0.0,
            pitch: 0.0,
            zoom: 1.0,
            show_grid: true,
            show_info: false,
            cancelled: Arc::new(AtomicBool::new(false)),
            worker: Some(worker),
        };
        let started = std::time::Instant::now();

        drop(session);

        assert!(started.elapsed() < std::time::Duration::from_millis(100));
    }
}
