//! Radar chart drawn inside the trial detail modal, plus generic radar rendering
//! functions.
//!
//! Axes (vertices) are ordered objectives -> variables. Each axis's radius scale is
//! fit to the value range of the Pareto front (`pareto_rank == 0`) individuals, so
//! the outer ring (radius = 1.0) corresponds to the Pareto front maximum (the
//! envelope's upper bound). For comparison, each Pareto front individual is overlaid
//! with a thin line, and the selected trial is drawn on top in bold red.
//!
//! `egui_plot` has no polar chart support, so this draws directly with
//! `egui::Painter`. Axis scale computation ([`axis_scale`] / [`value_fraction`]) and
//! axis construction ([`build`]) are factored out as pure functions and tested
//! independently of the rendering logic.
//!
//! The rendering itself is factored into [`draw_radar`], which can be reused outside
//! the modal (e.g. by the Radar Comparison widget) as long as axis labels and a set
//! of series are supplied.

use std::f32::consts::PI;

use egui::Color32;

use crate::state::types::StudyView;
use crate::theme::{ACCENT_BLUE, ERROR_COLOR, TEXT_SECONDARY};
use crate::ui::widgets::common::range_math::finite_value_range;

/// Line color for each Pareto front individual (accent blue #3B82F6, thinned to
/// alpha ≈ 48). Overlapping lines look darker, revealing distribution density.
/// Since `from_rgba_premultiplied` is const, we specify (59,130,246) premultiplied
/// with alpha 48 directly.
const FRONT_LINE: Color32 = Color32::from_rgba_premultiplied(11, 24, 46, 48);
/// Color of the selected trial's polygon (emphasized in red).
fn selected() -> Color32 {
    ERROR_COLOR()
}
/// Color of the grid (concentric polygons + spokes).
fn grid_color() -> Color32 {
    crate::theme::chart_colors::COLOR_PARALLEL_AXIS()
}
/// Opacity used for the fan-shaped mesh fill of an emphasized series
/// ([`RadarSeries::emphasized`]). To match the look of the old `SELECTED_FILL`
/// (ERROR_COLOR thinned via premultiplied (120,34,27)), use the value that produces
/// the same result via `from_rgba_unmultiplied(color, 131)`.
const EMPHASIZED_FILL_ALPHA: u8 = 131;

/// Metadata for a single radar axis.
#[derive(Debug, Clone, PartialEq)]
pub struct RadarAxis {
    /// Axis name (objective name or variable name).
    pub name: String,
    /// True for an objective (false for a variable). Used for label color-coding.
    pub is_objective: bool,
    /// The selected trial's value (None when missing or non-finite).
    pub selected: Option<f64>,
    /// Minimum value among Pareto front individuals (used for the radius scale's lower bound).
    pub front_min: f64,
    /// Maximum value among Pareto front individuals (= the axis's envelope upper bound).
    pub front_max: f64,
}

/// Rendering data for the radar chart.
#[derive(Debug, Clone, PartialEq)]
pub struct RadarData {
    /// Axis metadata (ordered objectives -> variables).
    pub axes: Vec<RadarAxis>,
    /// Values for each Pareto front individual. Outer = individual, inner = axis
    /// values in the same order as `axes`. Missing/non-finite is None.
    pub front: Vec<Vec<Option<f64>>>,
}

/// Builds radar rendering data from a `StudyView`, ordered objectives -> variables.
///
/// Skips axes that have no finite value on the Pareto front (`pareto_rank == 0`).
/// `front` stores each Pareto front individual's (post-skip) axis values, aligned.
pub fn build(
    view: &StudyView,
    obj_names: &[String],
    param_names: &[String],
    selected_row: usize,
) -> RadarData {
    let front_rows: Vec<usize> = (0..view.row_count())
        .filter(|&i| view.pareto_rank.get(i).copied() == Some(0))
        .collect();

    // Collect (column slice, metadata) for each adopted axis, in order.
    let mut axes: Vec<RadarAxis> = Vec::with_capacity(obj_names.len() + param_names.len());
    let mut cols: Vec<&[f64]> = Vec::with_capacity(axes.capacity());
    for (names, is_objective) in [(obj_names, true), (param_names, false)] {
        for name in names {
            let Some(col) = view.numeric_column(name) else {
                continue;
            };
            let Some((lo, hi)) =
                finite_value_range(front_rows.iter().filter_map(|&r| col.get(r).copied()))
            else {
                continue;
            };
            let selected = col.get(selected_row).copied().filter(|v| v.is_finite());
            axes.push(RadarAxis {
                name: name.clone(),
                is_objective,
                selected,
                front_min: lo,
                front_max: hi,
            });
            cols.push(col);
        }
    }

    // Extract each front individual's values, aligned to the adopted axes.
    let front: Vec<Vec<Option<f64>>> = front_rows
        .iter()
        .map(|&r| {
            cols.iter()
                .map(|col| col.get(r).copied().filter(|v| v.is_finite()))
                .collect()
        })
        .collect();

    RadarData { axes, front }
}

/// Returns the axis radius scale `(lo, hi)`.
///
/// `hi` is the Pareto front maximum (outer ring = envelope upper bound). `lo` adds a
/// margin below the front minimum so front individuals appear away from the center.
/// When the front is a single point (`front_min == front_max`), it is expanded
/// symmetrically so that value lands at the mid-radius.
pub fn axis_scale(front_min: f64, front_max: f64) -> (f64, f64) {
    let span = front_max - front_min;
    if span.abs() <= f64::EPSILON {
        let pad = front_max.abs().max(1.0);
        (front_max - pad, front_max + pad)
    } else {
        (front_min - span * 0.2, front_max)
    }
}

/// Maps a value to a radius fraction. Slight overshoot beyond the range is allowed
/// and clamped on the rendering side.
pub fn value_fraction(value: f64, lo: f64, hi: f64) -> f32 {
    let span = hi - lo;
    if span.abs() <= f64::EPSILON {
        return 0.5;
    }
    ((value - lo) / span) as f32
}

/// A single series (a polygon for one trial/one individual) passed to [`draw_radar`].
pub struct RadarSeries {
    /// Line color (the fill color is also derived from this).
    pub color: Color32,
    /// Radius fraction `[0,1]` per axis (None for missing/non-finite; that axis only
    /// connects to its adjacent vertices).
    pub fractions: Vec<Option<f32>>,
    /// Line width.
    pub width: f32,
    /// When true, in addition to a bold line, draws a fan-shaped mesh fill from the
    /// center plus vertex dots (the emphasis style used for the "selected trial" in
    /// the trial detail modal).
    /// When false, draws only a thin outline (used for series that shouldn't be
    /// over-emphasized, such as Pareto front individuals or overlaid multi-trial
    /// comparisons).
    pub emphasized: bool,
}

/// Draws a generic radar chart from a list of axis labels (axis name, true if an
/// objective) and a set of series.
///
/// Draws nothing and returns `false` when there are fewer than 3 axes (showing a
/// message is the caller's responsibility, since the modal and the Radar Comparison
/// widget use different wording; this function doesn't own that). Draws and returns
/// `true` when there are 3 or more axes.
pub fn draw_radar(
    ui: &mut egui::Ui,
    axis_labels: &[(String, bool)],
    series: &[RadarSeries],
) -> bool {
    let n = axis_labels.len();
    if n < 3 {
        return false;
    }

    // Both callers (detail modal / Radar Comparison) draw a legend and caption row
    // below, so reserve space for 2 rows and respect available height as well as
    // width (to avoid clipping the legend in short canvas cells).
    let bottom_reserve =
        2.0 * (ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y);
    let side = ui
        .available_width()
        .clamp(240.0, 460.0)
        .min((ui.available_height() - bottom_reserve).max(200.0));
    let (rect, _resp) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    // Radius after subtracting margin for labels.
    let radius = side * 0.5 - 64.0;

    // Angle of vertex i (starting up, clockwise).
    let angle = |i: usize| -> f32 { -PI / 2.0 + (i as f32) * 2.0 * PI / (n as f32) };
    // Screen coordinates for vertex i at radius fraction frac (overshoot clamped at 1.12).
    let point_at = |i: usize, frac: f32| -> egui::Pos2 {
        let a = angle(i);
        let r = frac.clamp(0.0, 1.12) * radius;
        center + egui::vec2(a.cos() * r, a.sin() * r)
    };
    // Maps a series of axis values (missing = None) to screen coordinates.
    let to_points = |values: &[Option<f32>]| -> Vec<Option<egui::Pos2>> {
        (0..n)
            .map(|i| values.get(i).copied().flatten().map(|f| point_at(i, f)))
            .collect()
    };

    // ── Grid (concentric polygons + spokes) ───────────────────────
    for ring in [0.25_f32, 0.5, 0.75, 1.0] {
        let pts: Vec<egui::Pos2> = (0..n).map(|i| point_at(i, ring)).collect();
        painter.add(egui::Shape::closed_line(
            pts,
            egui::Stroke::new(1.0, grid_color()),
        ));
    }
    for i in 0..n {
        painter.line_segment(
            [center, point_at(i, 1.0)],
            egui::Stroke::new(1.0, grid_color()),
        );
    }

    // ── Series (front individuals, selected trial, pinned trials, etc.) ──
    for s in series {
        let pts = to_points(&s.fractions);
        let stroke = egui::Stroke::new(s.width, s.color);

        if s.emphasized && pts.iter().all(|p| p.is_some()) {
            // When all axes are present, fill with a fan-shaped mesh from the center
            // (reasonable since it's star-shaped around the center).
            let poly: Vec<egui::Pos2> = pts.iter().map(|p| p.unwrap()).collect();
            let fill_color = Color32::from_rgba_unmultiplied(
                s.color.r(),
                s.color.g(),
                s.color.b(),
                EMPHASIZED_FILL_ALPHA,
            );
            let mut fill = egui::Mesh::default();
            fill.colored_vertex(center, fill_color);
            for &p in &poly {
                fill.colored_vertex(p, fill_color);
            }
            for i in 0..n {
                let a = 1 + i as u32;
                let b = 1 + ((i + 1) % n) as u32;
                fill.add_triangle(0, a, b);
            }
            painter.add(egui::Shape::mesh(fill));
            painter.add(egui::Shape::closed_line(poly, stroke));
        } else {
            // When an axis is missing, connect only adjacent valid vertices with a
            // line (same behavior for non-emphasized series).
            draw_ring_polyline(&painter, &pts, stroke);
        }
        if s.emphasized {
            for p in pts.iter().flatten() {
                painter.circle_filled(*p, 3.0, s.color);
            }
        }
    }

    // ── Axis labels ────────────────────────────────────────────
    for (i, (name, is_objective)) in axis_labels.iter().enumerate() {
        let a = angle(i);
        let lp = center + egui::vec2(a.cos() * (radius + 12.0), a.sin() * (radius + 12.0));
        let align = if a.cos().abs() < 0.3 {
            egui::Align2::CENTER_CENTER
        } else if a.cos() > 0.0 {
            egui::Align2::LEFT_CENTER
        } else {
            egui::Align2::RIGHT_CENTER
        };
        let color = if *is_objective {
            ACCENT_BLUE()
        } else {
            TEXT_SECONDARY()
        };
        painter.text(lp, align, name, egui::FontId::proportional(11.0), color);
    }

    true
}

/// Draws the radar chart. Shows only a note when there are fewer than 3 axes, since
/// that can't form a radar shape.
pub fn show(ui: &mut egui::Ui, data: &RadarData) {
    let axes = &data.axes;
    let axis_labels: Vec<(String, bool)> = axes
        .iter()
        .map(|a| (a.name.clone(), a.is_objective))
        .collect();

    if axis_labels.len() < 3 {
        ui.label(
            egui::RichText::new("Radar chart needs at least 3 axes (objectives + variables).")
                .weak(),
        );
        return;
    }

    let scales: Vec<(f64, f64)> = axes
        .iter()
        .map(|a| axis_scale(a.front_min, a.front_max))
        .collect();
    let to_fractions = |values: &[Option<f64>]| -> Vec<Option<f32>> {
        values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.map(|v| {
                    let (lo, hi) = scales[i];
                    value_fraction(v, lo, hi)
                })
            })
            .collect()
    };

    let mut series: Vec<RadarSeries> = Vec::with_capacity(data.front.len() + 1);
    for individual in &data.front {
        series.push(RadarSeries {
            color: FRONT_LINE,
            fractions: to_fractions(individual),
            width: 1.0,
            emphasized: false,
        });
    }
    let sel_values: Vec<Option<f64>> = axes.iter().map(|a| a.selected).collect();
    series.push(RadarSeries {
        color: selected(),
        fractions: to_fractions(&sel_values),
        width: 2.0,
        emphasized: true,
    });

    draw_radar(ui, &axis_labels, &series);

    // ── Legend ─────────────────────────────────────────────────
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        swatch(ui, ACCENT_BLUE());
        ui.label(
            egui::RichText::new(format!("Pareto front individuals ({})", data.front.len())).small(),
        );
        ui.add_space(12.0);
        swatch(ui, selected());
        ui.label(egui::RichText::new("This trial").small());
    });
    ui.label(
        egui::RichText::new(
            "Outer ring = Pareto front max (envelope). Objective axes in blue, variables in gray.",
        )
        .small()
        .weak(),
    );
}

/// Draws a sequence of points in axis order (missing = None) as a closed polyline.
/// Connects only adjacent valid vertices with line segments, skipping any gap.
fn draw_ring_polyline(painter: &egui::Painter, pts: &[Option<egui::Pos2>], stroke: egui::Stroke) {
    let n = pts.len();
    for i in 0..n {
        let j = (i + 1) % n;
        if let (Some(a), Some(b)) = (pts[i], pts[j]) {
            painter.line_segment([a, b], stroke);
        }
    }
}

/// Draws a small color swatch for the legend. Made `pub(crate)` so other widgets
/// (e.g. Radar Comparison) can reuse it to keep legend rows consistent.
pub(crate) fn swatch(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 2.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_scale_outer_maps_to_front_max() {
        let (lo, hi) = axis_scale(2.0, 10.0);
        // The outer ring (hi) matches the front maximum.
        assert!((hi - 10.0).abs() < 1e-9);
        // The minimum has a downward margin (20% below the span=8 range).
        assert!((lo - (2.0 - 1.6)).abs() < 1e-9);
        // The front maximum maps to fraction 1.0; the minimum to a positive fraction away from center.
        assert!((value_fraction(10.0, lo, hi) - 1.0).abs() < 1e-6);
        let f_min = value_fraction(2.0, lo, hi);
        assert!(f_min > 0.0 && f_min < 1.0);
    }

    #[test]
    fn axis_scale_handles_degenerate_front() {
        // When the front is a single point, the value lands at mid-radius.
        let (lo, hi) = axis_scale(5.0, 5.0);
        assert!((value_fraction(5.0, lo, hi) - 0.5).abs() < 1e-6);
        assert!(lo < 5.0 && hi > 5.0);
    }

    #[test]
    fn value_fraction_zero_span_is_center() {
        assert!((value_fraction(3.0, 1.0, 1.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn build_objectives_first_then_params_and_collects_front() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        // 3 trials. trial0, 1 are on the front (rank 0), trial2 is rank 1.
        let core_rows: Vec<CoreRow> = (0..3)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::from([("x".to_string(), i as f64)]),
                param_category_label: HashMap::new(),
                objective_values: vec![i as f64 * 2.0, 10.0 - i as f64],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let obj_names = vec!["o0".to_string(), "o1".to_string()];
        let param_names = vec!["x".to_string()];
        let df = DataFrame::from_trials(&core_rows, &param_names, &obj_names, &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0, 0, 1]);

        let data = build(&view, &obj_names, &param_names, 0);
        // 2 objectives + 1 variable = 3 axes, ordered objectives -> variables.
        assert_eq!(data.axes.len(), 3);
        assert_eq!(data.axes[0].name, "o0");
        assert!(data.axes[0].is_objective);
        assert_eq!(data.axes[2].name, "x");
        assert!(!data.axes[2].is_objective);

        // Take min/max over the front (trial0, 1) only: o0 = {0,2} -> [0,2].
        assert!((data.axes[0].front_min - 0.0).abs() < 1e-9);
        assert!((data.axes[0].front_max - 2.0).abs() < 1e-9);
        // Selected = row 0's o0 = 0.0.
        assert_eq!(data.axes[0].selected, Some(0.0));

        // Front individuals: 2 rows, 3 axes each. trial1 has o0=2, o1=9, x=1.
        assert_eq!(data.front.len(), 2);
        assert_eq!(data.front[0].len(), 3);
        assert_eq!(data.front[1], vec![Some(2.0), Some(9.0), Some(1.0)]);
    }

    #[test]
    fn build_skips_axis_without_front_values() {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let core_rows: Vec<CoreRow> = (0..2)
            .map(|i| CoreRow {
                trial_id: i,
                trial_number: i,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: vec![i as f64],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let obj_names = vec!["o0".to_string()];
        let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0, 0]);
        // Request a nonexistent column name to verify it gets skipped.
        let missing = vec!["nope".to_string()];
        let data = build(&view, &missing, &[], 0);
        assert!(data.axes.is_empty());
        // With no axes, each front row is also empty.
        assert!(data.front.iter().all(|r| r.is_empty()));
    }
}
