//! Protected sequential screenshot capture, with one frame in flight at a time.
use super::{
    data::{Data, PrefixCache},
    prepare,
    presentation::{self, Presentation},
    Job, ReadyFrame,
};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

pub(crate) struct Request {
    pub(super) data: Arc<Data>,
    pub(super) presentation: Presentation,
    pub(super) size: egui::Vec2,
    pub(super) fps: u32,
    pub(super) looping: bool,
    pub(super) path: std::path::PathBuf,
    pub(super) visuals: egui::Visuals,
}

#[cfg(test)]
mod tests {
    use super::super::data::{Frame, Prefix};
    use super::*;

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1000.0, 800.0),
            )),
            events,
            ..Default::default()
        }
    }
    fn fixture(ctx: &egui::Context, controller: &mut ExportController, path: std::path::PathBuf) {
        let study = super::super::tests::study(
            &[(70, 0, vec![1.0], vec![])],
            vec![crate::state::types::Direction::Minimize],
            false,
        );
        let data = Arc::new(Data::new(
            study,
            vec![Frame {
                row: 0,
                number: 0,
                entry: crate::io::artifacts::ArtifactEntry {
                    path: "unused.png".into(),
                    filename: "unused.png".into(),
                    mimetype: "image/png".into(),
                },
            }],
        ));
        let mut request = Some(Request {
            data: data.clone(),
            presentation: Presentation::default(),
            size: egui::vec2(400.0, 300.0),
            fps: 5,
            looping: true,
            path,
            visuals: egui::Visuals::light(),
        });
        let _ = ctx.run_ui(input(vec![]), |ui| {
            if let Some(request) = request.take() {
                controller.start(ui.ctx(), 11, request, 1.0);
            }
        });
        controller.session.as_mut().unwrap().ready = Some(ReadyFrame {
            index: 0,
            texture: ctx.load_texture(
                "fixture",
                egui::ColorImage::filled([2, 2], egui::Color32::BLUE),
                egui::TextureOptions::LINEAR,
            ),
            prefix: Arc::new(Prefix::new(&data, 0)),
        });
        for _ in 0..4 {
            let _ = ctx.run_ui(input(vec![]), |ui| controller.show(ui.ctx()));
            if controller.session.as_ref().unwrap().pending.is_some() {
                break;
            }
        }
        assert!(controller.session.as_ref().unwrap().pending.is_some());
    }
    fn finish(ctx: &egui::Context, controller: &mut ExportController) -> Result<bool, String> {
        let start = Instant::now();
        loop {
            let _ = ctx.run_ui(input(vec![]), |ui| {
                controller.logic(ui.ctx());
                controller.show(ui.ctx());
            });
            if let Some((owner, result)) = controller.take_completed() {
                assert_eq!(owner, 11);
                return result;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn screenshot_requires_matching_export_and_frame_acknowledgment() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.gif");
        let mut controller = ExportController::default();
        fixture(&ctx, &mut controller, path.clone());
        let token = controller.session.as_ref().unwrap().pending.unwrap().0;
        let image = Arc::new(egui::ColorImage::filled([1000, 800], egui::Color32::BLUE));
        let wrong = CaptureToken {
            export: token.export,
            frame: token.frame + 1,
        };
        let _ = ctx.run_ui(
            input(vec![egui::Event::Screenshot {
                viewport_id: egui::ViewportId::ROOT,
                user_data: egui::UserData::new(wrong),
                image: image.clone(),
            }]),
            |ui| controller.logic(ui.ctx()),
        );
        assert_eq!(
            controller.session.as_ref().unwrap().pending.unwrap().0,
            token
        );
        assert!(!path.exists());
        let _ = ctx.run_ui(
            input(vec![egui::Event::Screenshot {
                viewport_id: egui::ViewportId::ROOT,
                user_data: egui::UserData::new(token),
                image,
            }]),
            |ui| controller.logic(ui.ctx()),
        );
        assert!(finish(&ctx, &mut controller).unwrap());
        let image = image::open(&path).unwrap().into_rgba8();
        assert_eq!(image.dimensions(), (400, 300));
        assert_eq!(image.get_pixel(10, 10).0, [0, 0, 255, 255]);
    }

    #[test]
    fn cancel_and_capture_timeout_clear_session_without_publishing() {
        for timeout in [false, true] {
            let ctx = egui::Context::default();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("out.gif");
            std::fs::write(&path, b"previous").unwrap();
            let mut controller = ExportController::default();
            fixture(&ctx, &mut controller, path.clone());
            let session = controller.session.as_mut().unwrap();
            if timeout {
                session.pending.as_mut().unwrap().1 = Instant::now() - Duration::from_secs(11);
            } else {
                session.cancel.store(true, Ordering::Release);
            }
            let result = finish(&ctx, &mut controller);
            if timeout {
                assert!(result.unwrap_err().contains("timed out"));
            } else {
                assert!(!result.unwrap());
            }
            assert!(!controller.active());
            assert_eq!(std::fs::read(&path).unwrap(), b"previous");
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CaptureToken {
    export: u64,
    frame: usize,
}

enum Encoded {
    Frame(usize),
    Finished(Result<bool, String>),
}

struct Session {
    owner: u64,
    id: u64,
    request: Request,
    scale: f32,
    pixels_per_point: f32,
    screen: egui::Rect,
    rect: Option<egui::Rect>,
    index: usize,
    ready: Option<ReadyFrame>,
    job: Option<Job<ReadyFrame>>,
    crop: Option<Job<image::RgbaImage>>,
    cache: PrefixCache,
    rendered: usize,
    pending: Option<(CaptureToken, Instant)>,
    encoding: bool,
    tx: mpsc::SyncSender<image::RgbaImage>,
    commit: mpsc::SyncSender<()>,
    rx: mpsc::Receiver<Encoded>,
    cancel: Arc<AtomicBool>,
    error: Option<String>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

#[derive(Default)]
pub struct ExportController {
    session: Option<Session>,
    /// Returned to the initiating instance, including paths that never started.
    completed: Option<(u64, Result<bool, String>)>,
    last_error: Option<String>,
}

impl ExportController {
    pub(crate) fn active(&self) -> bool {
        self.session.is_some()
    }
    pub(crate) fn start(
        &mut self,
        ctx: &egui::Context,
        owner: u64,
        mut request: Request,
        scale: f32,
    ) {
        request.presentation = request.presentation.snapshot();
        if self.active() {
            self.completed = Some((owner, Err("Another GIF export is in progress".into())));
            return;
        }
        self.last_error = None;
        let screen = ctx.content_rect();
        let physical = request.size * scale;
        if physical.x + 32.0 > screen.width()
            || physical.y + 80.0 > screen.height()
            || physical.min_elem() <= 0.0
            || request.size.min_elem() <= 24.0
        {
            self.completed = Some((owner, Err("The content is too large for protected capture. Resize the widget or zoom out before exporting.".into())));
            return;
        }
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let (tx, frames) = mpsc::sync_channel(1);
        let (ack, rx) = mpsc::channel();
        let (commit, approval) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let path = request.path.clone();
        let count = request.data.frames.len();
        let fps = request.fps;
        let looping = request.looping;
        let context = ctx.clone();
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                crate::io::chart_capture::encode_gif_stream(
                    &path,
                    frames,
                    count,
                    fps,
                    looping,
                    &flag,
                    |index| {
                        let _ = ack.send(Encoded::Frame(index));
                        context.request_repaint();
                        // Publication requires the UI's final acknowledgment, so
                        // cancellation/capture errors cannot race the last frame.
                        if index + 1 == count {
                            while !flag.load(Ordering::Acquire) {
                                match approval.recv_timeout(Duration::from_millis(50)) {
                                    Ok(()) => break,
                                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                                        flag.store(true, Ordering::Release);
                                        break;
                                    }
                                }
                            }
                        }
                    },
                )
            }))
            .unwrap_or_else(|_| Err("GIF encoder worker failed".into()));
            let _ = ack.send(Encoded::Finished(result));
            context.request_repaint();
        });
        self.session = Some(Session {
            owner,
            id,
            request,
            scale,
            pixels_per_point: ctx.pixels_per_point(),
            screen,
            rect: None,
            index: 0,
            ready: None,
            job: None,
            crop: None,
            cache: PrefixCache::default(),
            rendered: 0,
            pending: None,
            encoding: false,
            tx,
            commit,
            rx,
            cancel,
            error: None,
        });
    }

    pub(crate) fn take_completed(&mut self) -> Option<(u64, Result<bool, String>)> {
        let completed = self.completed.take();
        if let Some((_, Err(error))) = &completed {
            self.last_error = Some(error.clone());
        }
        completed
    }

    pub(crate) fn logic(&mut self, ctx: &egui::Context) {
        let Some(session) = &mut self.session else {
            return;
        };
        let mut finished = None;
        if let Some(result) = session.crop.as_ref().and_then(Job::poll) {
            session.crop = None;
            if !session.cancel.load(Ordering::Acquire) {
                match result {
                    Ok(frame) => {
                        if let Err(e) = session.tx.try_send(frame) {
                            session.error = Some(format!("GIF encoder unavailable: {e}"));
                        }
                    }
                    Err(error) => session.error = Some(error),
                }
            }
        }
        while let Ok(message) = session.rx.try_recv() {
            match message {
                Encoded::Frame(index) if index == session.index => {
                    session.encoding = false;
                    session.index += 1;
                    session.ready = None;
                    session.rendered = 0;
                    if session.index == session.request.data.frames.len()
                        && session.error.is_none()
                        && !session.cancel.load(Ordering::Acquire)
                    {
                        if let Err(error) = session.commit.try_send(()) {
                            session.error = Some(format!("GIF finalization unavailable: {error}"));
                        }
                    }
                }
                Encoded::Frame(_) => {
                    session.error = Some("GIF encoder acknowledged the wrong frame".into());
                }
                Encoded::Finished(result) => {
                    finished = Some(result);
                    break;
                }
            }
        }
        if let Some((token, requested)) = session.pending {
            let screenshot = ctx.input(|i| {
                i.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot {
                        image, user_data, ..
                    } = event
                    {
                        if user_data
                            .data
                            .as_ref()
                            .and_then(|d| d.downcast_ref::<CaptureToken>())
                            == Some(&token)
                        {
                            return Some(image.clone());
                        }
                    }
                    None
                })
            });
            if let Some(image) = screenshot {
                session.pending = None;
                if !session.cancel.load(Ordering::Acquire) {
                    let rect = session.rect.expect("capture rect recorded before request");
                    let scale = session.pixels_per_point;
                    session.encoding = true;
                    session.crop = Some(Job::spawn(ctx, move |_| {
                        let frame = crate::io::chart_capture::crop_image(&image, rect, scale)
                            .ok_or("GIF capture rectangle is outside the viewport")?;
                        let expected = rect.size() * scale;
                        if (frame.width() as f32 - expected.x).abs() > 1.0
                            || (frame.height() as f32 - expected.y).abs() > 1.0
                        {
                            return Err("GIF capture rectangle was clipped".into());
                        }
                        Ok(frame)
                    }));
                }
            } else if requested.elapsed() > Duration::from_secs(10) {
                session.error = Some(
                    "Screenshot capture timed out; this graphics backend may not support capture"
                        .into(),
                );
            }
        }
        if session.error.is_some() {
            session.cancel.store(true, Ordering::Release);
        }
        if let Some(result) = finished {
            let owner = session.owner;
            let result = session.error.take().map_or(result, Err);
            self.session = None;
            self.completed = Some((owner, result));
        }
    }

    /// Drawn last, above canvas, side panels and other dialogs. The modal prevents
    /// interaction with the capture composition, but progress/cancel keep repainting.
    pub(crate) fn show(&mut self, ctx: &egui::Context) {
        let Some(s) = &mut self.session else {
            if let Some(error) = &self.last_error {
                let mut open = true;
                egui::Window::new("GIF export error")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        ui.label(error);
                    });
                if !open {
                    self.last_error = None;
                }
            }
            return;
        };
        if s.index < s.request.data.frames.len()
            && (ctx.content_rect() != s.screen || ctx.pixels_per_point() != s.pixels_per_point)
        {
            s.error = Some(
                "Window dimensions or display scale changed during GIF export. Please retry."
                    .into(),
            );
            s.cancel.store(true, Ordering::Release);
        }
        if s.index < s.request.data.frames.len()
            && s.ready.is_none()
            && s.job.is_none()
            && !s.cancel.load(Ordering::Acquire)
        {
            let data = s.request.data.clone();
            let cached = s.cache.get(data.frames[s.index].number);
            s.job = Some(prepare(ctx, data, s.index, cached));
        }
        if let Some(result) = s.job.as_ref().and_then(Job::poll) {
            s.job = None;
            match result {
                Ok(frame) => {
                    s.cache.insert(frame.prefix.clone());
                    s.ready = Some(frame);
                }
                Err(e) => {
                    s.error = Some(e);
                    s.cancel.store(true, Ordering::Release);
                }
            }
        }
        let size = s.request.size * s.scale;
        let mut rect = None;
        egui::Modal::new(egui::Id::new("artifact-animation-export"))
            .area(
                egui::Area::new(egui::Id::new("artifact-animation-export"))
                    .order(egui::Order::Tooltip)
                    .fade_in(false)
                    .fixed_pos(s.screen.min + egui::vec2(16.0, 16.0))
                    .constrain(false),
            )
            .show(ctx, |ui| {
                ui.visuals_mut().clone_from(&s.request.visuals);
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Exporting GIF: {} / {}",
                        s.index.min(s.request.data.frames.len()),
                        s.request.data.frames.len()
                    ));
                    let can_cancel = s.index < s.request.data.frames.len();
                    let cancel_clicked = ui
                        .add_enabled(can_cancel, egui::Button::new("Cancel"))
                        .clicked();
                    if can_cancel
                        && (cancel_clicked || ui.input(|i| i.key_pressed(egui::Key::Escape)))
                    {
                        s.cancel.store(true, Ordering::Release);
                    }
                });
                if let Some(error) = &s.error {
                    ui.label(error);
                }
                let (capture, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                rect = Some(capture);
                if let Some(ready) = &s.ready {
                    let layer = egui::LayerId::new(
                        egui::Order::Tooltip,
                        egui::Id::new("animation-snapshot-content"),
                    );
                    let world = egui::Rect::from_min_size(egui::Pos2::ZERO, s.request.size);
                    let transform = egui::emath::TSTransform::new(capture.min.to_vec2(), s.scale);
                    ctx.set_transform_layer(layer, transform);
                    // Separate layer preserves the canvas zoom without scaling controls.
                    let mut content =
                        ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(world));
                    content.set_clip_rect(world);
                    presentation::content(
                        &mut content,
                        world,
                        &s.request.data,
                        ready,
                        &s.request.presentation,
                    );
                    ctx.move_to_top(layer);
                } else {
                    ui.put(
                        capture,
                        egui::Label::new(if s.index == s.request.data.frames.len() {
                            "Finalizing GIF…"
                        } else {
                            "Preparing image and prefix…"
                        }),
                    );
                }
            });
        if s.index < s.request.data.frames.len()
            && s.rect.is_some()
            && s.rect != rect
            && s.error.is_none()
        {
            s.error = Some("GIF capture layout changed during export".into());
            s.cancel.store(true, Ordering::Release);
        }
        s.rect = rect;
        if s.ready.is_some()
            && !s.encoding
            && s.pending.is_none()
            && !s.cancel.load(Ordering::Acquire)
        {
            // First render uploads the texture and settles egui layout. Capture only
            // after a second complete render of the *same* prepared frame.
            s.rendered += 1;
            if s.rendered >= 2 {
                let token = CaptureToken {
                    export: s.id,
                    frame: s.index,
                };
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                    token,
                )));
                s.pending = Some((token, Instant::now()));
            }
        }
        ctx.request_repaint_after(Duration::from_millis(20));
    }
}
