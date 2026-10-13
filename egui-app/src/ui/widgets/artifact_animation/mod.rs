//! Standalone trial-artifact playback. Analysis never depends on image availability.
mod cache;
mod data;
pub mod export;
#[cfg(test)]
#[allow(dead_code, reason = "Opt-in main-thread workload harness")]
mod performance;
mod presentation;
#[cfg(test)]
mod tests;

#[cfg(test)]
#[allow(dead_code, reason = "Opt-in main-thread workload harness")]
pub fn performance_check() {
    performance::run();
}
#[cfg(test)]
#[allow(dead_code, reason = "Opt-in main-thread workload harness")]
fn configure_probe(w: &mut ArtifactAnimation) {
    w.display_duration_ms = 100;
}
#[cfg(test)]
#[allow(dead_code, reason = "Opt-in main-thread workload harness")]
fn probe_stats(ctx: &egui::Context) -> String {
    format!("cache={:?}", cache::shared(ctx).lock().unwrap().stats)
}

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
    fn cancelled(self) -> Option<Self> {
        self.cancel.store(true, Ordering::Relaxed);
        if self.poll().is_some() {
            None
        } else {
            Some(self)
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
    pub texture: Arc<egui::TextureHandle>,
    pub prefix: Arc<Prefix>,
}

/// Prefix analysis is independent of image caching and runs off the UI thread.
fn prepare(
    ctx: &egui::Context,
    data: Arc<Data>,
    index: usize,
    cached: Option<Arc<Prefix>>,
) -> Job<Arc<Prefix>> {
    Job::spawn(ctx, move |cancel| {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let frame = &data.frames[index];
        let prefix = cached.unwrap_or_else(|| Arc::new(Prefix::new(&data, frame.number)));
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        Ok(prefix)
    })
}

#[derive(Default)]
struct Runtime {
    key: Option<(usize, u64, u64, usize, Vec<bool>)>,
    mapping: Option<Job<Arc<Data>>>,
    mapping_needed: bool,
    data: Option<Arc<Data>>,
    loading: Option<Job<Arc<Prefix>>>,
    prefix: Option<Arc<Prefix>>,
    images: Option<cache::Client>,
    image_size: [u32; 2],
    sequence: Vec<usize>,
    selection_key: Option<(usize, usize)>,
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
impl Runtime {
    fn seek(&mut self, position: usize) {
        if let Some(&index) = self
            .sequence
            .get(position.min(self.sequence.len().saturating_sub(1)))
        {
            if index != self.cursor {
                self.ready = None;
                self.prefix = None;
            }
            self.cursor = index;
            self.next_tick = None;
        }
    }
    fn next_frame(&self, looping: bool) -> Option<usize> {
        let position = self.sequence.binary_search(&self.cursor).ok()?;
        self.sequence.get(position + 1).copied().or_else(|| {
            if looping {
                self.sequence.first().copied()
            } else {
                None
            }
        })
    }
    fn cancelled_prefix(&mut self) -> Option<Job<Arc<Prefix>>> {
        // A sorter already running cannot be interrupted internally. Retain its
        // slot across source resets until completion instead of spawning more.
        self.loading.take().and_then(Job::cancelled)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ArtifactAnimation {
    pub artifact_index: usize,
    #[serde(deserialize_with = "deserialize_duration")]
    pub display_duration_ms: u32,
    pub skip: usize,
    pub looping: bool,
    pub presentation: Presentation,
    #[serde(skip)]
    runtime: Runtime,
}
fn normalize_duration(value: u32) -> u32 {
    ((value.clamp(20, 10000) + 5) / 10) * 10
}
fn deserialize_duration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    use serde::Deserialize;
    Ok(normalize_duration(u32::deserialize(d)?))
}
impl Default for ArtifactAnimation {
    fn default() -> Self {
        Self {
            artifact_index: 0,
            display_duration_ms: 330,
            skip: 0,
            looping: true,
            presentation: Presentation::default(),
            runtime: Runtime::default(),
        }
    }
}

impl ArtifactAnimation {
    fn sync_sampling(&mut self, count: usize) {
        self.skip = self.skip.min(count.saturating_sub(1));
        let key = (count, self.skip);
        if self.runtime.selection_key == Some(key) {
            return;
        }
        let sequence = export::selected_frames(count, self.skip);
        let position = sequence
            .binary_search(&self.runtime.cursor)
            .unwrap_or_else(|insertion| insertion.saturating_sub(1));
        let cursor = sequence.get(position).copied().unwrap_or(0);
        if cursor != self.runtime.cursor {
            self.runtime.ready = None;
            self.runtime.prefix = None;
        }
        if let Some(job) = &self.runtime.loading {
            job.cancel.store(true, Ordering::Relaxed);
        }
        if let Some(images) = &mut self.runtime.images {
            images.suspend();
        }
        self.runtime.cursor = cursor;
        self.runtime.sequence = sequence;
        self.runtime.selection_key = Some(key);
        self.runtime.next_tick = None;
    }

    pub fn show(&mut self, ui: &mut egui::Ui, app: &AppState) {
        if self.runtime.exporting {
            if let Some(images) = &mut self.runtime.images {
                images.suspend();
            }
            if let Some(job) = &self.runtime.loading {
                job.cancel.store(true, Ordering::Relaxed);
            }
            ui.label("GIF export in progress. Playback and settings are paused.");
            return;
        }
        let Some(study) = app.current_study.as_ref() else {
            let had_source = self.runtime.key.is_some();
            self.runtime = Runtime {
                loading: self.runtime.cancelled_prefix(),
                mapping: self.runtime.mapping.take().and_then(Job::cancelled),
                ..Default::default()
            };
            if had_source {
                cache::shared(ui.ctx()).lock().unwrap().invalidate((
                    0,
                    app.source_generation,
                    app.artifact_revision,
                ));
            }
            ui.label("Select a Study to animate its PNG/JPEG artifacts.");
            return;
        };
        if app.study_streaming {
            let had_source = self.runtime.key.is_some();
            self.runtime = Runtime {
                loading: self.runtime.cancelled_prefix(),
                mapping: self.runtime.mapping.take().and_then(Job::cancelled),
                error: self.runtime.error.take(),
                status: self.runtime.status.take(),
                ..Default::default()
            };
            if had_source {
                cache::shared(ui.ctx()).lock().unwrap().invalidate((
                    0,
                    app.source_generation,
                    app.artifact_revision,
                ));
            }
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
        self.display_duration_ms = normalize_duration(self.display_duration_ms);
        let previous_duration = self.display_duration_ms;
        ui.horizontal_wrapped(|ui| {
            ui.label("Artifact #:");
            ui.add(egui::DragValue::new(&mut self.artifact_index).range(0..=max_index));
            ui.add(
                egui::Slider::new(&mut self.display_duration_ms, 20..=10000)
                    .logarithmic(true)
                    .step_by(10.0)
                    .show_value(false)
                    .text("Display duration"),
            );
            ui.add(
                egui::DragValue::new(&mut self.display_duration_ms)
                    .range(20..=10000)
                    .speed(10.0)
                    .suffix(" ms"),
            );
            self.display_duration_ms = normalize_duration(self.display_duration_ms);
            ui.label(format!(
                "{:.2} FPS",
                1000.0 / self.display_duration_ms as f64
            ));
            ui.checkbox(&mut self.looping, "Loop");
        });
        self.presentation
            .controls(ui, &study.meta.objective_names, &study.meta.param_names);
        if self.display_duration_ms != previous_duration {
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
            cache::shared(ui.ctx())
                .lock()
                .unwrap()
                .invalidate((key.0, key.1, key.2));
            self.runtime = Runtime {
                loading: self.runtime.cancelled_prefix(),
                mapping: self.runtime.mapping.take().and_then(Job::cancelled),
                mapping_needed: true,
                key: Some(key),
                error: self.runtime.error.take(),
                status: self.runtime.status.take(),
                ..Default::default()
            };
        }
        if self.runtime.mapping_needed && self.runtime.mapping.is_none() {
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
            self.runtime.mapping = Some(Job::spawn(ui.ctx(), move |cancel| {
                let frames = data::frame_list(&snapshot, &artifacts, index, cancel);
                if cancel.load(Ordering::Relaxed) {
                    return Err("Cancelled".into());
                }
                Ok(Arc::new(Data::new(snapshot, frames)))
            }));
            self.runtime.mapping_needed = false;
        }
        if let Some(result) = self.runtime.mapping.as_ref().and_then(Job::poll) {
            let cancelled = self
                .runtime
                .mapping
                .as_ref()
                .unwrap()
                .cancel
                .load(Ordering::Relaxed);
            self.runtime.mapping = None;
            match result {
                _ if cancelled => {}
                Ok(data) => self.runtime.data = Some(data),
                Err(e) => self.runtime.error = Some(e),
            }
        }
        let Some(data) = self.runtime.data.clone() else {
            if let Some(error) = &self.runtime.error {
                ui.colored_label(crate::theme::ERROR_COLOR(), error);
            }
            ui.spinner();
            ui.label("Checking PNG/JPEG artifacts…");
            if self.runtime.mapping_needed {
                ui.ctx().request_repaint();
            }
            return;
        };
        if data.frames.is_empty() {
            if let Some(error) = &self.runtime.error {
                ui.colored_label(crate::theme::ERROR_COLOR(), error);
            }
            ui.label("No readable PNG/JPEG artifacts at this Artifact # in the current Study.");
            return;
        }
        self.skip = self.skip.min(data.frames.len().saturating_sub(1));
        ui.horizontal(|ui| {
            ui.label("Frame skip:");
            ui.add(
                egui::DragValue::new(&mut self.skip).range(0..=data.frames.len().saturating_sub(1)),
            )
            .on_hover_text(
                "Skip this many eligible images between frames in both playback and GIF export.",
            );
            let count = export::selected_count(data.frames.len(), self.skip);
            ui.label(format!(
                "{} selected frames · {:.2} s per pass",
                count,
                count as f64 * self.display_duration_ms as f64 / 1000.0
            ));
        });
        self.sync_sampling(data.frames.len());
        let mut position = self
            .runtime
            .sequence
            .binary_search(&self.runtime.cursor)
            .unwrap();
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
                .add_enabled(position > 0, egui::Button::new("Previous"))
                .clicked()
            {
                position -= 1;
                changed = true;
            }
            if ui
                .add_enabled(
                    position + 1 < self.runtime.sequence.len(),
                    egui::Button::new("Next"),
                )
                .clicked()
            {
                position += 1;
                changed = true;
            }
            changed |= ui
                .add(
                    egui::Slider::new(&mut position, 0..=self.runtime.sequence.len() - 1)
                        .text("Frame")
                        .custom_formatter(|v, _| {
                            format!(
                                "Trial {}",
                                data.frames[self.runtime.sequence[v as usize]].number
                            )
                        }),
                )
                .changed();
        });
        if changed {
            self.runtime.seek(position);
            self.runtime.error = None;
        }
        let was_playing = self.runtime.playing;
        self.advance(Instant::now());
        if was_playing && !self.runtime.playing {
            ui.ctx().request_repaint(); // Refresh the transport label after stopping at the end.
        }
        let scope = (
            self.runtime.key.as_ref().unwrap().0,
            app.source_generation,
            app.artifact_revision,
        );
        let layer_scale = ui
            .ctx()
            .layer_transform_to_global(ui.layer_id())
            .map_or(1.0, |t| t.scaling);
        let image_size = cache::display_size(
            presentation::image_box(ui.available_size(), &self.presentation, data.bounds.len()),
            ui.ctx().pixels_per_point() * layer_scale,
            ui.input(|i| i.max_texture_side)
                .min(cache::shared(ui.ctx()).lock().unwrap().maximum_side()),
        );
        if self.runtime.image_size != image_size {
            self.runtime.image_size = image_size;
            self.runtime.ready = None;
        }
        let images = self
            .runtime
            .images
            .get_or_insert_with(|| cache::Client::new(ui.ctx()));
        images.update(
            ui.ctx(),
            data.clone(),
            scope,
            &self.runtime.sequence,
            self.runtime
                .sequence
                .binary_search(&self.runtime.cursor)
                .unwrap(),
            self.runtime.playing,
            self.looping,
            image_size,
        );
        let next_index = self.runtime.next_frame(self.looping);
        let wanted_prefix = if self
            .runtime
            .ready
            .as_ref()
            .is_some_and(|r| r.index == self.runtime.cursor)
        {
            if self.runtime.playing {
                next_index
            } else {
                None
            }
        } else {
            Some(self.runtime.cursor)
        };
        if Some(self.runtime.loading_index) != wanted_prefix {
            if let Some(job) = &self.runtime.loading {
                job.cancel.store(true, Ordering::Relaxed);
            }
        }
        if self.runtime.ready.as_ref().map(|r| r.index) != Some(self.runtime.cursor) {
            self.runtime.ready = None;
            if self.runtime.prefix.as_ref().map(|p| p.number)
                != Some(data.frames[self.runtime.cursor].number)
            {
                self.runtime.prefix = None;
                self.runtime.prefix = self
                    .runtime
                    .cache
                    .get(data.frames[self.runtime.cursor].number);
            }
        }
        if let Some(result) = self.runtime.loading.as_ref().and_then(Job::poll) {
            let cancelled = self
                .runtime
                .loading
                .as_ref()
                .unwrap()
                .cancel
                .load(Ordering::Relaxed);
            self.runtime.loading = None;
            match result {
                _ if cancelled => {}
                Ok(prefix) => {
                    self.runtime.cache.insert(prefix.clone());
                    if self.runtime.loading_index == self.runtime.cursor {
                        self.runtime.prefix = Some(prefix);
                    }
                }
                Err(_) if self.runtime.loading_index != self.runtime.cursor => {}
                Err(e) => {
                    self.runtime.error = Some(e);
                    self.runtime.playing = false;
                }
            }
        }
        if self.runtime.ready.is_none() {
            let result = self
                .runtime
                .images
                .as_ref()
                .unwrap()
                .cache
                .lock()
                .unwrap()
                .get(&data, scope, self.runtime.cursor, image_size);
            match result {
                Ok(Some(texture)) => {
                    if let Some(prefix) = &self.runtime.prefix {
                        self.runtime.ready = Some(ReadyFrame {
                            index: self.runtime.cursor,
                            texture,
                            prefix: prefix.clone(),
                        });
                    }
                }
                Err(error) => {
                    self.runtime.error = Some(error);
                    let mut updated = (*data).clone();
                    updated.frames.remove(self.runtime.cursor);
                    self.runtime.cursor = self
                        .runtime
                        .cursor
                        .min(updated.frames.len().saturating_sub(1));
                    self.runtime.data = Some(Arc::new(updated));
                    self.runtime.selection_key = None;
                    if let Some(job) = &self.runtime.loading {
                        job.cancel.store(true, Ordering::Relaxed);
                    }
                    self.runtime.images = None;
                    self.runtime.prefix = None;
                    self.runtime.next_tick = None;
                    ui.ctx().request_repaint();
                    return;
                }
                Ok(None) => {}
            }
            if self.runtime.ready.is_none() {
                // A real preparation miss freezes the playback clock. Cached,
                // prefetched frames keep the existing cadence; a late frame gets
                // its dwell after it is actually presented, never skipped.
                self.runtime.next_tick = None;
            }
        }
        // A resident image and cached prefix can establish readiness in this
        // pass. Start lookahead now, before sleeping until the playback deadline.
        let target = if self.runtime.ready.is_some() && self.runtime.playing {
            next_index
        } else if self.runtime.prefix.is_none() {
            Some(self.runtime.cursor)
        } else {
            None
        };
        if self.runtime.loading.is_none() {
            if let Some(index) = target {
                if self.runtime.cache.get(data.frames[index].number).is_none() {
                    self.runtime.loading = Some(prepare(ui.ctx(), data.clone(), index, None));
                    self.runtime.loading_index = index;
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
                self.runtime
                    .sequence
                    .binary_search(&self.runtime.cursor)
                    .unwrap()
                    + 1,
                self.runtime.sequence.len()
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
            if self.runtime.playing && self.runtime.next_tick.is_none() {
                self.runtime.next_tick =
                    Some(Instant::now() + Duration::from_millis(self.display_duration_ms as u64));
            }
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
                    selected: self.runtime.sequence.clone(),
                    data,
                    presentation: self.presentation.snapshot(),
                    size,
                    display_duration_ms: self.display_duration_ms,
                    scope,
                    looping: self.looping,
                    path,
                    visuals: ui.visuals().clone(),
                });
                self.runtime.exporting = true;
                self.runtime.error = None;
                self.runtime.status = None;
            }
        }
        if self.runtime.playing {
            if let Some(deadline) = self.runtime.next_tick {
                // Workers wake the UI on completion. Ready playback only needs
                // its next deadline, not 50 idle repaints per second at slow rates.
                ui.ctx()
                    .request_repaint_after(deadline.saturating_duration_since(Instant::now()));
            }
        }
    }

    fn advance(&mut self, now: Instant) {
        if !self.runtime.playing
            || self.runtime.ready.as_ref().map(|r| r.index) != Some(self.runtime.cursor)
        {
            return;
        }
        let next = *self
            .runtime
            .next_tick
            .get_or_insert(now + Duration::from_millis(self.display_duration_ms as u64));
        if now >= next {
            if let Some(index) = self.runtime.next_frame(self.looping) {
                self.runtime.cursor = index;
            } else {
                self.runtime.playing = false;
            }
            // Advance the deadline rather than adding preparation time on every
            // frame. If preparation missed an entire dwell, wait safely and rearm
            // after presenting the frame instead of skipping it.
            let deadline = next + Duration::from_millis(self.display_duration_ms as u64);
            self.runtime.next_tick = (deadline > now).then_some(deadline);
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
