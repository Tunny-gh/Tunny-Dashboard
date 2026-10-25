use super::*;
use crate::state::types::StudyView;
use crate::ui::widgets::timeline::build_timeline_bars;
use crate::ui::widgets::timeline::{select_time_unit, TimelineChart};
use std::sync::Arc;
use tdashboard_analysis::projection::project;
use tdashboard_io::journal::parser::study;
use tdashboard_study::{IdentityAllocator, LoadKind, StudyKey, StudyManager};

const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/timeline-intervals.jsonl");

fn load(bytes: &[u8], batch_size: Option<usize>) -> AppState {
    let key = StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id: 0,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut apply = |batch| {
        header = manager.apply(header, batch).unwrap().next_header(ticket);
        Ok(())
    };
    if let Some(size) = batch_size {
        study::read_batches(bytes, key, size, &mut apply).unwrap();
    } else {
        apply(study::read_full(bytes, key).unwrap()).unwrap();
    }
    manager.finish(ticket, header.sequence).unwrap();
    let input = manager.acquire().unwrap();
    let projection = Arc::new(project(&input));
    let meta = &projection.meta;
    let study = StudyContext {
        meta: crate::state::types::StudyMeta {
            study_id: 0,
            name: meta.name.clone(),
            directions: vec![crate::state::types::Direction::Minimize],
            completed_trials: meta.completed_trials as usize,
            param_names: meta.param_names.clone(),
            objective_names: meta.objective_names.clone(),
            param_bounds: meta.param_bounds.clone(),
        },
        view: StudyView::from_projection(projection, input, None),
        pareto_indices: vec![],
    };
    AppState {
        current_study: Some(study),
        ..Default::default()
    }
}

#[test]
fn timeline_journal_projection_and_csv_preserve_intervals_and_missingness() {
    for batch_size in [None, Some(1), Some(100)] {
        let state = load(FIXTURE, batch_size);
        let extras = state.current_study.as_ref().unwrap().view.extras().unwrap();
        let epoch = 1_767_225_600.0;
        assert_eq!(extras.trials[0].datetime_complete, Some(epoch + 2.0));
        assert_eq!(extras.trials[3].datetime_complete, Some(epoch + 5.0));
        for i in [6, 7, 8, 9, 10] {
            assert_eq!(extras.trials[i].datetime_complete, None);
        }
        let bars = build_timeline_bars(&extras.trials);
        assert_eq!(
            bars.iter()
                .map(|b| (b.trial_number, b.start, b.end))
                .collect::<Vec<_>>(),
            vec![
                (0, 0.0, 2.0),
                (1, 3.0, 4.0),
                (2, 1.0, 3.0),
                (3, 4.0, 5.0),
                (4, 5.0, 6.0),
                (5, 6.0, 7.0),
                (9, 7.0, 8.0)
            ]
        );
        let csv = build_chart_csv(&ChartId::Timeline, &state, &WidgetStates::default()).unwrap();
        let lines: Vec<_> = csv.lines().collect();
        assert_eq!(
            lines[0],
            "trial_number,state,start_elapsed_s,end_elapsed_s,duration_s"
        );
        assert_eq!(lines.len(), bars.len() + 1);
        for (line, bar) in lines[1..].iter().zip(&bars) {
            assert_eq!(
                *line,
                format!(
                    "{},{},{},{},{}",
                    bar.trial_number,
                    bar.state.label(),
                    bar.start,
                    bar.end,
                    bar.end - bar.start
                )
            );
        }
    }
}

#[test]
fn timeline_csv_is_unavailable_when_only_terminal_timestamps_are_missing() {
    let bytes = concat!(
        "{\"op_code\":0,\"study_name\":\"missing\",\"directions\":[1]}\n",
        "{\"op_code\":4,\"study_id\":0,\"state\":1,\"distributions\":{},\"params\":{},\"values\":[1],\"datetime_start\":\"2026-01-01T00:00:00\"}\n"
    );
    let state = load(bytes.as_bytes(), None);
    assert!(!has_csv_data(
        &ChartId::Timeline,
        &state,
        &WidgetStates::default()
    ));
    assert!(build_chart_csv(&ChartId::Timeline, &state, &WidgetStates::default()).is_none());
}

fn rendered_rects(
    extras: &tdashboard_analysis::extras::StudyExtras,
    zoom: Option<egui_plot::PlotBounds>,
) -> Vec<egui::Rect> {
    let ctx = egui::Context::default();
    let mut chart = TimelineChart::default();
    let mut plot_id = egui::Id::NULL;
    let mut output = None;
    for frame in 0..4 {
        output = Some(ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100.0, 650.0),
                )),
                ..Default::default()
            },
            |ui| {
                plot_id = ui.make_persistent_id(egui::Id::new("timeline_plot"));
                chart.show(ui, Some(extras));
            },
        ));
        if frame == 1 {
            if let Some(bounds) = zoom {
                let mut memory = egui_plot::PlotMemory::load(&ctx, plot_id).unwrap();
                memory.auto_bounds = false.into();
                memory.set_bounds(bounds);
                memory.store(&ctx, plot_id);
            }
        }
    }
    let memory = egui_plot::PlotMemory::load(&ctx, plot_id).unwrap();
    let transform = memory.transform();
    let colors = [
        crate::theme::chart_colors::COLOR_STATE_COMPLETE(),
        crate::theme::chart_colors::COLOR_STATE_PRUNED(),
        crate::theme::chart_colors::COLOR_STATE_FAIL(),
        crate::theme::chart_colors::COLOR_STATE_RUNNING(),
    ];
    fn collect(shape: &egui::Shape, colors: &[egui::Color32], rects: &mut Vec<egui::Rect>) {
        match shape {
            egui::Shape::Rect(r)
                if colors.contains(&r.fill) && r.corner_radius == egui::CornerRadius::ZERO =>
            {
                rects.push(r.rect)
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, colors, rects);
                }
            }
            _ => {}
        }
    }
    let mut rects = Vec::new();
    for shape in output.unwrap().shapes {
        collect(&shape.shape, &colors, &mut rects);
    }
    let bars = build_timeline_bars(&extras.trials);
    assert_eq!(rects.len(), bars.len());
    let divisor = select_time_unit(bars.iter().map(|b| b.end).fold(0.0, f64::max)).divisor();
    for (rect, bar) in rects.iter().zip(&bars) {
        let left = transform.value_from_position(rect.left_center());
        let right = transform.value_from_position(rect.right_center());
        assert!(
            (left.x * divisor - bar.start).abs() < 0.001,
            "left endpoint: {bar:?}"
        );
        assert!(
            (right.x * divisor - bar.end).abs() < 0.001,
            "right endpoint: {bar:?}"
        );
        assert!((left.y - bar.trial_number as f64).abs() < 0.01);
    }
    rects
        .into_iter()
        .filter(|r| r.intersects(*transform.frame()))
        .collect()
}

#[test]
fn timeline_rendered_rectangles_agree_with_sequential_and_parallel_csv_intervals() {
    let state = load(FIXTURE, None);
    let extras = state.current_study.as_ref().unwrap().view.extras().unwrap();
    assert_eq!(rendered_rects(extras, None).len(), 7);
}

fn check_dense_render(state: &AppState) {
    let extras = state.current_study.as_ref().unwrap().view.extras().unwrap();
    let bars = build_timeline_bars(&extras.trials);
    assert_eq!(bars.len(), 2500);
    assert!(bars
        .iter()
        .all(|b| b.end > b.start && b.end - b.start < 0.1));
    let overview = rendered_rects(extras, None);
    assert_eq!(overview.len(), 2500);
    let target = bars[1250];
    let zoom = rendered_rects(
        extras,
        Some(egui_plot::PlotBounds::from_min_max(
            [target.start - 0.05, 1248.0],
            [target.end + 0.05, 1252.0],
        )),
    );
    assert!(!zoom.is_empty());
    assert!(zoom.iter().any(|r| r.width() > 1.0 && r.height() > 1.0));
    let csv = build_chart_csv(&ChartId::Timeline, state, &WidgetStates::default()).unwrap();
    assert_eq!(csv.lines().count(), 2501);
    for (line, bar) in csv.lines().skip(1).zip(&bars) {
        let fields: Vec<_> = line.split(',').collect();
        for (field, value) in fields[2..]
            .iter()
            .zip([bar.start, bar.end, bar.end - bar.start])
        {
            assert!((field.parse::<f64>().unwrap() - value).abs() < 0.000001);
        }
    }
}

#[test]
fn timeline_2500_trial_overview_and_zoom_preserve_short_intervals() {
    let mut bytes =
        b"{\"op_code\":0,\"study_name\":\"dense-synthetic\",\"directions\":[1]}\n".to_vec();
    for i in 0..2500 {
        let start_us = i * 14000;
        let end_us = start_us + 10000;
        let timestamp = |us: u32| {
            format!(
                "2026-01-01T00:00:{:02}.{:06}",
                us / 1_000_000,
                us % 1_000_000
            )
        };
        let record = serde_json::json!({"op_code":4,"study_id":0,"state":1,
            "datetime_start":timestamp(start_us),"datetime_complete":timestamp(end_us),
            "distributions":{},"params":{},"values":[1]});
        bytes.extend_from_slice(format!("{record}\n").as_bytes());
    }
    let state = load(&bytes, Some(100));
    check_dense_render(&state);
}

#[test]
#[ignore = "requires TUNNY_TIMELINE_JOURNAL pointing to a local 2500-trial reference Journal"]
fn timeline_optional_reference_journal_overview_and_zoom() {
    let path = std::env::var_os("TUNNY_TIMELINE_JOURNAL")
        .expect("set TUNNY_TIMELINE_JOURNAL to the local reference Journal");
    let bytes = std::fs::read(path).unwrap();
    let state = load(&bytes, Some(100));
    let extras = state.current_study.as_ref().unwrap().view.extras().unwrap();
    assert!(extras
        .trials
        .iter()
        .all(|t| t.datetime_start.is_some() && t.datetime_complete.is_some()));
    check_dense_render(&state);
}
