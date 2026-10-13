use super::*;
use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyContext, StudyMeta, StudyView};
use std::collections::HashMap;
use tunny_core::dataframe::{DataFrame, TrialRow};

#[test]
fn duration_normalizes_input_and_persistence_without_fps_conversion() {
    for (input, expected) in [
        (0, 20),
        (19, 20),
        (324, 320),
        (325, 330),
        (333, 330),
        (9999, 10000),
        (u32::MAX, 10000),
    ] {
        assert_eq!(normalize_duration(input), expected);
        let saved = format!("{{\"display_duration_ms\":{input}}}");
        let restored: ArtifactAnimation = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.display_duration_ms, expected);
        let roundtrip: ArtifactAnimation =
            serde_json::from_str(&serde_json::to_string(&restored).unwrap()).unwrap();
        assert_eq!(roundtrip.display_duration_ms, expected);
    }
    assert_eq!(ArtifactAnimation::default().display_duration_ms, 330);
}

#[test]
fn playback_deadline_does_not_add_preparation_or_skip_unready_frames() {
    let ctx = egui::Context::default();
    let mut widget = ArtifactAnimation {
        display_duration_ms: 100,
        ..Default::default()
    };
    widget.runtime.playing = true;
    widget.runtime.sequence = vec![0, 1, 2];
    widget.runtime.ready = Some(ReadyFrame {
        index: 0,
        texture: Arc::new(ctx.load_texture(
            "pacing",
            egui::ColorImage::filled([2, 2], egui::Color32::WHITE),
            egui::TextureOptions::LINEAR,
        )),
        prefix: Arc::new(Prefix {
            number: 0,
            rows: vec![],
            ranks: HashMap::new(),
        }),
    });
    let now = Instant::now();
    widget.advance(now);
    widget.advance(now + Duration::from_millis(100));
    assert_eq!(widget.runtime.cursor, 1);
    assert_eq!(
        widget.runtime.next_tick,
        Some(now + Duration::from_millis(200))
    );
    widget.advance(now + Duration::from_millis(250));
    assert_eq!(widget.runtime.cursor, 1); // Cannot advance over a frame that isn't ready.
    widget.runtime.ready.as_mut().unwrap().index = 1;
    widget.advance(now + Duration::from_millis(251));
    assert_eq!(widget.runtime.cursor, 2); // At most one frame per advancement.
}

#[test]
fn resident_cache_hit_starts_next_prefix_before_sleeping_until_deadline() {
    fn input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            ..Default::default()
        }
    }
    let ctx = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let mut app = AppState::new();
    app.current_study = Some(study(
        &(0..6)
            .map(|i| (i, i, vec![6.0 - i as f64], vec![]))
            .collect::<Vec<_>>(),
        vec![Direction::Minimize],
        false,
    ));
    for i in 0..6 {
        let name = format!("{i}.png");
        image::RgbImage::from_pixel(16, 8, image::Rgb([i as u8, 0, 0]))
            .save(dir.path().join(&name))
            .unwrap();
        app.artifact_map
            .insert(i, vec![entry(dir.path(), &name, &name, "image/png")]);
    }
    let mut widget = ArtifactAnimation {
        looping: false,
        display_duration_ms: 1000,
        skip: 1,
        ..Default::default()
    };
    let started = Instant::now();
    // Setup only: warm every selected image while paused, so no image worker will supply
    // an incidental repaint during the transition being tested.
    loop {
        let _ = ctx.run_ui(input(), |ui| widget.show(ui, &app));
        if widget.runtime.ready.is_some() {
            let data = widget.runtime.data.as_ref().unwrap();
            let k = widget.runtime.key.as_ref().unwrap();
            let shared = cache::shared(&ctx);
            let mut images = shared.lock().unwrap();
            if widget.runtime.sequence.iter().all(|&i| {
                images
                    .get(data, (k.0, k.1, k.2), i, widget.runtime.image_size)
                    .unwrap()
                    .is_some()
            }) {
                break;
            }
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
    let data = widget.runtime.data.clone().unwrap();
    widget.runtime.cache = PrefixCache::default();
    widget
        .runtime
        .cache
        .insert(Arc::new(Prefix::new(&data, data.frames[2].number)));
    assert!(widget.runtime.cache.get(data.frames[4].number).is_none());
    assert!(widget.runtime.loading.is_none());
    let decodes = cache::shared(&ctx).lock().unwrap().stats.decodes;
    let (tx, rx) = mpsc::channel();
    ctx.set_request_repaint_callback(move |info| {
        if info.delay.is_zero() {
            let _ = tx.send(());
        }
    });
    widget.runtime.playing = true;
    let boundary = Instant::now() - Duration::from_millis(1);
    let deadline = boundary + Duration::from_millis(1000);
    widget.runtime.next_tick = Some(boundary);
    let mut output = ctx.run_ui(input(), |ui| widget.show(ui, &app));
    assert_eq!(widget.runtime.sequence, vec![0, 2, 4, 5]);
    assert_eq!(widget.runtime.ready.as_ref().unwrap().index, 2);
    assert_eq!(widget.runtime.next_tick, Some(deadline));
    assert!(
        widget.runtime.loading.is_some(),
        "Cache-hit readiness must start lookahead in the same pass"
    );
    assert_eq!(widget.runtime.loading_index, 4);
    // Follow only egui's immediate passes, worker wakeups, or its actual deadline.
    // No unconditional redraw loop is allowed to hide missing lookahead.
    while widget.runtime.cache.get(data.frames[4].number).is_none() {
        let delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        if delay.is_zero() {
            std::thread::yield_now();
        } else {
            let _ = rx.recv_timeout(delay);
        }
        output = ctx.run_ui(input(), |ui| widget.show(ui, &app));
        assert!(
            Instant::now() < deadline,
            "Next prefix must be prepared during the current dwell"
        );
    }
    std::thread::sleep(
        deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(1),
    );
    let _ = ctx.run_ui(input(), |ui| widget.show(ui, &app));
    assert_eq!(widget.runtime.ready.as_ref().unwrap().index, 4);
    assert_eq!(
        widget.runtime.ready.as_ref().unwrap().prefix.number,
        data.frames[4].number
    );
    assert_eq!(
        widget.runtime.next_tick,
        Some(deadline + Duration::from_millis(1000))
    );
    assert_eq!(cache::shared(&ctx).lock().unwrap().stats.decodes, decodes);
}

#[test]
fn common_sampling_transport_endpoints_remap_and_persistence() {
    let ctx = egui::Context::default();
    let texture = Arc::new(ctx.load_texture(
        "selected-transport",
        egui::ColorImage::filled([2, 2], egui::Color32::WHITE),
        egui::TextureOptions::LINEAR,
    ));
    let mut widget = ArtifactAnimation {
        skip: 1,
        display_duration_ms: 100,
        looping: false,
        ..Default::default()
    };
    widget.sync_sampling(6);
    assert_eq!(widget.runtime.sequence, vec![0, 2, 4, 5]);
    assert_eq!(
        widget.runtime.sequence,
        export::selected_frames(6, widget.skip)
    );
    let now = Instant::now();
    for (position, index) in [0, 2, 4, 5].into_iter().enumerate() {
        assert_eq!(widget.runtime.cursor, index);
        widget.runtime.ready = Some(ReadyFrame {
            index,
            texture: texture.clone(),
            prefix: Arc::new(Prefix {
                number: index as u32,
                rows: vec![],
                ranks: HashMap::new(),
            }),
        });
        widget.runtime.playing = false;
        widget.advance(now + Duration::from_secs(100));
        assert_eq!(widget.runtime.cursor, index); // Paused frames stay selected.
        widget.runtime.playing = true;
        widget.runtime.next_tick = Some(now + Duration::from_millis(position as u64 * 100));
        widget.advance(now + Duration::from_millis(position as u64 * 100));
    }
    assert!(!widget.runtime.playing);
    assert_eq!(widget.runtime.cursor, 5); // Appended endpoint, no duplicate.
    widget.looping = true;
    widget.runtime.playing = true;
    widget.runtime.next_tick = Some(now);
    widget.advance(now);
    assert_eq!(widget.runtime.cursor, 0);
    widget.runtime.seek(1);
    assert_eq!(widget.runtime.cursor, 2); // Next / seek selected ordinal.
    widget.runtime.seek(0);
    assert_eq!(widget.runtime.cursor, 0); // Previous.
    widget.runtime.seek(usize::MAX);
    assert_eq!(widget.runtime.cursor, 5); // Clamp.
    widget.runtime.cursor = 4;
    widget.skip = 2;
    widget.sync_sampling(6);
    assert_eq!(widget.runtime.sequence, vec![0, 3, 5]);
    assert_eq!(widget.runtime.cursor, 3); // Nearest selected predecessor.
    assert!(widget.runtime.ready.is_none());
    let saved = serde_json::to_string(&widget).unwrap();
    let restored: ArtifactAnimation = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored.skip, 2);
    assert!(!saved.contains("export_skip"));
    widget.skip = usize::MAX;
    widget.sync_sampling(6);
    assert_eq!(widget.skip, 5);
    assert_eq!(widget.runtime.sequence, vec![0, 5]);
    widget.sync_sampling(1);
    assert_eq!(widget.skip, 0);
    assert_eq!(widget.runtime.sequence, vec![0]);
}

#[test]
fn common_sampling_changes_retarget_workers_and_preserve_full_observations() {
    let ctx = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let mut app = AppState::new();
    let specs: Vec<_> = (0..8)
        .map(|i| (i, i * 2, vec![8.0 - i as f64], vec![]))
        .collect();
    app.current_study = Some(study(&specs, vec![Direction::Minimize], false));
    for row in [0, 1, 3, 4, 6, 7] {
        let name = format!("{row}.png");
        image::RgbImage::from_pixel(16 + row, 8, image::Rgb([row as u8, 0, 0]))
            .save(dir.path().join(&name))
            .unwrap();
        app.artifact_map
            .insert(row, vec![entry(dir.path(), &name, &name, "image/png")]);
    }
    fn wait(ctx: &egui::Context, w: &mut ArtifactAnimation, app: &AppState) {
        let started = Instant::now();
        loop {
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| w.show(ui, app));
            if w.runtime
                .ready
                .as_ref()
                .is_some_and(|r| r.index == w.runtime.cursor)
            {
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    let mut w = ArtifactAnimation::default();
    wait(&ctx, &mut w, &app);
    let data = w.runtime.data.clone().unwrap();
    let bounds = data.bounds.clone();
    w.skip = 1;
    wait(&ctx, &mut w, &app);
    assert!(Arc::ptr_eq(&data, w.runtime.data.as_ref().unwrap())); // No mapping rebuild.
    assert_eq!(w.runtime.sequence, vec![0, 2, 4, 5]);
    w.runtime.seek(1);
    wait(&ctx, &mut w, &app);
    let ready = w.runtime.ready.as_ref().unwrap();
    assert_eq!(ready.index, 2);
    assert_eq!(ready.prefix.number, 6);
    assert_eq!(ready.prefix.rows, vec![0, 1, 2, 3]); // Skipped image and artifactless row.
    assert_eq!(ready.texture.size(), [19, 8]);
    let (tx, rx) = mpsc::sync_channel(0);
    let obsolete = data.clone();
    w.runtime.loading = Some(Job::spawn(&ctx, move |_| {
        rx.recv().unwrap();
        Ok(Arc::new(Prefix::new(&obsolete, 12)))
    }));
    w.runtime.loading_index = 4;
    w.skip = 2;
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| w.show(ui, &app));
    assert_eq!(w.runtime.sequence, vec![0, 3, 5]);
    assert_eq!(w.runtime.cursor, 0);
    assert!(w.runtime.ready.as_ref().is_none_or(|r| r.index == 0));
    assert!(w
        .runtime
        .loading
        .as_ref()
        .unwrap()
        .cancel
        .load(Ordering::Relaxed));
    tx.send(()).unwrap();
    w.runtime.seek(1);
    wait(&ctx, &mut w, &app);
    let ready = w.runtime.ready.as_ref().unwrap();
    assert_eq!(ready.index, 3);
    assert_eq!(ready.prefix.number, 8);
    assert_eq!(ready.prefix.rows, vec![0, 1, 2, 3, 4]);
    assert_eq!(w.runtime.data.as_ref().unwrap().bounds, bounds);
    let decoder_count = cache::shared(&ctx).lock().unwrap().stats.decodes;
    let cancelled_before = w
        .runtime
        .loading
        .as_ref()
        .map(|j| j.cancel.load(Ordering::Relaxed));
    w.sync_sampling(6);
    assert_eq!(
        cache::shared(&ctx).lock().unwrap().stats.decodes,
        decoder_count
    );
    assert_eq!(
        w.runtime
            .loading
            .as_ref()
            .map(|j| j.cancel.load(Ordering::Relaxed)),
        cancelled_before
    );
    let other = ArtifactAnimation::default();
    assert_eq!(other.skip, 0);
}

#[test]
fn source_resets_keep_obsolete_prefix_work_in_one_slot_until_completion() {
    let ctx = egui::Context::default();
    let (tx, rx) = mpsc::sync_channel(0);
    let mut runtime = Runtime {
        loading: Some(Job::spawn(&ctx, move |_| {
            rx.recv().unwrap();
            Ok(Arc::new(Prefix {
                number: 999,
                rows: vec![],
                ranks: HashMap::new(),
            }))
        })),
        ..Default::default()
    };
    for _ in 0..10 {
        runtime = Runtime {
            loading: runtime.cancelled_prefix(),
            ..Default::default()
        };
        assert!(runtime
            .loading
            .as_ref()
            .unwrap()
            .cancel
            .load(Ordering::Relaxed));
    }
    tx.send(()).unwrap();
    let start = Instant::now();
    while runtime.loading.is_some() {
        runtime.loading = runtime.cancelled_prefix();
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(runtime.prefix.is_none() && runtime.cache.0.is_empty());
}

#[test]
fn repeated_source_changes_queue_only_latest_mapping_and_reject_old_completion() {
    let ctx = egui::Context::default();
    let (tx, rx) = mpsc::sync_channel(0);
    let old = Arc::new(Data::new(
        study(
            &[(999, 99, vec![99.0], vec![])],
            vec![Direction::Minimize],
            false,
        ),
        vec![],
    ));
    let mut widget = ArtifactAnimation::default();
    widget.runtime.mapping = Some(Job::spawn(&ctx, move |_| {
        rx.recv().unwrap();
        Ok(old)
    }));
    let mut app = AppState::new();
    app.current_study = Some(study(
        &[(0, 0, vec![1.0], vec![])],
        vec![Direction::Minimize],
        false,
    ));
    for _ in 0..10 {
        app.source_generation += 1;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| widget.show(ui, &app));
        assert!(widget.runtime.data.is_none() && widget.runtime.mapping_needed);
        assert!(widget
            .runtime
            .mapping
            .as_ref()
            .unwrap()
            .cancel
            .load(Ordering::Relaxed));
    }
    tx.send(()).unwrap();
    let start = Instant::now();
    while widget.runtime.data.is_none() {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| widget.show(ui, &app));
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(widget.runtime.data.as_ref().unwrap().number(0), 0);
    assert!(!widget.runtime.mapping_needed);
}

#[test]
fn rapid_seek_and_reload_discard_obsolete_image_and_prefix_results() {
    let ctx = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let mut app = AppState::new();
    let specs: Vec<_> = (0..24)
        .map(|i| (i, i * 3, vec![i as f64, 24.0 - i as f64], vec![]))
        .collect();
    app.current_study = Some(study(&specs, vec![Direction::Minimize; 2], false));
    for i in 0..24 {
        let path = dir.path().join(format!("{i}.png"));
        image::RgbImage::from_pixel(64 + i, 64, image::Rgb([i as u8, 0, 0]))
            .save(&path)
            .unwrap();
        app.artifact_map.insert(
            i,
            vec![entry(
                dir.path(),
                &format!("{i}.png"),
                &format!("{i}.png"),
                "image/png",
            )],
        );
    }
    let mut widget = ArtifactAnimation::default();
    fn wait(ctx: &egui::Context, widget: &mut ArtifactAnimation, app: &AppState) {
        let start = Instant::now();
        loop {
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| widget.show(ui, app));
            if widget
                .runtime
                .ready
                .as_ref()
                .is_some_and(|r| r.index == widget.runtime.cursor)
            {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    wait(&ctx, &mut widget, &app);
    for position in [23, 1, 22, 2, 21, 3, 20, 4] {
        widget.runtime.cursor = position;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| widget.show(ui, &app));
    }
    wait(&ctx, &mut widget, &app);
    let ready = widget.runtime.ready.as_ref().unwrap();
    assert_eq!(ready.index, 4);
    assert_eq!(ready.prefix.number, 12);
    assert_eq!(ready.texture.size(), [68, 64]);
    std::fs::remove_file(dir.path().join("12.png")).unwrap();
    widget.runtime.cursor = 12;
    wait(&ctx, &mut widget, &app);
    let ready = widget.runtime.ready.as_ref().unwrap();
    assert_eq!(ready.index, 12);
    assert_eq!(ready.prefix.number, 39); // Removed image shifts the index, not the prefix identity.
    assert_eq!(ready.prefix.rows.len(), 14); // The now-artifactless observation is retained.
    assert_eq!(ready.texture.size(), [77, 64]);
    app.source_generation += 1;
    app.artifact_revision += 1;
    wait(&ctx, &mut widget, &app);
    assert_eq!(widget.runtime.cursor, 0);
    assert_eq!(widget.runtime.ready.as_ref().unwrap().prefix.number, 0);
    assert_eq!(
        widget.runtime.ready.as_ref().unwrap().texture.size(),
        [64, 64]
    );
}

#[test]
fn export_sampling_preserves_endpoints_count_duration_and_all_prefix_observations() {
    for count in 0..20 {
        for skip in [0, 1, 2, 19, usize::MAX] {
            assert_eq!(
                export::selected_count(count, skip),
                export::selected_frames(count, skip).len()
            );
        }
    }
    assert!(export::selected_frames(0, usize::MAX).is_empty());
    assert_eq!(export::selected_frames(1, 50), vec![0]);
    assert_eq!(export::selected_frames(6, 0), vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(export::selected_frames(6, 1), vec![0, 2, 4, 5]);
    assert_eq!(export::selected_frames(7, 2), vec![0, 3, 6]);
    assert_eq!(export::selected_frames(6, usize::MAX), vec![0, 5]);
    let s = study(
        &(0..7)
            .map(|i| (i, i * 2, vec![7.0 - i as f64], vec![]))
            .collect::<Vec<_>>(),
        vec![Direction::Minimize],
        false,
    );
    let frames = [0, 2, 4, 6]
        .into_iter()
        .map(|row| data::Frame {
            row,
            number: row as u32 * 2,
            entry: entry(
                std::path::Path::new("."),
                "unused.png",
                "unused.png",
                "image/png",
            ),
        })
        .collect();
    let d = Data::new(s, frames);
    let selected = export::selected_frames(d.frames.len(), 1);
    assert_eq!(selected, vec![0, 2, 3]);
    assert_eq!(selected.len() * 330, 990);
    assert_eq!(
        Prefix::new(&d, d.frames[selected[1]].number).rows,
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(
        Prefix::new(&d, d.frames[*selected.last().unwrap()].number)
            .rows
            .len(),
        7
    );
}

pub(super) fn study(
    specs: &[(u32, u32, Vec<f64>, Vec<f64>)],
    directions: Vec<Direction>,
    constrained: bool,
) -> StudyContext {
    let names: Vec<_> = (0..directions.len()).map(|i| format!("obj{i}")).collect();
    let rows: Vec<_> = specs
        .iter()
        .map(|(id, number, objectives, constraints)| TrialRow {
            trial_id: *id,
            trial_number: *number,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: objectives.clone(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: constraints.clone(),
        })
        .collect();
    let mut df = DataFrame::from_trials(
        &rows,
        &[],
        &names,
        &[],
        &[],
        if constrained { 1 } else { 0 },
    );
    if constrained {
        df.mark_constrained();
    }
    StudyContext {
        meta: StudyMeta {
            study_id: 4,
            name: "animation".into(),
            directions,
            completed_trials: rows.len(),
            param_names: vec![],
            objective_names: names,
            param_bounds: HashMap::new(),
        },
        view: StudyView::new(Arc::new(df), vec![0; rows.len()]),
        pareto_indices: vec![],
    }
}
fn entry(dir: &std::path::Path, disk: &str, filename: &str, mime: &str) -> ArtifactEntry {
    ArtifactEntry {
        path: dir.join(disk),
        filename: filename.into(),
        mimetype: mime.into(),
    }
}

#[test]
fn mapping_uses_study_numbers_and_original_entry_indices_not_journal_ids_or_filtered_indices() {
    let dir = tempfile::tempdir().unwrap();
    for (disk, format) in [
        ("uuid-png", image::ImageFormat::Png),
        ("uuid-jpeg", image::ImageFormat::Jpeg),
    ] {
        image::RgbImage::from_pixel(3, 2, image::Rgb([100, 20, 30]))
            .save_with_format(dir.path().join(disk), format)
            .unwrap();
    }
    std::fs::write(dir.path().join("bad"), b"not an image").unwrap();
    let s = study(
        &[
            (900, 7, vec![1.0], vec![]),
            (12, 0, vec![2.0], vec![]),
            (4, 3, vec![3.0], vec![]),
            (15, 9, vec![4.0], vec![]),
        ],
        vec![Direction::Minimize],
        false,
    );
    let png = entry(dir.path(), "uuid-png", "original.PNG", "image/png");
    let jpeg = entry(dir.path(), "uuid-jpeg", "original.jpg", "image/jpeg");
    assert!(data::read_image(&entry(dir.path(), "uuid-png", "uuid-png", "")).is_ok());
    assert!(data::read_image(&entry(
        dir.path(),
        "uuid-jpeg",
        "uuid-jpeg",
        "image/jpeg; charset=binary"
    ))
    .is_ok());
    let mut artifacts = HashMap::from([
        (900, vec![png.clone(), jpeg.clone()]),
        (12, vec![jpeg.clone()]),
        (
            4,
            vec![
                entry(dir.path(), "uuid-png", "animation.gif", "image/gif"),
                png.clone(),
            ],
        ),
        (
            15,
            vec![entry(dir.path(), "bad", "corrupt.png", "image/png")],
        ),
        (9999, vec![png.clone()]), // Another Study, not a frame.
    ]);
    let cancel = AtomicBool::new(false);
    let frames = data::frame_list(&s, &artifacts, 0, &cancel);
    assert_eq!(
        frames.iter().map(|f| (f.row, f.number)).collect::<Vec<_>>(),
        vec![(1, 0), (0, 7)]
    );
    let frames = data::frame_list(&s, &artifacts, 1, &cancel);
    assert_eq!(
        frames.iter().map(|f| f.number).collect::<Vec<_>>(),
        vec![3, 7]
    );
    assert_eq!(frames[0].entry, png);
    for mime in ["image/webp", "text/csv"] {
        artifacts.insert(12, vec![entry(dir.path(), "uuid-png", "file.csv", mime)]);
        assert_eq!(data::frame_list(&s, &artifacts, 0, &cancel).len(), 1);
    }
    artifacts.insert(
        900,
        vec![entry(dir.path(), "missing", "gone.jpeg", "image/jpeg")],
    );
    assert!(data::frame_list(&s, &artifacts, 0, &cancel).is_empty());
}

#[test]
fn prefix_includes_artifactless_trials_excludes_future_and_restores_front_on_backseek() {
    let s = study(
        &[
            (80, 0, vec![4.0, 2.0, 1.0], vec![]),
            (100, 3, vec![3.0, 3.0, 0.0], vec![]), // Better X/Y but worse maximize Z: both remain on front.
            (150, 8, vec![2.0, 1.0, 4.0], vec![]), // Dominates both, no artifact needed.
            (170, 10, vec![f64::INFINITY, 0.0, 5.0], vec![]),
            (180, 12, vec![1.0, f64::NAN, 8.0], vec![]),
        ],
        vec![
            Direction::Minimize,
            Direction::Minimize,
            Direction::Maximize,
        ],
        false,
    );
    let d = Data::new(s, vec![]);
    let mut cache = PrefixCache::default();
    let early = Arc::new(Prefix::new(&d, 3));
    assert_eq!(early.rows, vec![0, 1]);
    assert_eq!(early.ranks.get(&0), Some(&0));
    assert_eq!(early.ranks.get(&1), Some(&0));
    cache.insert(early.clone());
    let late = Arc::new(Prefix::new(&d, 12));
    assert_eq!(late.ranks.get(&2), Some(&0));
    assert_eq!(late.ranks.get(&0), Some(&1));
    assert!(!late.ranks.contains_key(&3));
    assert!(!late.ranks.contains_key(&4));
    cache.insert(late);
    assert!(Arc::ptr_eq(&cache.get(3).unwrap(), &early));
    for number in 20..40 {
        cache.insert(Arc::new(Prefix::new(&d, number)));
    }
    assert_eq!(cache.0.len(), 8);
    assert_eq!(Prefix::new(&d, 3).ranks, early.ranks);
    assert!(d
        .bounds
        .iter()
        .all(|(lo, hi)| lo.is_finite() && hi.is_finite()));
}

#[test]
fn constrained_prefix_ranks_feasible_then_violation_then_unverified_and_invalid_never_on_front() {
    let s = study(
        &[
            (1, 0, vec![1.0, 1.0], vec![1.0]),
            (2, 2, vec![0.0, 0.0], vec![]),
            (3, 5, vec![3.0, 3.0], vec![-1.0]),
            (4, 7, vec![2.0, 2.0], vec![0.0]),
            (5, 9, vec![f64::NAN, 0.0], vec![0.0]),
        ],
        vec![Direction::Minimize, Direction::Minimize],
        true,
    );
    let d = Data::new(s, vec![]);
    let before = Prefix::new(&d, 2);
    assert!(before.ranks.values().all(|&r| r > 0));
    let after = Prefix::new(&d, 9);
    assert_eq!(after.ranks[&3], 0);
    assert_eq!(after.ranks[&2], 1);
    assert!(after.ranks[&0] > after.ranks[&2]);
    assert!(after.ranks[&1] > after.ranks[&0]);
    assert!(!after.ranks.contains_key(&4));
    let tied = Data::new(
        study(
            &[
                (90, 9, vec![1.0, 1.0], vec![1.0]),
                (10, 0, vec![1.0, 1.0], vec![1.0]),
                (40, 5, vec![1.0, 1.0], vec![]),
                (30, 3, vec![1.0, 1.0], vec![]),
            ],
            vec![Direction::Minimize; 2],
            true,
        ),
        vec![],
    );
    let ranks = Prefix::new(&tied, 9).ranks;
    assert!(ranks[&0] < ranks[&1]); // Constraint-violation ties retain DataFrame input order.
    assert!(ranks[&2] < ranks[&3]); // Unverified ranks also retain input order, not display order.
}

#[test]
fn history_uses_local_numbers_finite_selected_values_and_all_three_feasibility_states() {
    let context = egui::Context::default();
    let texture = context.load_texture(
        "fixture",
        egui::ColorImage::filled([2, 2], egui::Color32::WHITE),
        egui::TextureOptions::LINEAR,
    );
    let s = study(
        &[
            (800, 8, vec![4.0, 5.0], vec![0.0]),
            (300, 0, vec![3.0, 1.0], vec![0.0]),
            (400, 3, vec![f64::NAN, 7.0], vec![1.0]), // Valid for selected history objective only.
            (600, 5, vec![2.0, 2.0], vec![]),
            (900, 12, vec![1.0, 99.0], vec![0.0]), // Future result is never plotted.
        ],
        vec![Direction::Minimize, Direction::Maximize],
        true,
    );
    let frames = vec![data::Frame {
        row: 0,
        number: 8,
        entry: entry(
            std::path::Path::new("."),
            "unused",
            "unused.png",
            "image/png",
        ),
    }];
    let d = Data::new(s, frames);
    let ready = ReadyFrame {
        index: 0,
        texture: Arc::new(texture),
        prefix: Arc::new(Prefix::new(&d, 8)),
    };
    let p = Presentation {
        history_objective: 1,
        ..Default::default()
    };
    let series = presentation::series(&d, &ready, &p, true);
    assert_eq!(series.groups[1], vec![[0.0, 1.0], [8.0, 5.0]]);
    assert_eq!(series.groups[2], vec![[3.0, 7.0]]);
    assert_eq!(series.groups[3], vec![[5.0, 2.0]]);
    assert_eq!(
        series.best_points,
        vec![[0.0, 1.0], [3.0, 7.0], [5.0, 7.0], [8.0, 7.0]]
    );
    assert_eq!(series.highlighted, vec![[8.0, 5.0]]);
    let pareto = presentation::series(&d, &ready, &p, false);
    assert!(!pareto
        .groups
        .iter()
        .flatten()
        .any(|v| !v[0].is_finite() || !v[1].is_finite()));
}

#[test]
fn objective_identity_is_not_shadowed_by_a_same_named_parameter() {
    let mut s = study(
        &[(70, 0, vec![1.0], vec![])],
        vec![Direction::Minimize],
        false,
    );
    let row = TrialRow {
        trial_id: 70,
        trial_number: 0,
        param_display: HashMap::from([("obj0".into(), 999.0)]),
        distribution_metadata: Default::default(),
        param_category_label: HashMap::new(),
        objective_values: vec![1.0],
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        user_attrs_json: HashMap::new(),
        constraint_values: vec![],
    };
    s.meta.param_names = vec!["obj0".into()];
    s.view = StudyView::new(
        Arc::new(DataFrame::from_trials(
            &[row],
            &["obj0".into()],
            &["obj0".into()],
            &[],
            &[],
            0,
        )),
        vec![0],
    );
    assert_eq!(s.view.numeric_column("obj0").unwrap(), &[999.0]); // Existing generic lookup is ambiguous.
    let data = Data::new(s, vec![]);
    assert_eq!(data.objectives, vec![vec![1.0]]);
    assert!(data.bounds[0].1 < 2.0);
    assert_eq!(
        data.study.view.df.numeric_parameter_column("obj0").unwrap(),
        &[999.0]
    );
}

#[test]
fn playback_pause_step_loop_and_persistent_instances_are_independent() {
    let context = egui::Context::default();
    let texture = context.load_texture(
        "fixture",
        egui::ColorImage::filled([2, 2], egui::Color32::WHITE),
        egui::TextureOptions::LINEAR,
    );
    let mut a = ArtifactAnimation {
        display_duration_ms: 100,
        looping: false,
        ..Default::default()
    };
    let b = ArtifactAnimation::default();
    a.presentation.history = true;
    a.presentation.objective_names = vec!["obj0".into()];
    let snapshot = a.presentation.snapshot();
    a.presentation.objective_names.push("obj1".into());
    assert_eq!(snapshot.objective_names, vec!["obj0"]);
    assert!(snapshot.palette.is_some());
    a.runtime.ready = Some(ReadyFrame {
        index: 0,
        texture: Arc::new(texture),
        prefix: Arc::new(Prefix {
            number: 0,
            rows: vec![],
            ranks: HashMap::new(),
        }),
    });
    let now = Instant::now();
    a.runtime.sequence = vec![0, 1];
    a.advance(now);
    assert_eq!(a.runtime.cursor, 0);
    a.runtime.playing = true;
    a.advance(now);
    a.advance(now + Duration::from_millis(101));
    assert_eq!(a.runtime.cursor, 1);
    a.runtime.ready.as_mut().unwrap().index = 1;
    a.advance(now + Duration::from_millis(200));
    a.advance(now + Duration::from_millis(301));
    assert!(!a.runtime.playing);
    a.looping = true;
    a.runtime.playing = true;
    a.advance(now + Duration::from_millis(400));
    a.advance(now + Duration::from_millis(501));
    assert_eq!(a.runtime.cursor, 0);
    a.runtime.exporting = true;
    a.finish_export(Err("write error".into()));
    assert!(a.runtime.playing); // Original playback state restored, not reset.
    assert!(!a.runtime.exporting);
    for result in [Ok(true), Ok(false), Err("capture failed".into())] {
        let cursor = a.runtime.cursor;
        a.runtime.exporting = true;
        a.finish_export(result);
        assert!(!a.runtime.exporting);
        assert_eq!(a.runtime.cursor, cursor);
        assert!(a.runtime.playing);
    }
    assert!(!b.presentation.history);
    let saved = serde_json::to_string(&a).unwrap();
    let restored: ArtifactAnimation = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored.display_duration_ms, 100);
    assert!(restored.presentation.history);
    assert!(!restored.runtime.playing);
}

#[test]
fn shared_composition_draws_only_prefix_and_omits_disabled_values_and_transport() {
    let context = egui::Context::default();
    let texture = context.load_texture(
        "fixture",
        egui::ColorImage::filled([3, 2], egui::Color32::RED),
        egui::TextureOptions::LINEAR,
    );
    let s = study(
        &[
            (1, 0, vec![4.0, 3.0], vec![]),
            (2, 8, vec![1.0, 2.0], vec![]),
        ],
        vec![Direction::Minimize; 2],
        false,
    );
    let frame = data::Frame {
        row: 0,
        number: 0,
        entry: entry(
            std::path::Path::new("."),
            "unused",
            "unused.png",
            "image/png",
        ),
    };
    let d = Data::new(s, vec![frame]);
    let ready = ReadyFrame {
        index: 0,
        texture: Arc::new(texture),
        prefix: Arc::new(Prefix::new(&d, 0)),
    };
    let p = Presentation {
        history: true,
        pareto: true,
        ..Default::default()
    };
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        presentation::content(
            ui,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)),
            &d,
            &ready,
            &p,
        );
    });
    assert!(!output.shapes.is_empty());
    assert_eq!(ready.prefix.rows, vec![0]);
    fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
        match shape {
            egui::epaint::Shape::Text(t) => out.push(t.galley.text().into()),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    texts(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut labels = Vec::new();
    for shape in output.shapes {
        texts(&shape.shape, &mut labels);
    }
    assert!(labels.iter().any(|t| t == "Trial 0"));
    assert!(labels.iter().any(|t| t == "Optimization History"));
    assert!(!labels.iter().any(|t| [
        "Play",
        "Export GIF",
        "Presentation",
        "Objectives",
        "Parameters"
    ]
    .contains(&t.as_str())));
    let p = Presentation {
        trial_number: false,
        objectives: true,
        objective_names: vec!["obj1".into()],
        parameters: true,
        parameter_names: vec!["missing".into()],
        ..Default::default()
    };
    let output = context.run_ui(egui::RawInput::default(), |ui| {
        presentation::content(
            ui,
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)),
            &d,
            &ready,
            &p,
        );
    });
    let mut labels = Vec::new();
    for shape in output.shapes {
        texts(&shape.shape, &mut labels);
    }
    assert!(labels.iter().any(|t| t == "obj1: 3.0000"));
    assert!(labels.iter().any(|t| t == "missing: —"));
    assert!(!labels.iter().any(|t| t == "Trial 0"
        || t == "Optimization History"
        || t == "Pareto 2D (all objectives)"));
}

#[test]
fn source_snapshot_and_same_path_rescan_reset_cursor_cache_and_owned_texture() {
    let context = egui::Context::default();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("image.png");
    image::RgbImage::from_pixel(8, 4, image::Rgb([255, 0, 0]))
        .save(&path)
        .unwrap();
    let e = entry(dir.path(), "image.png", "image.png", "image/png");
    let mut app = AppState::new();
    app.current_study = Some(study(
        &[(1000, 4, vec![3.0], vec![])],
        vec![Direction::Minimize],
        false,
    ));
    app.artifact_map.insert(1000, vec![e.clone()]);
    let mut widget = ArtifactAnimation::default();
    fn wait(context: &egui::Context, widget: &mut ArtifactAnimation, app: &AppState) {
        let start = Instant::now();
        loop {
            let _ = context.run_ui(egui::RawInput::default(), |ui| widget.show(ui, app));
            if widget.runtime.ready.is_some() {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "preparation timed out"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    wait(&context, &mut widget, &app);
    let old_texture = widget.runtime.ready.as_ref().unwrap().texture.id();
    assert_eq!(widget.runtime.data.as_ref().unwrap().frames[0].number, 4);
    widget.runtime.playing = true;
    image::RgbImage::from_pixel(4, 8, image::Rgb([0, 255, 0]))
        .save(&path)
        .unwrap();
    app.artifact_revision += 1;
    wait(&context, &mut widget, &app);
    assert!(!widget.runtime.playing);
    assert_ne!(
        widget.runtime.ready.as_ref().unwrap().texture.id(),
        old_texture
    );
    assert_eq!(
        widget.runtime.ready.as_ref().unwrap().texture.size(),
        [4, 8]
    );
    app.current_study = Some(study(
        &[(22, 9, vec![1.0], vec![])],
        vec![Direction::Maximize],
        false,
    ));
    app.artifact_map.insert(22, vec![e]);
    app.source_generation += 1;
    widget.artifact_index = 7; // New Study has only entry zero.
    wait(&context, &mut widget, &app);
    assert_eq!(widget.artifact_index, 0);
    assert_eq!(widget.runtime.ready.as_ref().unwrap().prefix.number, 9);
    assert_eq!(widget.runtime.data.as_ref().unwrap().frames.len(), 1);
    app.study_streaming = true;
    let _ = context.run_ui(egui::RawInput::default(), |ui| widget.show(ui, &app));
    assert!(widget.runtime.data.is_none() && widget.runtime.ready.is_none());
    app.study_streaming = false;
    wait(&context, &mut widget, &app);
    widget.display_duration_ms = 10000;
    widget.looping = false;
    widget.runtime.playing = true;
    let mut delay = Duration::ZERO;
    for _ in 0..6 {
        let output = context.run_ui(egui::RawInput::default(), |ui| widget.show(ui, &app));
        delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(
        delay > Duration::from_secs(9),
        "Ready slow playback must not poll at 50 Hz: {delay:?}"
    );
    widget.runtime.next_tick = Some(Instant::now() - Duration::from_millis(1));
    let stopped = context.run_ui(egui::RawInput::default(), |ui| widget.show(ui, &app));
    assert!(!widget.runtime.playing);
    assert_eq!(
        stopped.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        Duration::ZERO
    );
    app.current_study = None;
    let old_data = widget.runtime.data.clone().unwrap();
    let old_key = widget.runtime.key.as_ref().unwrap();
    let old_scope = (old_key.0, old_key.1, old_key.2);
    let old_size = widget.runtime.image_size;
    let _ = context.run_ui(egui::RawInput::default(), |ui| widget.show(ui, &app));
    assert!(widget.runtime.ready.is_none() && widget.runtime.cache.0.is_empty());
    assert!(cache::shared(&context)
        .lock()
        .unwrap()
        .get(&old_data, old_scope, 0, old_size)
        .unwrap()
        .is_none());
}

/// Real eframe/wgpu screenshot -> crop -> background GIF -> decode, not a synthetic
/// screenshot event. Opt-in because CI/headless machines need not have a desktop.
#[allow(
    dead_code,
    reason = "Called only by the opt-in main-thread native capture harness"
)]
pub(super) fn native_capture_check() {
    use image::AnimationDecoder;
    struct Probe {
        app: crate::app::TunnyApp,
        path: std::path::PathBuf,
        exporting: bool,
        started: Instant,
        outcome: Arc<std::sync::Mutex<Option<Result<bool, String>>>>,
        ticks: Arc<std::sync::atomic::AtomicUsize>,
        capture_size: Arc<std::sync::Mutex<Option<egui::Vec2>>>,
        preview: Arc<std::sync::Mutex<Vec<usize>>>,
        preview_started: bool,
        preview_done: bool,
    }
    impl eframe::App for Probe {
        fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
            self.app.logic(ctx, frame);
            let widget = &self.app.canvas_widgets[&0].artifact_animation;
            if self.exporting && !widget.runtime.exporting {
                let result = widget.runtime.error.clone().map_or(Ok(true), Err);
                assert_eq!(
                    widget.runtime.cursor,
                    widget.runtime.sequence[1.min(widget.runtime.sequence.len() - 1)]
                );
                assert!(!widget.runtime.playing);
                assert_eq!(widget.display_duration_ms, 330);
                assert!(widget.presentation.history && widget.presentation.pareto);
                let other = &self.app.canvas_widgets[&1].artifact_animation;
                assert_eq!(other.display_duration_ms, 1000);
                assert!(!other.presentation.history && !other.looping);
                *self.outcome.lock().unwrap() = Some(result);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if self.started.elapsed() > Duration::from_secs(180) {
                *self.outcome.lock().unwrap() = Some(Err("Native capture check timed out".into()));
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
            self.ticks.fetch_add(1, Ordering::Relaxed);
            self.app.ui(ui, frame);
            let widget = &mut self
                .app
                .canvas_widgets
                .get_mut(&0)
                .unwrap()
                .artifact_animation;
            if !self.exporting {
                let Some(index) = widget.runtime.ready.as_ref().map(|ready| ready.index) else {
                    return;
                };
                if !self.preview_done {
                    let mut preview = self.preview.lock().unwrap();
                    if preview.last() != Some(&index) {
                        preview.push(index);
                        assert_eq!(*preview, widget.runtime.sequence[..preview.len()]);
                        assert_eq!(
                            widget.runtime.ready.as_ref().unwrap().prefix.rows.len(),
                            index + 1
                        );
                    }
                    if !self.preview_started {
                        self.preview_started = true;
                        widget.runtime.playing = true;
                        widget.looping = false;
                        ui.ctx().request_repaint();
                        return;
                    }
                    if index != *widget.runtime.sequence.last().unwrap() || widget.runtime.playing {
                        return;
                    }
                    assert_eq!(*preview, widget.runtime.sequence);
                    self.preview_done = true;
                }
                // Seek away from the beginning before exporting through the real
                // per-instance request handoff. Only the OS file picker is bypassed.
                let target = widget.runtime.sequence[1.min(widget.runtime.sequence.len() - 1)];
                if widget.runtime.cursor != target {
                    widget.runtime.cursor = target;
                    ui.ctx().request_repaint();
                    return;
                }
                if index != target {
                    return;
                }
                let size = widget.runtime.content_size.unwrap();
                *self.capture_size.lock().unwrap() =
                    Some(size * self.app.layout.canvas.zoom * ui.ctx().pixels_per_point());
                widget.runtime.export_request = Some(export::Request {
                    data: widget.runtime.data.clone().unwrap(),
                    presentation: widget.presentation.snapshot(),
                    size,
                    display_duration_ms: widget.display_duration_ms,
                    selected: widget.runtime.sequence.clone(),
                    scope: {
                        let k = widget.runtime.key.as_ref().unwrap();
                        (k.0, k.1, k.2)
                    },
                    looping: widget.looping,
                    path: self.path.clone(),
                    visuals: ui.visuals().clone(),
                });
                widget.runtime.exporting = true;
                self.exporting = true;
                ui.ctx().request_repaint();
            }
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let count: usize = std::env::var("TUNNY_NATIVE_CAPTURE_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);
    let scale: f32 = std::env::var("TUNNY_NATIVE_CAPTURE_SCALE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0);
    assert!(count > 0 && count <= 1000);
    let specs: Vec<_> = (0..count)
        .map(|i| {
            (
                77 + i as u32 * 12,
                if i == 1 { 3 } else { i as u32 * 4 },
                vec![(count - i) as f64, (i % 4 + 1) as f64],
                vec![],
            )
        })
        .collect();
    let s = study(&specs, vec![Direction::Minimize; 2], false);
    let mut frames = Vec::new();
    for (row, spec) in specs.iter().enumerate() {
        let (dimensions, color) = [
            ((160, 80), [255, 0, 0]),
            ((80, 160), [0, 255, 0]),
            ((120, 120), [0, 0, 255]),
        ][row % 3];
        let filename = format!("{row}.{}", if row % 2 == 0 { "png" } else { "jpg" });
        image::RgbImage::from_pixel(dimensions.0, dimensions.1, image::Rgb(color))
            .save(dir.path().join(&filename))
            .unwrap();
        frames.push(data::Frame {
            row,
            number: spec.1,
            entry: entry(
                dir.path(),
                &filename,
                &filename,
                if row % 2 == 0 {
                    "image/png"
                } else {
                    "image/jpeg"
                },
            ),
        });
    }
    let artifacts: HashMap<_, _> = frames
        .into_iter()
        .map(|f| (s.view.trial_ids[f.row], vec![f.entry]))
        .collect();
    let path = std::env::var_os("TUNNY_NATIVE_CAPTURE_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| dir.path().join("native.gif"));
    let output = path.clone();
    let outcome = Arc::new(std::sync::Mutex::new(None));
    let result = outcome.clone();
    let ticks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let ui_ticks = ticks.clone();
    let capture_size = Arc::new(std::sync::Mutex::new(None));
    let expected_size = capture_size.clone();
    let preview = Arc::new(std::sync::Mutex::new(Vec::new()));
    let preview_order = preview.clone();
    eframe::run_native(
        "Artifact Animation capture check",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 800.0]),
            renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = crate::app::TunnyApp::new(cc, None, None, false);
            app.app_state.all_studies = vec![s.meta.clone()];
            app.app_state.current_study = Some(s);
            app.app_state.artifact_map = artifacts;
            app.layout.canvas.zoom = scale;
            for x in [0.0, 720.0] {
                app.layout.canvas.add(
                    crate::state::layout_state::PanelItem::Chart(
                        crate::state::layout_state::ChartId::ArtifactAnimation,
                    ),
                    x,
                    0.0,
                    700.0,
                    700.0,
                );
            }
            let first = app.canvas_widgets.entry(0).or_default();
            first.artifact_animation.display_duration_ms = 330;
            first.artifact_animation.skip = std::env::var("TUNNY_NATIVE_CAPTURE_SKIP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            first.artifact_animation.presentation = Presentation {
                objectives: true,
                objective_names: vec!["obj0".into()],
                history: true,
                pareto: true,
                ..Default::default()
            };
            let second = app.canvas_widgets.entry(1).or_default();
            second.artifact_animation.display_duration_ms = 1000;
            second.artifact_animation.looping = false;
            Ok(Box::new(Probe {
                app,
                path,
                exporting: false,
                started: Instant::now(),
                outcome: result,
                ticks: ui_ticks,
                capture_size,
                preview,
                preview_started: false,
                preview_done: false,
            }))
        }),
    )
    .unwrap();
    assert!(outcome
        .lock()
        .unwrap()
        .take()
        .expect("capture completed")
        .unwrap());
    let decoder = image::codecs::gif::GifDecoder::new(std::io::BufReader::new(
        std::fs::File::open(&output).unwrap(),
    ))
    .unwrap();
    let frames = decoder.into_frames().collect_frames().unwrap();
    let skip = std::env::var("TUNNY_NATIVE_CAPTURE_SKIP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let selected = export::selected_frames(count, skip);
    assert_eq!(*preview_order.lock().unwrap(), selected);
    assert_eq!(frames.len(), selected.len());
    let dimensions = frames[0].buffer().dimensions();
    let expected = expected_size.lock().unwrap().unwrap();
    assert!(
        (dimensions.0 as f32 - expected.x).abs() <= 1.0
            && (dimensions.1 as f32 - expected.y).abs() <= 1.0
    );
    for (i, frame) in frames.iter().enumerate() {
        let channel = selected[i] % 3;
        assert_eq!(frame.buffer().dimensions(), dimensions);
        assert_eq!(frame.delay().numer_denom_ms(), (330, 1));
        // Image center, safely away from annotations/text/plots, checks no one-frame lag.
        let pixel = frame
            .buffer()
            .get_pixel(dimensions.0 * 18 / 100, dimensions.1 / 2)
            .0;
        assert!(
            pixel[channel] > 240 && pixel[(channel + 1) % 3] < 20 && pixel[(channel + 2) % 3] < 20,
            "wrong native frame color: {pixel:?}"
        );
        let corner = frame.buffer().get_pixel(0, 0).0;
        assert_ne!(
            corner,
            [255, 0, 255, 255],
            "surrounding viewport was captured"
        );
    }
    assert!(ticks.load(Ordering::Relaxed) >= selected.len() * 2);
    println!("Native full-app wgpu preview/GIF passed: {} selected of {count} frames, skip {skip}, identical live/GIF order, {dimensions:?}, scale {scale}, {} responsive UI passes, full prefix observations, two independent instances, original seek/pause restoration, PNG/JPEG aspect ratios 2:1 / 1:2 / 1:1, 330 ms, frame-order image colors and content-only crop. Output: {}", selected.len(), ticks.load(Ordering::Relaxed), output.display());
}
