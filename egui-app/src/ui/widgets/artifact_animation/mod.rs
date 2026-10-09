//! Standalone trial-artifact playback. Analysis never depends on image availability.
mod data;
pub mod export;
mod presentation;
#[cfg(test)]
mod tests;

#[cfg(test)]
#[allow(
    dead_code,
    reason = "Called only by the opt-in main-thread native capture harness"
)]
pub fn native_capture_check() {
    tests::native_capture_check();
}

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

use crate::state::app_state::AppState;
use data::{Data, Prefix, PrefixCache};
use presentation::Presentation;

/// A single bounded asynchronous result. Dropping a job invalidates its completion.
pub(super) struct Job<T> {
    rx: mpsc::Receiver<Result<T, String>>,
    cancel: Arc<AtomicBool>,
}
impl<T: Send + 'static> Job<T> {
    fn spawn(
        ctx: &egui::Context,
        work: impl FnOnce(&AtomicBool) -> Result<T, String> + Send + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&flag)))
                .unwrap_or_else(|_| Err("Artifact Animation worker failed".into()));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        Self { rx, cancel }
    }
    fn poll(&self) -> Option<Result<T, String>> {
        match self.rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Artifact Animation worker disconnected".into()))
            }
        }
    }
}
impl<T> Drop for Job<T> {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

struct ReadyFrame {
    pub index: usize,
    pub texture: egui::TextureHandle,
    pub prefix: Arc<Prefix>,
}

/// Use the same image dependency and native egui Image/texture path as the gallery,
/// but decode off-thread. Polling egui's globally locked URI loaders from a worker
/// can deadlock against Context::has_pending_images on the UI thread.
fn prepare(
    ctx: &egui::Context,
    data: Arc<Data>,
    index: usize,
    cached: Option<Arc<Prefix>>,
) -> Job<ReadyFrame> {
    let context = ctx.clone();
    Job::spawn(ctx, move |cancel| {
        let frame = &data.frames[index];
        let image = data::read_image(&frame.entry)?;
        let side = context
            .input(|i| i.max_texture_side)
            .max(1)
            .min(u32::MAX as usize) as u32;
        let image = if image.width() > side || image.height() > side {
            image.thumbnail(side, side)
        } else {
            image
        };
        let image = image.into_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
        let prefix = cached.unwrap_or_else(|| Arc::new(Prefix::new(&data, frame.number)));
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let texture = context.load_texture(
            "artifact-animation-frame",
            image,
            egui::TextureOptions::LINEAR,
        );
        // Owned TextureHandle drops free old-frame textures without evicting the
        // gallery's independently cached URI or retaining every animation frame.
        Ok(ReadyFrame {
            index,
            texture,
            prefix,
        })
    })
}

#[derive(Default)]
struct Runtime {
    key: Option<(usize, u64, u64, usize, Vec<bool>)>,
    mapping: Option<Job<Arc<Data>>>,
    data: Option<Arc<Data>>,
    loading: Option<Job<ReadyFrame>>,
    loading_index: usize,
    ready: Option<ReadyFrame>,
    cache: PrefixCache,
    cursor: usize,
    playing: bool,
    next_tick: Option<Instant>,
    error: Option<String>,
    status: Option<String>,
    export_request: Option<export::Request>,
    exporting: bool,
    #[cfg(test)]
    content_size: Option<egui::Vec2>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ArtifactAnimation {
    pub artifact_index: usize,
    pub fps: u32,
    pub looping: bool,
    pub presentation: Presentation,
    #[serde(skip)]
    runtime: Runtime,
}
impl Default for ArtifactAnimation {
    fn default() -> Self {
        Self {
            artifact_index: 0,
            fps: 3,
            looping: true,
            presentation: Presentation::default(),
            runtime: Runtime::default(),
        }
    }
}

impl ArtifactAnimation {
    pub fn show(&mut self, ui: &mut egui::Ui, app: &AppState) {
        if self.runtime.exporting {
            ui.label("GIF export in progress. Playback and settings are paused.");
            return;
        }
        let Some(study) = app.current_study.as_ref() else {
            self.runtime = Runtime::default();
            ui.label("Select a Study to animate its PNG/JPEG artifacts.");
            return;
        };
        if app.study_streaming {
            self.runtime = Runtime {
                error: self.runtime.error.take(),
                status: self.runtime.status.take(),
                ..Default::default()
            };
            ui.label("Loading Study… Animation becomes available after loading completes.");
            return;
        }
        let max_index = study
            .view
            .trial_ids
            .iter()
            .filter_map(|id| app.artifact_map.get(id))
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .saturating_sub(1);
        self.artifact_index = self.artifact_index.min(max_index);
        self.fps = self.fps.clamp(1, 10);
        let previous_fps = self.fps;
        ui.horizontal_wrapped(|ui| {
            ui.label("Artifact #:");
            ui.add(egui::DragValue::new(&mut self.artifact_index).range(0..=max_index));
            ui.label("FPS:");
            egui::ComboBox::from_id_salt("animation_fps")
                .selected_text(self.fps.to_string())
                .show_ui(ui, |ui| {
                    for fps in [1, 3, 5, 10] {
                        ui.selectable_value(&mut self.fps, fps, fps.to_string());
                    }
                });
            ui.checkbox(&mut self.looping, "Loop");
        });
        self.presentation
            .controls(ui, &study.meta.objective_names, &study.meta.param_names);
        if self.fps != previous_fps {
            self.runtime.next_tick = None;
        }
        let directions = study
            .meta
            .objective_names
            .iter()
            .enumerate()
            .map(|(i, _)| {
                !matches!(
                    study.meta.directions.get(i),
                    Some(crate::state::types::Direction::Maximize)
                )
            })
            .collect();
        let key = (
            Arc::as_ptr(&study.view.df) as usize,
            app.source_generation,
            app.artifact_revision,
            self.artifact_index,
            directions,
        );
        if self.runtime.key.as_ref() != Some(&key) {
            let snapshot = study.clone();
            let artifacts = study
                .view
                .trial_ids
                .iter()
                .filter_map(|id| {
                    app.artifact_map
                        .get(id)
                        .map(|entries| (*id, entries.clone()))
                })
                .collect();
            let index = self.artifact_index;
            self.runtime = Runtime {
                key: Some(key),
                error: self.runtime.error.take(),
                status: self.runtime.status.take(),
                mapping: Some(Job::spawn(ui.ctx(), move |cancel| {
                    let frames = data::frame_list(&snapshot, &artifacts, index, cancel);
                    Ok(Arc::new(Data::new(snapshot, frames)))
                })),
                ..Default::default()
            };
        }
        if let Some(result) = self.runtime.mapping.as_ref().and_then(Job::poll) {
            self.runtime.mapping = None;
            match result {
                Ok(data) => self.runtime.data = Some(data),
                Err(e) => self.runtime.error = Some(e),
            }
        }
        let Some(data) = self.runtime.data.clone() else {
            ui.spinner();
            ui.label("Checking PNG/JPEG artifacts…");
            return;
        };
        if data.frames.is_empty() {
            if let Some(error) = &self.runtime.error {
                ui.colored_label(crate::theme::ERROR_COLOR(), error);
            }
            ui.label("No readable PNG/JPEG artifacts at this Artifact # in the current Study.");
            return;
        }
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if self.runtime.playing {
                    "Pause"
                } else {
                    "Play"
                })
                .clicked()
            {
                self.runtime.playing = !self.runtime.playing;
                self.runtime.next_tick = None;
                self.runtime.error = None;
            }
            if ui
                .add_enabled(self.runtime.cursor > 0, egui::Button::new("Previous"))
                .clicked()
            {
                self.runtime.cursor -= 1;
                changed = true;
            }
            if ui
                .add_enabled(
                    self.runtime.cursor + 1 < data.frames.len(),
                    egui::Button::new("Next"),
                )
                .clicked()
            {
                self.runtime.cursor += 1;
                changed = true;
            }
            changed |= ui
                .add(
                    egui::Slider::new(&mut self.runtime.cursor, 0..=data.frames.len() - 1)
                        .text("Frame")
                        .custom_formatter(|v, _| {
                            format!("Trial {}", data.frames[v as usize].number)
                        }),
                )
                .changed();
        });
        if changed {
            self.runtime.next_tick = None;
            self.runtime.error = None;
        }
        self.advance(Instant::now(), data.frames.len());
        if self.runtime.ready.as_ref().map(|r| r.index) != Some(self.runtime.cursor)
            && self.runtime.loading.is_none()
        {
            self.runtime.ready = None;
            let cached = self
                .runtime
                .cache
                .get(data.frames[self.runtime.cursor].number);
            self.runtime.loading =
                Some(prepare(ui.ctx(), data.clone(), self.runtime.cursor, cached));
            self.runtime.loading_index = self.runtime.cursor;
        }
        if let Some(result) = self.runtime.loading.as_ref().and_then(Job::poll) {
            self.runtime.loading = None;
            match result {
                Ok(ready) if ready.index == self.runtime.cursor => {
                    self.runtime.cache.insert(ready.prefix.clone());
                    self.runtime.ready = Some(ready);
                    self.runtime.next_tick = None;
                }
                Ok(_) => {}
                Err(_) if self.runtime.loading_index != self.runtime.cursor => {}
                Err(e) => {
                    self.runtime.error = Some(e);
                    // Remove only the unavailable frame, not its analysis result.
                    let mut updated = (*data).clone();
                    updated.frames.remove(self.runtime.cursor);
                    self.runtime.cursor = self
                        .runtime
                        .cursor
                        .min(updated.frames.len().saturating_sub(1));
                    self.runtime.data = Some(Arc::new(updated));
                    self.runtime.next_tick = None;
                    ui.ctx().request_repaint();
                    return;
                }
            }
        }
        let size = egui::vec2(
            ui.available_width().max(1.0),
            ui.available_height().max(1.0),
        );
        let mut export = false;
        ui.horizontal(|ui| {
            ui.label(format!(
                "Trial {} · Frame {} of {}",
                data.frames[self.runtime.cursor].number,
                self.runtime.cursor + 1,
                data.frames.len()
            ));
            export = ui
                .add_enabled(
                    self.runtime.ready.is_some(),
                    egui::Button::new("Export GIF"),
                )
                .clicked();
        });
        if let Some(error) = &self.runtime.error {
            ui.colored_label(crate::theme::ERROR_COLOR(), error);
        }
        if let Some(status) = &self.runtime.status {
            ui.label(status);
        }
        let size = egui::vec2(size.x, ui.available_height().max(1.0));
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        #[cfg(test)]
        {
            self.runtime.content_size = Some(size);
        }
        let mut content_ready = false;
        if let Some(ready) = &self.runtime.ready {
            content_ready = presentation::content(ui, rect, &data, ready, &self.presentation);
        } else {
            ui.put(
                rect,
                egui::Label::new("Loading image and trial-prefix plots…"),
            );
        }
        if export {
            if !content_ready {
                self.runtime.error = Some("Enlarge the widget before exporting GIF.".into());
                return;
            }
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name("artifact-animation.gif")
                .add_filter("Animated GIF", &["gif"])
                .save_file()
            {
                self.runtime.export_request = Some(export::Request {
                    data,
                    presentation: self.presentation.snapshot(),
                    size,
                    fps: self.fps,
                    looping: self.looping,
                    path,
                    visuals: ui.visuals().clone(),
                });
                self.runtime.exporting = true;
                self.runtime.error = None;
                self.runtime.status = None;
            }
        }
        if self.runtime.playing || self.runtime.loading.is_some() || self.runtime.mapping.is_some()
        {
            ui.ctx().request_repaint_after(Duration::from_millis(20));
        }
    }

    fn advance(&mut self, now: Instant, len: usize) {
        if !self.runtime.playing
            || self.runtime.ready.as_ref().map(|r| r.index) != Some(self.runtime.cursor)
        {
            return;
        }
        let next = self
            .runtime
            .next_tick
            .get_or_insert(now + Duration::from_secs_f64(1.0 / self.fps as f64));
        if now >= *next {
            if self.runtime.cursor + 1 < len {
                self.runtime.cursor += 1;
            } else if self.looping {
                self.runtime.cursor = 0;
            } else {
                self.runtime.playing = false;
            }
            self.runtime.next_tick = None;
        }
    }
    pub(crate) fn take_export(&mut self) -> Option<export::Request> {
        self.runtime.export_request.take()
    }
    pub(crate) fn finish_export(&mut self, result: Result<bool, String>) {
        self.runtime.exporting = false;
        self.runtime.next_tick = None;
        match result {
            Ok(true) => self.runtime.status = Some("GIF exported successfully.".into()),
            Ok(false) => {
                self.runtime.status = Some("GIF export cancelled; destination unchanged.".into())
            }
            Err(error) => self.runtime.error = Some(error),
        }
    }
}
