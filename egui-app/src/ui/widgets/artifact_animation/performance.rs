//! Opt-in production-path probe; inputs are read-only and never printed or copied.
use super::*;
use crate::state::types::{StudyContext, StudyView};

fn number(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
fn workload() -> (AppState, Option<tempfile::TempDir>) {
    let mut app = AppState::new();
    if let Some(path) = std::env::var_os("TUNNY_ANIMATION_JOURNAL") {
        let path = std::path::PathBuf::from(path);
        let bytes = std::fs::read(&path).unwrap();
        let meta = tdashboard_io::journal::parser::scan_study_list(&bytes)
            .unwrap()
            .remove(0);
        let projection = crate::test_support::journal(&bytes, meta.study_id);
        let df = projection.dataframe;
        let count = df.row_count();
        app.current_study = Some(StudyContext {
            meta: crate::io::journal::convert_projection_meta(projection.meta),
            view: StudyView::new(df, vec![0; count]),
            pareto_indices: vec![],
        });
        let root = std::path::PathBuf::from(
            std::env::var_os("TUNNY_ANIMATION_ARTIFACTS").expect("artifact root"),
        );
        app.artifact_map = tdashboard_io::artifacts::resolve_from_metadata(
            &root,
            &tdashboard_io::artifacts::parse_artifact_metadata(&path),
        );
        (app, None)
    } else {
        let dir = tempfile::Builder::new()
            .prefix("animation-probe-")
            .tempdir_in(std::env::temp_dir().join("opencode"))
            .unwrap();
        let count = number("TUNNY_ANIMATION_COUNT", 1000);
        let specs: Vec<_> = (0..count)
            .map(|i| (i as u32, i as u32, vec![(i % 19) as f64], vec![]))
            .collect();
        app.current_study = Some(super::tests::study(
            &specs,
            vec![crate::state::types::Direction::Minimize],
            false,
        ));
        for i in 0..count {
            let path = dir.path().join(format!("{i}.png"));
            image::RgbImage::from_fn(656, 614, |x, y| {
                image::Rgb([
                    (x.wrapping_mul(17) + i as u32) as u8,
                    (y * 13) as u8,
                    (x ^ y) as u8,
                ])
            })
            .save(&path)
            .unwrap();
            app.artifact_map.insert(
                i as u32,
                vec![crate::io::artifacts::ArtifactEntry {
                    path,
                    filename: format!("{i}.png"),
                    mimetype: "image/png".into(),
                }],
            );
        }
        (app, Some(dir))
    }
}
struct Probe {
    app: AppState,
    widgets: Vec<ArtifactAnimation>,
    _files: Option<tempfile::TempDir>,
    size: egui::Vec2,
    count: usize,
    pass: usize,
    position: usize,
    setup: Instant,
    started: Option<Instant>,
    requested: Instant,
    waits: Vec<f64>,
    intervals: Vec<f64>,
    last: Option<Instant>,
    cpu_ms: f64,
    stalls: usize,
    passes: usize,
    done: bool,
    native: bool,
    continuous: bool,
}
impl Probe {
    fn new(native: bool) -> Self {
        let (app, files) = workload();
        let widgets = (0..number("TUNNY_ANIMATION_INSTANCES", 1))
            .map(|_| {
                let mut w = ArtifactAnimation::default();
                super::configure_probe(&mut w);
                w.presentation.history = true;
                w
            })
            .collect();
        Self {
            app,
            widgets,
            _files: files,
            size: egui::vec2(
                number("TUNNY_ANIMATION_WIDTH", 700) as f32,
                number("TUNNY_ANIMATION_HEIGHT", 650) as f32,
            ),
            count: number("TUNNY_ANIMATION_COUNT", 1000),
            pass: 0,
            position: 0,
            setup: Instant::now(),
            started: None,
            requested: Instant::now(),
            waits: vec![],
            intervals: vec![],
            last: None,
            cpu_ms: 0.0,
            stalls: 0,
            passes: 0,
            done: false,
            native,
            continuous: std::env::var_os("TUNNY_ANIMATION_PERF_CONTINUOUS").is_some(),
        }
    }
    fn draw(&mut self, ui: &mut egui::Ui) {
        for (i, w) in self.widgets.iter_mut().enumerate() {
            egui::Window::new(format!("Animation {}", i + 1))
                .fixed_pos(egui::pos2(i as f32 * (self.size.x + 10.0), 0.0))
                .fixed_size(self.size)
                .resizable(false)
                .show(ui.ctx(), |ui| w.show(ui, &self.app));
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui) {
        let cpu = Instant::now();
        self.draw(ui);
        self.cpu_ms += cpu.elapsed().as_secs_f64() * 1000.0;
        self.passes += 1;
        if self.widgets.iter().any(|w| {
            w.runtime
                .ready
                .as_ref()
                .is_none_or(|r| r.index != w.runtime.cursor)
        }) {
            self.stalls += 1;
        } else {
            let now = Instant::now();
            if self.started.is_none() {
                println!(
                    "PROBE mapping+first-ready {:.3} ms",
                    self.setup.elapsed().as_secs_f64() * 1000.0
                );
                let source = data::read_image(
                    &self.widgets[0].runtime.data.as_ref().unwrap().frames[0].entry,
                )
                .unwrap();
                println!(
                    "PROBE eligible={} first source dimensions={}x{}",
                    self.widgets[0].runtime.data.as_ref().unwrap().frames.len(),
                    source.width(),
                    source.height()
                );
                self.count = self
                    .count
                    .min(self.widgets[0].runtime.data.as_ref().unwrap().frames.len());
                for w in &mut self.widgets {
                    let mut d = (**w.runtime.data.as_ref().unwrap()).clone();
                    d.frames.truncate(self.count);
                    w.runtime.data = Some(Arc::new(d));
                }
                self.started = Some(now);
                self.requested = now;
                for w in &mut self.widgets {
                    w.runtime.playing = true;
                }
                // The probe activates Play after drawing, unlike a transport
                // click inside show(). One startup pass arms the widget clocks.
                ui.ctx().request_repaint();
            }
            let cursor = self.widgets[0].runtime.cursor;
            if !self.native || self.last.is_none() || cursor != self.position {
                if self.native && cursor != self.position {
                    self.position = cursor;
                }
                self.waits
                    .push(now.duration_since(self.requested).as_secs_f64() * 1000.0);
                if let Some(last) = self.last {
                    self.intervals
                        .push(now.duration_since(last).as_secs_f64() * 1000.0);
                }
                self.last = Some(now);
                if self.position + 1 == self.count {
                    self.report(ui.ctx());
                    self.pass += 1;
                    if self.pass == 2 {
                        self.done = true;
                    }
                    self.position = if self.native { self.count - 1 } else { 0 };
                    self.started = Some(now);
                    self.waits.clear();
                    self.intervals.clear();
                    self.last = if self.native { Some(now) } else { None };
                    self.cpu_ms = 0.0;
                    self.stalls = 0;
                    self.passes = 0;
                } else if !self.native {
                    self.position += 1;
                }
                if !self.native {
                    for w in &mut self.widgets {
                        w.runtime.cursor = self.position;
                        w.runtime.next_tick = None;
                    }
                }
                self.requested = now;
            }
        }
        assert!(
            self.setup.elapsed() < Duration::from_secs(600),
            "probe timed out"
        );
        if !self.native || self.continuous {
            ui.ctx().request_repaint();
        }
    }
    fn report(&self, ctx: &egui::Context) {
        assert_eq!(
            self.waits.len(),
            self.count,
            "Every frame must be observed; no implicit drops"
        );
        let mut intervals = self.intervals.clone();
        intervals.sort_by(f64::total_cmp);
        let mean = intervals.iter().sum::<f64>() / intervals.len().max(1) as f64;
        let p95 = intervals
            .get(intervals.len() * 95 / 100)
            .copied()
            .unwrap_or(0.0);
        let mode = if !self.native {
            "headless-egui-readiness"
        } else if self.continuous {
            "native-wgpu-continuous"
        } else {
            "native-wgpu-event-driven"
        };
        println!("PROBE {mode} pass={} frames={} instances={} size={:?} total_ms={:.3} mean_interval_ms={mean:.3} p95_ms={p95:.3} wait_sum_ms={:.3} loading_passes={} ui_passes={} ui_cpu_ms={:.3} {}",self.pass+1,self.count,self.widgets.len(),self.size,self.started.unwrap().elapsed().as_secs_f64()*1000.0,self.waits.iter().sum::<f64>(),self.stalls,self.passes,self.cpu_ms,super::probe_stats(ctx));
    }
}
impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.ui(ui);
        if self.done {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
pub(super) fn run() {
    println!("PROBE process_id={}", std::process::id());
    let native = std::env::var_os("TUNNY_ANIMATION_PERF_NATIVE").is_some();
    if native {
        eframe::run_native(
            "Animation workload probe",
            eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default().with_inner_size([1500.0, 900.0]),
                renderer: eframe::Renderer::Wgpu,
                ..Default::default()
            },
            Box::new(|cc| {
                cc.egui_ctx.set_zoom_factor(
                    std::env::var("TUNNY_ANIMATION_SCALE")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(1.0),
                );
                Ok(Box::new(Probe::new(true)))
            }),
        )
        .unwrap();
    } else {
        let ctx = egui::Context::default();
        ctx.set_zoom_factor(
            std::env::var("TUNNY_ANIMATION_SCALE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
        );
        let mut p = Probe::new(false);
        while !p.done {
            let _ = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(2400.0, 1600.0),
                    )),
                    ..Default::default()
                },
                |ui| p.ui(ui),
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        // Near return, distant seek and rapidly obsolete prefetch under production UI.
        for position in [
            p.count.saturating_sub(3),
            0,
            p.count / 2,
            1,
            p.count.saturating_sub(1),
        ] {
            for w in &mut p.widgets {
                w.runtime.cursor = position;
                w.runtime.playing = false;
                w.runtime.next_tick = None;
            }
            let started = Instant::now();
            loop {
                let _ = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(2400.0, 1600.0),
                        )),
                        ..Default::default()
                    },
                    |ui| p.draw(ui),
                );
                if p.widgets.iter().all(|w| {
                    w.runtime
                        .ready
                        .as_ref()
                        .is_some_and(|r| r.index == position)
                }) {
                    break;
                }
                assert!(started.elapsed() < Duration::from_secs(20));
                std::thread::sleep(Duration::from_millis(1));
            }
            println!(
                "PROBE return/seek ms={:.3} {}",
                started.elapsed().as_secs_f64() * 1000.0,
                super::probe_stats(&ctx)
            );
        }
    }
    #[cfg(windows)]
    {
        let command = format!(
            "Get-Process -Id {} | Format-List WorkingSet64,PeakWorkingSet64,PrivateMemorySize64",
            std::process::id()
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &command])
            .output()
            .unwrap();
        println!(
            "PROBE process memory (includes renderer/test harness; not cache budget): {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
