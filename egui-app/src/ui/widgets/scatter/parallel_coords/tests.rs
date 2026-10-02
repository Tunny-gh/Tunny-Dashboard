use super::*;

struct RenderedPcp {
    labels: Vec<(egui::Rect, egui::epaint::TextShape)>,
    axes: Vec<[egui::Pos2; 2]>,
    fresh: Vec<std::sync::Arc<egui::Galley>>,
    output: egui::FullOutput,
}

fn render_pcp(
    ctx: &egui::Context,
    chart: &mut ParallelCoordsChart,
    names: &[String],
    size: egui::Vec2,
    events: Vec<egui::Event>,
    scale: f32,
) -> RenderedPcp {
    use std::{collections::HashMap, sync::Arc};
    use tunny_core::dataframe::{DataFrame, TrialRow};
    let rows: Vec<_> = (0..3)
        .map(|i| TrialRow {
            trial_id: i,
            trial_number: i,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: vec![i as f64; names.len()],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let view = crate::state::app_state::StudyView::new(
        Arc::new(DataFrame::from_trials(&rows, &[], names, &[], &[], 0)),
        vec![0; 3],
    );
    let transform = egui::emath::TSTransform::new(egui::vec2(30.0, 20.0), scale);
    let mut fresh = Vec::new();
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(2000.0, 1500.0),
            )),
            events,
            max_texture_side: Some(1024),
            ..Default::default()
        },
        |root| {
            let ctx = root.ctx();
            // Allocate and clip the fixed child just as the canvas item does.
            let area = egui::Area::new(egui::Id::new("pcp_test_canvas"))
                .fixed_pos(egui::pos2(20.0, 20.0))
                .constrain(false)
                .fade_in(false)
                .show(ctx, |ui| {
                    let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), size);
                    ui.set_clip_rect(rect);
                    ui.allocate_rect(rect, egui::Sense::click());
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    chart.show(
                        &mut child,
                        &view,
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
                                COLOR_CHART_TEXT(),
                            )
                        })
                        .collect();
                });
            ctx.set_transform_layer(area.response.layer_id, transform);
        },
    );
    fn collect(
        shape: &egui::Shape,
        clip: egui::Rect,
        labels: &mut Vec<(egui::Rect, egui::epaint::TextShape)>,
        axes: &mut Vec<[egui::Pos2; 2]>,
    ) {
        match shape {
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, clip, labels, axes);
                }
            }
            egui::Shape::Text(text) if text.galley.job.sections[0].format.font_id.size == 10.0 => {
                labels.push((clip, text.clone()));
            }
            egui::Shape::LineSegment { points, stroke }
                if stroke.color == COLOR_PARALLEL_AXIS() =>
            {
                axes.push(*points)
            }
            _ => {}
        }
    }
    let mut labels = Vec::new();
    let mut axes = Vec::new();
    for shape in &output.shapes {
        collect(&shape.shape, shape.clip_rect, &mut labels, &mut axes);
    }
    RenderedPcp {
        labels,
        axes,
        fresh,
        output,
    }
}

fn pcp_names(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("axis_{i}_long_parameter_name"))
        .collect()
}

fn assert_labels_fit(render: &RenderedPcp, count: usize) {
    assert_eq!(render.labels.len(), count);
    for (clip, label) in &render.labels {
        assert!(
            clip.contains_rect(label.visual_bounding_rect()),
            "{}: bounds {:?} outside {:?}",
            label.galley.job.text,
            label.visual_bounding_rect(),
            clip
        );
    }
}

fn assert_current_galleys(render: &RenderedPcp) {
    for (_, label) in &render.labels {
        let fresh = render
            .fresh
            .iter()
            .find(|g| g.job.text == label.galley.job.text)
            .unwrap();
        assert_eq!(label.galley.pixels_per_point, fresh.pixels_per_point);
        let uvs = |g: &egui::Galley| {
            g.rows
                .iter()
                .flat_map(|r| r.row.visuals.mesh.vertices.iter().map(|v| v.uv))
                .collect::<Vec<_>>()
        };
        assert!(
            uvs(&label.galley) == uvs(fresh),
            "stale atlas coordinates for {}",
            label.galley.job.text
        );
    }
}

fn check_label_clip(n: usize, width: f32) {
    let ctx = egui::Context::default();
    let mut chart = ParallelCoordsChart::default();
    let names = pcp_names(n);
    render_pcp(
        &ctx,
        &mut chart,
        &names,
        egui::vec2(width, 450.0),
        vec![],
        1.25,
    );
    let render = render_pcp(
        &ctx,
        &mut chart,
        &names,
        egui::vec2(width, 450.0),
        vec![],
        1.25,
    );
    assert_eq!(render.labels[0].1.angle != 0.0, n > 2);
    assert_labels_fit(&render, n);
}

#[test]
fn pcp_horizontal_labels_fit_clip() {
    check_label_clip(2, 900.0);
}

#[test]
fn pcp_rotated_labels_fit_clip() {
    check_label_clip(7, 650.0);
}

#[test]
fn pcp_pointer_movement_does_not_change_labels() {
    for n in [2, 7] {
        let ctx = egui::Context::default();
        let mut chart = ParallelCoordsChart::default();
        let names = pcp_names(n);
        let size = egui::vec2(900.0, 450.0);
        render_pcp(&ctx, &mut chart, &names, size, vec![], 1.25);
        let baseline = render_pcp(&ctx, &mut chart, &names, size, vec![], 1.25);
        for point in [
            egui::pos2(100.0, 100.0),
            egui::pos2(400.0, 300.0),
            egui::pos2(1100.0, 450.0),
        ] {
            let frame = render_pcp(
                &ctx,
                &mut chart,
                &names,
                size,
                vec![egui::Event::PointerMoved(point)],
                1.25,
            );
            assert!(
                frame.labels == baseline.labels,
                "pointer movement changed labels"
            );
            assert!(chart.brush_ranges.is_empty());
            let primitives = ctx.tessellate(frame.output.shapes, frame.output.pixels_per_point);
            assert!(!primitives.is_empty());
        }
    }
}

#[test]
fn pcp_galleys_refresh_after_atlas_reset_and_dpi_change() {
    for n in [2, 7] {
        let ctx = egui::Context::default();
        let mut chart = ParallelCoordsChart::default();
        let names = pcp_names(n);
        let size = egui::vec2(900.0, 450.0);
        render_pcp(&ctx, &mut chart, &names, size, vec![], 1.0);
        let before = render_pcp(&ctx, &mut chart, &names, size, vec![], 1.0);
        assert_current_galleys(&before);
        ctx.all_styles_mut(|style| {
            style.visuals.text_options.font_hinting = !style.visuals.text_options.font_hinting
        });
        let after = render_pcp(
            &ctx,
            &mut chart,
            &names,
            size,
            vec![egui::Event::PointerMoved(egui::pos2(300.0, 200.0))],
            1.0,
        );
        assert!(
            after
                .output
                .textures_delta
                .set
                .iter()
                .any(|(_, delta)| delta.pos.is_none()),
            "atlas was not recreated"
        );
        assert_current_galleys(&after);
        ctx.set_pixels_per_point(2.0);
        let dpi = render_pcp(&ctx, &mut chart, &names, size, vec![], 1.0);
        assert_current_galleys(&dpi);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.set_theme(theme);
            let themed = render_pcp(&ctx, &mut chart, &names, size, vec![], 1.0);
            assert_current_galleys(&themed);
            assert_labels_fit(&themed, n);
        }
    }
}

#[test]
fn pcp_galleys_refresh_after_full_atlas() {
    let ctx = egui::Context::default();
    let mut chart = ParallelCoordsChart::default();
    let names = pcp_names(7);
    let size = egui::vec2(900.0, 450.0);
    render_pcp(&ctx, &mut chart, &names, size, vec![], 1.0);
    render_pcp(&ctx, &mut chart, &names, size, vec![], 1.0);
    let mut fill = 0.0;
    let _ = ctx.run_ui(
        egui::RawInput {
            max_texture_side: Some(1024),
            ..Default::default()
        },
        |ui| {
            let text: String = ('!'..='~').collect();
            for font_size in 12..100 {
                let _ = ui.painter().layout_no_wrap(
                    text.clone(),
                    egui::FontId::proportional(font_size as f32),
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
    let reset = render_pcp(
        &ctx,
        &mut chart,
        &names,
        size,
        vec![egui::Event::PointerMoved(egui::pos2(400.0, 300.0))],
        1.0,
    );
    assert!(reset
        .output
        .textures_delta
        .set
        .iter()
        .any(|(_, delta)| delta.pos.is_none()));
    assert_current_galleys(&reset);
    assert_labels_fit(&reset, 7);
}

#[test]
fn pcp_resize_show_hide_and_long_label_band() {
    let ctx = egui::Context::default();
    let mut chart = ParallelCoordsChart::default();
    let names = pcp_names(7);
    render_pcp(
        &ctx,
        &mut chart,
        &names,
        egui::vec2(1200.0, 450.0),
        vec![],
        0.8,
    );
    for (width, hidden) in [
        (1200.0, false),
        (650.0, false),
        (650.0, true),
        (1200.0, true),
        (1200.0, false),
    ] {
        for name in &names[1..6] {
            chart.axis_visibility.insert(name.clone(), !hidden);
        }
        let frame = render_pcp(
            &ctx,
            &mut chart,
            &names,
            egui::vec2(width, 450.0),
            vec![],
            0.8,
        );
        assert_labels_fit(&frame, if hidden { 2 } else { 7 });
        assert_current_galleys(&frame);
        assert_eq!(
            frame
                .labels
                .iter()
                .map(|(_, t)| t.galley.job.text.clone())
                .collect::<Vec<_>>(),
            names
                .iter()
                .filter(|name| !hidden || *name == &names[0] || *name == &names[6])
                .cloned()
                .collect::<Vec<_>>()
        );
        let again = render_pcp(
            &ctx,
            &mut chart,
            &names,
            egui::vec2(width, 450.0),
            vec![],
            0.8,
        );
        assert!(frame.labels == again.labels);
    }
    let long_names: Vec<_> = (0..8)
        .map(|i| format!("axis_{i}_{}", "long_name_".repeat(6)))
        .collect();
    let long = render_pcp(
        &ctx,
        &mut chart,
        &long_names,
        egui::vec2(950.0, 650.0),
        vec![],
        1.0,
    );
    assert!(long.labels[0].1.visual_bounding_rect().height() > 110.0);
    assert_labels_fit(&long, 8);
}

#[test]
fn pcp_cramped_layout_has_ordered_finite_geometry() {
    let ctx = egui::Context::default();
    let mut chart = ParallelCoordsChart::default();
    let names = pcp_names(7);
    for size in [
        egui::vec2(60.0, 150.0),
        egui::vec2(250.0, 70.0),
        egui::vec2(60.0, 20.0),
    ] {
        let frame = render_pcp(&ctx, &mut chart, &names, size, vec![], 1.0);
        for axis in &frame.axes {
            assert!(axis[0].is_finite() && axis[1].is_finite());
            assert!(axis[0].y < axis[1].y);
        }
        assert!(frame.axes.windows(2).all(|a| a[0][0].x <= a[1][0].x));
        let primitives = ctx.tessellate(frame.output.shapes, frame.output.pixels_per_point);
        for primitive in primitives {
            if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                assert!(mesh.vertices.iter().all(|v| v.pos.is_finite()));
            }
        }
    }
}

#[test]
fn pcp_brush_create_move_hover_and_clear_unchanged() {
    for n in [2, 7] {
        let ctx = egui::Context::default();
        let mut chart = ParallelCoordsChart::default();
        let names = pcp_names(n);
        let size = egui::vec2(900.0, 450.0);
        render_pcp(&ctx, &mut chart, &names, size, vec![], 1.25);
        let baseline = render_pcp(&ctx, &mut chart, &names, size, vec![], 1.25);
        assert_eq!(baseline.axes.len(), n);
        let axis = baseline.axes[0];
        let point = |norm| {
            egui::pos2(
                axis[0].x,
                normalized_to_screen_y(norm, axis[0].y, axis[1].y),
            )
        };
        let button = |norm, button, pressed| egui::Event::PointerButton {
            pos: point(norm),
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut frame = |events| render_pcp(&ctx, &mut chart, &names, size, events, 1.25);
        frame(vec![
            egui::Event::PointerMoved(point(0.2)),
            button(0.2, egui::PointerButton::Primary, true),
        ]);
        frame(vec![egui::Event::PointerMoved(point(0.25))]);
        frame(vec![egui::Event::PointerMoved(point(0.7))]);
        frame(vec![button(0.7, egui::PointerButton::Primary, false)]);
        let created = chart.brush_ranges[&names[0]].unwrap();
        assert!(
            (created.0 - 0.25).abs() < 1e-5 && (created.1 - 0.7).abs() < 1e-5,
            "{created:?}"
        );
        assert_eq!(chart.pending_selection.take(), Some(vec![1]));
        let hover = render_pcp(
            &ctx,
            &mut chart,
            &names,
            size,
            vec![egui::Event::PointerMoved(point(0.5))],
            1.25,
        );
        assert_eq!(
            hover.output.platform_output.cursor_icon,
            egui::CursorIcon::Grab
        );
        assert!(hover.labels == baseline.labels);
        let mut frame = |events| render_pcp(&ctx, &mut chart, &names, size, events, 1.25);
        frame(vec![button(0.5, egui::PointerButton::Primary, true)]);
        frame(vec![egui::Event::PointerMoved(point(0.55))]);
        let moving = frame(vec![egui::Event::PointerMoved(point(0.65))]);
        assert_eq!(
            moving.output.platform_output.cursor_icon,
            egui::CursorIcon::Grabbing
        );
        frame(vec![button(0.65, egui::PointerButton::Primary, false)]);
        let moved = chart.brush_ranges[&names[0]].unwrap();
        assert!(
            (moved.0 - 0.35).abs() < 1e-5 && (moved.1 - 0.8).abs() < 1e-5,
            "{moved:?}"
        );
        assert_eq!(chart.pending_selection.take(), Some(vec![1]));
        render_pcp(
            &ctx,
            &mut chart,
            &names,
            size,
            vec![button(0.5, egui::PointerButton::Secondary, true)],
            1.25,
        );
        render_pcp(
            &ctx,
            &mut chart,
            &names,
            size,
            vec![button(0.5, egui::PointerButton::Secondary, false)],
            1.25,
        );
        assert!(chart.brush_ranges.is_empty());
        assert_eq!(chart.pending_selection, Some(vec![]));
    }
}

#[test]
fn normalize_min_maps_to_zero() {
    let n = normalize_value(0.0, 0.0, 10.0);
    assert!((n - 0.0).abs() < 1e-6);
}

#[test]
fn normalize_max_maps_to_one() {
    let n = normalize_value(10.0, 0.0, 10.0);
    assert!((n - 1.0).abs() < 1e-6);
}

#[test]
fn normalize_equal_min_max_returns_half() {
    let n = normalize_value(5.0, 5.0, 5.0);
    assert!((n - 0.5).abs() < 1e-6);
}

#[test]
fn normalize_clamps_out_of_range() {
    let below = normalize_value(-1.0, 0.0, 1.0);
    let above = normalize_value(2.0, 0.0, 1.0);
    assert!((below - 0.0).abs() < 1e-6);
    assert!((above - 1.0).abs() < 1e-6);
}

#[test]
fn normalized_to_screen_y_zero_maps_to_bottom() {
    let y = normalized_to_screen_y(0.0, 100.0, 400.0);
    assert!((y - 400.0).abs() < 1e-3);
}

#[test]
fn normalized_to_screen_y_one_maps_to_top() {
    let y = normalized_to_screen_y(1.0, 100.0, 400.0);
    assert!((y - 100.0).abs() < 1e-3);
}

#[test]
fn build_axis_order_concatenates_params_then_objectives() {
    let params = vec!["x".to_string(), "y".to_string()];
    let objs = vec!["obj0".to_string()];
    let axes = build_axis_order(&params, &objs);
    assert_eq!(axes, vec!["x", "y", "obj0"]);
}

#[test]
fn parallel_coords_chart_default() {
    let chart = ParallelCoordsChart::default();
    assert!(chart.axis_order.is_empty());
    assert!(chart.show_params);
    assert!(chart.show_objectives);
    assert!(chart.brush_ranges.is_empty());
    assert!(chart.drag_start.is_none());
    assert!(chart.color_axis.is_none());
}

// TASK-2022 tests

#[test]
fn ordered_brush_range_forward_drag() {
    let (min, max) = ordered_brush_range(0.2, 0.8);
    assert!((min - 0.2).abs() < 1e-6);
    assert!((max - 0.8).abs() < 1e-6);
}

#[test]
fn ordered_brush_range_reverse_drag() {
    // Dragging upward: start > end
    let (min, max) = ordered_brush_range(0.8, 0.2);
    assert!((min - 0.2).abs() < 1e-6);
    assert!((max - 0.8).abs() < 1e-6);
}

// TASK-2125 tests
#[test]
fn axis_visibility_filter() {
    use std::collections::HashMap;
    let mut visibility: HashMap<String, bool> = HashMap::new();
    visibility.insert("x1".to_string(), true);
    visibility.insert("x2".to_string(), false);
    visibility.insert("x3".to_string(), true);
    let axis_order = ["x1".to_string(), "x2".to_string(), "x3".to_string()];
    let visible: Vec<_> = axis_order
        .iter()
        .filter(|name| *visibility.get(*name).unwrap_or(&true))
        .collect();
    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0], "x1");
    assert_eq!(visible[1], "x3");
}

#[test]
fn axis_reorder_logic() {
    let mut axis_order = vec!["x1".to_string(), "x2".to_string(), "x3".to_string()];
    let dragged = "x1";
    let target_idx = 2;
    if let Some(from_idx) = axis_order.iter().position(|a| a == dragged) {
        let name = axis_order.remove(from_idx);
        let insert_at = target_idx.min(axis_order.len());
        axis_order.insert(insert_at, name);
    }
    assert_eq!(axis_order, vec!["x2", "x3", "x1"]);
}

#[test]
fn axis_visibility_all_hidden() {
    use std::collections::HashMap;
    let mut visibility: HashMap<String, bool> = HashMap::new();
    visibility.insert("x1".to_string(), false);
    visibility.insert("x2".to_string(), false);
    let axis_order = ["x1".to_string(), "x2".to_string()];
    let visible: Vec<_> = axis_order
        .iter()
        .filter(|name| *visibility.get(*name).unwrap_or(&true))
        .collect();
    assert!(visible.is_empty());
}

#[test]
fn axis_visibility_default_true_for_unknown() {
    use std::collections::HashMap;
    let visibility: HashMap<String, bool> = HashMap::new();
    let axis_order = ["unknown_axis".to_string()];
    let visible: Vec<_> = axis_order
        .iter()
        .filter(|name| *visibility.get(*name).unwrap_or(&true))
        .collect();
    assert_eq!(visible.len(), 1);
}

// ── constraint-aware visualization (TASK-2349) ──────────────────

#[test]
fn tc_cav_parallel_coords_show_infeasible_default_true() {
    let chart = ParallelCoordsChart::default();
    assert!(chart.show_infeasible);
}

// --- TASK-2242: PCP brush tests ---

#[test]
fn multi_axis_brush_applies_and_filter() {
    use std::collections::HashMap;
    let trial_ids = vec![0u32, 1, 2];
    // col_data: axis 0 = x, axis 1 = obj
    let col_data = [
        vec![2.0, 8.0, 2.0], // x values
        vec![5.0, 5.0, 9.0], // obj values
    ];
    let cols: Vec<Option<&[f64]>> =
        vec![Some(col_data[0].as_slice()), Some(col_data[1].as_slice())];
    let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
    let all_names = vec!["x".to_string(), "obj".to_string()];

    let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    // x in [0.0, 0.5] = values 0..5 → trial 0 and 2 pass
    brush_ranges.insert("x".to_string(), Some((0.0, 0.5)));
    // obj in [0.0, 0.6] = values 0..6 → trial 0 passes; trial 2 (obj=9) fails
    brush_ranges.insert("obj".to_string(), Some((0.0, 0.6)));

    let sel = filter_trials_by_brushes(&trial_ids, &brush_ranges, &cols, &col_ranges, &all_names);
    assert_eq!(sel.len(), 1);
    assert_eq!(sel[0], 0);
}

#[test]
fn shifted_brush_range_moves_within_bounds() {
    let (lo, hi) = shifted_brush_range((0.2, 0.5), 0.1);
    assert!((lo - 0.3).abs() < 1e-6);
    assert!((hi - 0.6).abs() < 1e-6);
}

#[test]
fn shifted_brush_range_clamps_at_top() {
    // width 0.3, shift up by 0.4 → would exceed 1.0, clamp so hi == 1.0
    let (lo, hi) = shifted_brush_range((0.5, 0.8), 0.4);
    assert!((hi - 1.0).abs() < 1e-6);
    assert!((lo - 0.7).abs() < 1e-6); // width preserved
}

#[test]
fn shifted_brush_range_clamps_at_bottom() {
    // shift down past 0 → clamp so lo == 0.0, width preserved
    let (lo, hi) = shifted_brush_range((0.2, 0.5), -0.4);
    assert!((lo - 0.0).abs() < 1e-6);
    assert!((hi - 0.3).abs() < 1e-6);
}

#[test]
fn shifted_brush_range_preserves_width() {
    let orig = (0.1_f32, 0.6_f32);
    let (lo, hi) = shifted_brush_range(orig, 0.25);
    assert!(((hi - lo) - (orig.1 - orig.0)).abs() < 1e-6);
}

#[test]
fn trial_passes_brushes_no_active_brush_passes() {
    use std::collections::HashMap;
    let col_data = [vec![2.0, 8.0], vec![5.0, 9.0]];
    let cols: Vec<Option<&[f64]>> =
        vec![Some(col_data[0].as_slice()), Some(col_data[1].as_slice())];
    let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
    let all_names = vec!["x".to_string(), "obj".to_string()];
    // No brush set (None only) -> everything passes.
    let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    brush_ranges.insert("x".to_string(), None);
    assert!(trial_passes_brushes(
        0,
        &brush_ranges,
        &cols,
        &col_ranges,
        &all_names
    ));
}

#[test]
fn trial_passes_brushes_missing_value_with_active_brush_excluded() {
    use std::collections::HashMap;
    // axis 1 has only one value, so t_idx=1 is missing.
    let col_data_x = vec![2.0, 8.0];
    let col_data_obj = vec![5.0];
    let cols: Vec<Option<&[f64]>> =
        vec![Some(col_data_x.as_slice()), Some(col_data_obj.as_slice())];
    let col_ranges = vec![(0.0_f64, 10.0_f64), (0.0_f64, 10.0_f64)];
    let all_names = vec!["x".to_string(), "obj".to_string()];
    let mut brush_ranges: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    brush_ranges.insert("obj".to_string(), Some((0.0, 1.0)));
    // t_idx=1 is missing the obj value -> fails since the brush is active.
    assert!(!trial_passes_brushes(
        1,
        &brush_ranges,
        &cols,
        &col_ranges,
        &all_names
    ));
}

#[test]
fn visible_axis_indices_default_all_visible() {
    use std::collections::HashMap;
    let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let vis = HashMap::new(); // unregistered = all visible
    assert_eq!(visible_axis_indices(&names, &vis), vec![0, 1, 2]);
}

#[test]
fn visible_axis_indices_filters_hidden_and_preserves_order() {
    use std::collections::HashMap;
    let names = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut vis = HashMap::new();
    vis.insert("b".to_string(), false);
    assert_eq!(visible_axis_indices(&names, &vis), vec![0, 2]);
}

#[test]
fn visible_axis_indices_all_hidden_is_empty() {
    use std::collections::HashMap;
    let names = vec!["a".to_string(), "b".to_string()];
    let mut vis = HashMap::new();
    vis.insert("a".to_string(), false);
    vis.insert("b".to_string(), false);
    assert!(visible_axis_indices(&names, &vis).is_empty());
}

#[test]
fn feasible_color_range_excludes_infeasible_outliers() {
    use tunny_core::dataframe::Feasibility;
    // The infeasible solution (idx 3) has an outlier value of 1000.0, but
    // the range is computed from feasible solutions only.
    let col = [1.0, 2.0, 3.0, 1000.0];
    let feas_col = [1.0, 1.0, 1.0, 0.0];
    let feas = Feasibility::from_column(Some(&feas_col));
    let (mn, mx) = feasible_color_range(&col, feas, (0.0, 9999.0));
    assert_eq!(mn, 1.0);
    assert_eq!(mx, 3.0);
}

#[test]
fn feasible_color_range_no_constraints_uses_all() {
    use tunny_core::dataframe::Feasibility;
    let col = [1.0, 2.0, 3.0, 1000.0];
    let feas = Feasibility::from_column(None);
    let (mn, mx) = feasible_color_range(&col, feas, (0.0, 9999.0));
    assert_eq!(mn, 1.0);
    assert_eq!(mx, 1000.0);
}

#[test]
fn feasible_color_range_all_infeasible_falls_back() {
    use tunny_core::dataframe::Feasibility;
    let col = [1.0, 2.0, 3.0];
    let feas_col = [0.0, 0.0, 0.0];
    let feas = Feasibility::from_column(Some(&feas_col));
    let range = feasible_color_range(&col, feas, (-5.0, 5.0));
    assert_eq!(range, (-5.0, 5.0));
}

#[test]
fn feasible_color_range_skips_non_finite() {
    use tunny_core::dataframe::Feasibility;
    let col = [1.0, f64::NAN, f64::INFINITY, 4.0];
    let feas_col = [1.0, 1.0, 1.0, 1.0];
    let feas = Feasibility::from_column(Some(&feas_col));
    let (mn, mx) = feasible_color_range(&col, feas, (0.0, 0.0));
    assert_eq!(mn, 1.0);
    assert_eq!(mx, 4.0);
}
