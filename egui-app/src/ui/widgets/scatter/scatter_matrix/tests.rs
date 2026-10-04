use super::draw::data_to_screen;
use super::stats::{
    compute_correlation, compute_histogram, downsample_indices_to_cap, resolve_color_objective,
    split_feasibility_indices,
};
use super::*;

struct RenderedMatrix {
    labels: Vec<egui::epaint::TextShape>,
    fresh: Vec<std::sync::Arc<egui::Galley>>,
    output: egui::FullOutput,
}

fn render_matrix(
    ctx: &egui::Context,
    chart: &mut ScatterMatrix,
    view: &crate::state::app_state::StudyView,
    names: &[String],
    rotated: bool,
    prime_atlas: bool,
) -> RenderedMatrix {
    let mut fresh = Vec::new();
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            max_texture_side: Some(1024),
            ..Default::default()
        },
        |ui| {
            if prime_atlas {
                // Another widget allocates glyphs first after the pressure reset,
                // so rebuilding cannot accidentally reuse the old label coordinates.
                let _ = ui.painter().layout_no_wrap(
                    "Other widget glyphs".into(),
                    egui::FontId::proportional(23.0),
                    egui::Color32::WHITE,
                );
            }
            let rect = egui::Rect::from_min_size(
                egui::pos2(20.0, 20.0),
                egui::vec2(if rotated { 300.0 } else { 1000.0 }, 600.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            chart.show(
                &mut child,
                view,
                &[],
                names,
                &crate::theme::colormap::ColorMap::viridis(),
            );
            fresh = names
                .iter()
                .map(|name| {
                    child.painter().layout_no_wrap(
                        name.clone(),
                        egui::FontId::proportional(10.0),
                        child.visuals().text_color(),
                    )
                })
                .collect();
        },
    );
    fn collect(shape: &egui::Shape, names: &[String], labels: &mut Vec<egui::epaint::TextShape>) {
        match shape {
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, names, labels);
                }
            }
            egui::Shape::Text(text)
                if names.contains(&text.galley.job.text)
                    && text.galley.job.sections[0].format.font_id.size == 10.0 =>
            {
                labels.push(text.clone());
            }
            _ => {}
        }
    }
    let mut labels = Vec::new();
    for shape in &output.shapes {
        collect(&shape.shape, names, &mut labels);
    }
    RenderedMatrix {
        labels,
        fresh,
        output,
    }
}

fn assert_current_matrix_labels(render: &RenderedMatrix, rotated: bool) {
    assert_eq!(render.labels.len(), render.fresh.len() * 2);
    for (pair, fresh) in render.labels.chunks_exact(2).zip(&render.fresh) {
        // Each axis is painted once as a column header and once as a row header.
        for label in pair {
            assert_eq!(
                label.angle,
                if rotated {
                    -std::f32::consts::FRAC_PI_4
                } else {
                    0.0
                }
            );
            assert!(
                label.galley.as_ref() == fresh.as_ref(),
                "stale label: {} (rendered DPI {}, fresh DPI {})",
                fresh.job.text,
                label.galley.pixels_per_point,
                fresh.pixels_per_point
            );
        }
    }
}

#[derive(Clone, Copy)]
enum LabelRefresh {
    TextOptions,
    AtlasPressure,
    Dpi,
    Theme,
}

fn check_matrix_label_refresh(rotated: bool, trigger: LabelRefresh) {
    use std::{collections::HashMap, sync::Arc};
    use tunny_core::dataframe::{DataFrame, TrialRow};
    let names: Vec<_> = (0..2)
        .map(|i| format!("axis_{i}_long_parameter_name"))
        .collect();
    let rows: Vec<_> = (0..3)
        .map(|i| TrialRow {
            trial_id: i,
            trial_number: i,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![i as f64; names.len()],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let view = crate::state::app_state::StudyView::new(
        Arc::new(DataFrame::from_trials(&rows, &[], &names, &[], &[], 0)),
        vec![0; 3],
    );
    let ctx = egui::Context::default();
    ctx.set_theme(egui::Theme::Dark);
    let mut chart = ScatterMatrix::default();
    render_matrix(&ctx, &mut chart, &view, &names, rotated, false);
    let before = render_matrix(&ctx, &mut chart, &view, &names, rotated, false);
    assert_current_matrix_labels(&before, rotated);
    match trigger {
        LabelRefresh::TextOptions => ctx.all_styles_mut(|style| {
            style.visuals.text_options.font_hinting = !style.visuals.text_options.font_hinting;
        }),
        LabelRefresh::AtlasPressure => {
            let mut fill = 0.0;
            let _ = ctx.run_ui(
                egui::RawInput {
                    max_texture_side: Some(1024),
                    ..Default::default()
                },
                |ui| {
                    let text: String = ('!'..='~').collect();
                    for size in 12..100 {
                        let _ = ui.painter().layout_no_wrap(
                            text.clone(),
                            egui::FontId::proportional(size as f32),
                            egui::Color32::WHITE,
                        );
                        fill = ctx.fonts(|fonts| fonts.font_atlas_fill_ratio());
                        if fill > 0.8 {
                            break;
                        }
                    }
                },
            );
            assert!(fill > 0.8, "atlas pressure was not reached: {fill}");
        }
        LabelRefresh::Dpi => ctx.set_pixels_per_point(2.0),
        LabelRefresh::Theme => ctx.set_theme(egui::Theme::Light),
    }
    let prime_atlas = matches!(trigger, LabelRefresh::AtlasPressure);
    let after = render_matrix(&ctx, &mut chart, &view, &names, rotated, prime_atlas);
    match trigger {
        LabelRefresh::TextOptions | LabelRefresh::AtlasPressure => {
            assert!(
                after.output.textures_delta.set.iter().any(|(id, delta)| {
                    *id == egui::TextureId::Managed(0) && delta.pos.is_none()
                }),
                "font atlas was not recreated"
            );
            let uvs = |g: &egui::Galley| {
                g.rows
                    .iter()
                    .flat_map(|r| r.row.visuals.mesh.vertices.iter().map(|v| v.uv))
                    .collect::<Vec<_>>()
            };
            assert_ne!(
                uvs(&before.fresh[0]),
                uvs(&after.fresh[0]),
                "atlas coordinates did not change"
            );
        }
        LabelRefresh::Dpi => {
            assert_eq!(before.fresh[0].pixels_per_point, 1.0);
            assert_eq!(after.output.pixels_per_point, 2.0);
            assert_eq!(after.fresh[0].pixels_per_point, 2.0);
        }
        LabelRefresh::Theme => assert_ne!(
            before.fresh[0].job.sections[0].format.color,
            after.fresh[0].job.sections[0].format.color,
            "theme text color did not change",
        ),
    }
    assert_current_matrix_labels(&after, rotated);
    let again = render_matrix(&ctx, &mut chart, &view, &names, rotated, prime_atlas);
    assert_current_matrix_labels(&again, rotated);
    assert_eq!(after.labels, again.labels);
}

macro_rules! matrix_label_refresh_test {
    ($name:ident, $rotated:expr, $trigger:ident) => {
        #[test]
        fn $name() {
            check_matrix_label_refresh($rotated, LabelRefresh::$trigger);
        }
    };
}

matrix_label_refresh_test!(matrix_labels_horizontal_text_options, false, TextOptions);
matrix_label_refresh_test!(matrix_labels_rotated_text_options, true, TextOptions);
matrix_label_refresh_test!(
    matrix_labels_horizontal_atlas_pressure,
    false,
    AtlasPressure
);
matrix_label_refresh_test!(matrix_labels_rotated_atlas_pressure, true, AtlasPressure);
matrix_label_refresh_test!(matrix_labels_horizontal_dpi, false, Dpi);
matrix_label_refresh_test!(matrix_labels_rotated_dpi, true, Dpi);
matrix_label_refresh_test!(matrix_labels_horizontal_theme, false, Theme);
matrix_label_refresh_test!(matrix_labels_rotated_theme, true, Theme);

// ── resolve_color_objective ──────────────────────────────────────

#[test]
fn resolve_color_objective_none_returns_first() {
    let names = vec!["obj0".to_string(), "obj1".to_string()];
    assert_eq!(resolve_color_objective(&None, &names), Some("obj0"));
}

#[test]
fn resolve_color_objective_existing_name_returns_it() {
    let names = vec!["obj0".to_string(), "obj1".to_string()];
    assert_eq!(
        resolve_color_objective(&Some("obj1".to_string()), &names),
        Some("obj1")
    );
}

#[test]
fn resolve_color_objective_unknown_name_falls_back_to_first() {
    let names = vec!["obj0".to_string(), "obj1".to_string()];
    assert_eq!(
        resolve_color_objective(&Some("unknown".to_string()), &names),
        Some("obj0")
    );
}

#[test]
fn resolve_color_objective_empty_names_returns_none() {
    assert_eq!(resolve_color_objective(&None, &[]), None);
    assert_eq!(
        resolve_color_objective(&Some("obj0".to_string()), &[]),
        None
    );
}

// ── constraint-aware visualization (TASK-2350) ──────────────────

#[test]
fn tc_cav_scatter_matrix_show_infeasible_default_true() {
    let sm = ScatterMatrix::default();
    assert!(sm.show_infeasible);
}

#[test]
fn tc_cav_split_feasibility_no_constraints_all_feasible() {
    use tunny_core::dataframe::Feasibility;
    let feas = Feasibility::from_column(None);
    let (f, inf, unverified) = split_feasibility_indices(3, feas);
    assert!(unverified.is_empty());
    assert_eq!(f, vec![0, 1, 2]);
    assert!(inf.is_empty());
}

#[test]
fn tc_cav_split_feasibility_mixed() {
    use tunny_core::dataframe::Feasibility;
    let col = vec![1.0_f64, 0.0, 1.0];
    let feas = Feasibility::from_column(Some(&col));
    let (f, inf, unverified) = split_feasibility_indices(3, feas);
    assert!(unverified.is_empty());
    assert_eq!(f, vec![0, 2]);
    assert_eq!(inf, vec![1]);
}

#[test]
fn unverified_is_not_partitioned_as_infeasible() {
    let col = [1.0, f64::NAN, 0.0];
    let feas = tunny_core::dataframe::Feasibility::from_column(Some(&col));
    assert_eq!(
        split_feasibility_indices(3, feas),
        (vec![0], vec![2], vec![1])
    );
    let mut rows = Vec::new();
    crate::ui::widgets::trial_detail_modal::push_feasible_row(&mut rows, feas, 1);
    assert_eq!(
        rows,
        vec![("Feasibility".into(), "Feasibility unverified".into())]
    );
}

#[test]
fn tc_cav_split_feasibility_all_infeasible() {
    use tunny_core::dataframe::Feasibility;
    let col = vec![0.0_f64, 0.0];
    let feas = Feasibility::from_column(Some(&col));
    let (f, inf, unverified) = split_feasibility_indices(2, feas);
    assert!(unverified.is_empty());
    assert!(f.is_empty());
    assert_eq!(inf, vec![0, 1]);
}

// TASK-2019 tests

#[test]
fn scatter_matrix_default_mode() {
    let sm = ScatterMatrix::default();
    assert_eq!(sm.mode, MatrixMode::ParamsVsParams);
    assert_eq!(sm.sort, AxisSort::Alphabetical);
    assert!(sm.selected_cell.is_none());
}

#[test]
fn downsample_cap_keeps_all_when_under_cap() {
    let idx: Vec<u32> = (0..100).collect();
    let out = downsample_indices_to_cap(&idx, 4000);
    assert_eq!(out, idx);
}

#[test]
fn downsample_cap_limits_when_over_cap() {
    let idx: Vec<u32> = (0..100_000).collect();
    let out = downsample_indices_to_cap(&idx, 4000);
    assert!(out.len() <= 4000, "got {}", out.len());
    assert!(!out.is_empty());
    // The first element is preserved, and downsampling keeps ascending order.
    assert_eq!(out[0], 0);
    assert!(out.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn downsample_cap_zero_is_empty() {
    let idx: Vec<u32> = (0..10).collect();
    assert!(downsample_indices_to_cap(&idx, 0).is_empty());
}

#[test]
fn compute_histogram_bins_count() {
    let data = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let bins = compute_histogram(&data, 5);
    assert_eq!(bins.len(), 5);
    let total: usize = bins.iter().sum();
    assert_eq!(total, data.len());
}

#[test]
fn compute_histogram_all_in_same_bin() {
    let data = vec![5.0; 10];
    let bins = compute_histogram(&data, 4);
    let total: usize = bins.iter().sum();
    assert_eq!(total, 10);
}

#[test]
fn compute_histogram_empty_data() {
    let bins = compute_histogram(&[], 5);
    assert_eq!(bins.len(), 5);
    assert!(bins.iter().all(|&b| b == 0));
}

#[test]
fn compute_correlation_perfect_positive() {
    let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let y = x.clone();
    let corr = compute_correlation(&x, &y);
    assert!((corr - 1.0).abs() < 1e-9);
}

#[test]
fn compute_correlation_perfect_negative() {
    let x: Vec<f64> = (0..10).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&v| -v).collect();
    let corr = compute_correlation(&x, &y);
    assert!((corr + 1.0).abs() < 1e-9);
}

#[test]
fn compute_correlation_range_bounded() {
    let x = vec![1.0, 3.0, 5.0, 7.0, 9.0];
    let y = vec![2.0, 1.0, 4.0, 3.0, 5.0];
    let corr = compute_correlation(&x, &y);
    assert!((-1.0..=1.0).contains(&corr));
}

#[test]
fn data_to_screen_min_maps_to_left_bottom() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
    let pos = data_to_screen(0.0, 0.0, (0.0, 1.0), (0.0, 1.0), rect);
    assert!((pos.x - 0.0).abs() < 1e-3);
    assert!((pos.y - 100.0).abs() < 1e-3); // y is inverted
}

#[test]
fn data_to_screen_max_maps_to_right_top() {
    let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
    let pos = data_to_screen(1.0, 1.0, (0.0, 1.0), (0.0, 1.0), rect);
    assert!((pos.x - 100.0).abs() < 1e-3);
    assert!((pos.y - 0.0).abs() < 1e-3); // y is inverted
}
