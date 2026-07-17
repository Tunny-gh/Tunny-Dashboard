//! Shared infrastructure for 3D scatter plot rendering.
//! Used by the Pareto, Clustering, and MCDM 3D widgets.

use crate::state::types::StudyView;
use crate::theme::chart_colors::{
    COLOR_3D_BG, COLOR_3D_GRID, COLOR_AXIS_X, COLOR_AXIS_Y, COLOR_AXIS_Z,
};
use crate::ui::widgets::common::range_math;
use crate::ui::widgets::trial_detail_modal::{
    nearest_within, show_hover_tooltip, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};

// ── Quaternion math ───────────────────────────────────────────────

/// Quaternion product (Hamilton product).
pub fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

/// Converts axis + angle to a unit quaternion.
pub fn axis_angle_to_quat(axis: [f32; 3], angle: f32) -> [f32; 4] {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    if len < f32::EPSILON {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let half = angle * 0.5;
    let s = half.sin() / len;
    let c = half.cos();
    [axis[0] * s, axis[1] * s, axis[2] * s, c]
}

/// Rotates a point by a quaternion (optimized Rodrigues form).
pub fn rotate_by_quaternion(p: [f32; 3], q: [f32; 4]) -> [f32; 3] {
    let [qx, qy, qz, qw] = q;
    let [px, py, pz] = p;
    let tx = 2.0 * (qy * pz - qz * py);
    let ty = 2.0 * (qz * px - qx * pz);
    let tz = 2.0 * (qx * py - qy * px);
    [
        px + qw * tx + qy * tz - qz * ty,
        py + qw * ty + qz * tx - qx * tz,
        pz + qw * tz + qx * ty - qy * tx,
    ]
}

// ── ArcballCamera ─────────────────────────────────────────────────

/// Arcball camera state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ArcballCamera {
    /// Quaternion [x, y, z, w].
    pub rotation: [f32; 4],
    pub zoom: f32,
    pub pan: [f32; 2],
}

impl Default for ArcballCamera {
    fn default() -> Self {
        Self {
            rotation: [0.0, 0.0, 0.0, 1.0],
            zoom: 3.0,
            pan: [0.0, 0.0],
        }
    }
}

impl ArcballCamera {
    /// Returns a camera at the default isometric view (equivalent to a 45° Y-axis
    /// rotation plus a -30° X-axis rotation).
    /// Used as the default camera pose by the Pareto/Cluster/MCDM/PDP 3D widgets.
    pub fn isometric_default() -> Self {
        Self {
            rotation: [-0.2391, 0.3696, 0.0990, 0.8924],
            ..Default::default()
        }
    }

    pub fn apply_zoom(&mut self, delta: f32) {
        self.zoom = (self.zoom - delta).clamp(0.5, 10.0);
    }

    /// Accumulates a drag amount (in pixels) as screen pan (translation).
    pub fn pan_by_drag(&mut self, dx: f32, dy: f32) {
        self.pan[0] += dx;
        self.pan[1] += dy;
    }

    /// Converts a drag amount (in pixels) to an arcball rotation and accumulates it.
    pub fn rotate_by_drag(&mut self, dx: f32, dy: f32) {
        const SENSITIVITY: f32 = 0.005;
        let q_y = axis_angle_to_quat([0.0, 1.0, 0.0], dx * SENSITIVITY);
        let q_x = axis_angle_to_quat([1.0, 0.0, 0.0], dy * SENSITIVITY);
        let q = quat_mul(q_y, quat_mul(q_x, self.rotation));
        let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        if len > f32::EPSILON {
            self.rotation = [q[0] / len, q[1] / len, q[2] / len, q[3] / len];
        }
    }
}

// ── Data range ────────────────────────────────────────────────────

/// Computes [min, max] from a column slice (takes a StudyView column directly, MEM-002).
pub fn compute_range_from_col(col: Option<&[f64]>) -> (f64, f64) {
    match col.and_then(|c| range_math::value_range(c.iter().copied())) {
        Some((mn, mx)) if mn.is_finite() && mx.is_finite() => range_math::expand_degenerate(mn, mx),
        _ => (-1.0, 1.0),
    }
}

/// Computes [min, max] from finite values only and expands a degenerate range
/// (NaN/Inf are ignored). Returns (-1.0, 1.0) when empty or all non-finite.
/// Shared with the MCDM 3D scatter plot's axis range computation (D-12).
pub fn val_range(vals: &[f64]) -> (f64, f64) {
    let finite = vals.iter().copied().filter(|v| v.is_finite());
    match range_math::value_range(finite) {
        Some((mn, mx)) => range_math::expand_degenerate(mn, mx),
        None => (-1.0, 1.0),
    }
}

/// Cache for the x/y/z axis data ranges.
/// Skips recomputation via `compute_range_from_col` as long as `key` (typically a
/// tuple of three axis indices plus the row count) is unchanged from last time.
#[derive(Debug, Clone)]
pub struct Range3DCache<K> {
    key: Option<K>,
    ranges: [(f64, f64); 3],
}

impl<K> Default for Range3DCache<K> {
    fn default() -> Self {
        Self {
            key: None,
            ranges: [(-1.0, 1.0); 3],
        }
    }
}

impl<K: PartialEq> Range3DCache<K> {
    /// Recomputes the x/y/z ranges via `compute` only when `key` differs from the
    /// previous cache key.
    pub fn get_or_compute(
        &mut self,
        key: K,
        compute: impl FnOnce() -> [(f64, f64); 3],
    ) -> [(f64, f64); 3] {
        if self.key.as_ref() != Some(&key) {
            self.ranges = compute();
            self.key = Some(key);
        }
        self.ranges
    }
}

/// 3D normalized coordinates: converts the data range [v_min, v_max] to [-1, 1].
pub fn normalize_to_clip(v: f64, v_min: f64, v_max: f64) -> f32 {
    if (v_max - v_min).abs() < f64::EPSILON {
        return 0.0;
    }
    (2.0 * (v - v_min) / (v_max - v_min) - 1.0).clamp(-1.0, 1.0) as f32
}

// ── Depth-sorted rendering ───────────────────────────────────────

/// A drawable point with depth. A temporary buffer element for the 3D scatter
/// plot's painter's algorithm (draw back-to-front). Shared by 4 places: `pareto_3d`,
/// `cluster_scatter_3d`, `mcdm_scatter_chart_3d`, and `surrogate_opt` (D-1).
#[derive(Clone, Copy)]
pub struct DepthPoint {
    /// Screen coordinates.
    pub pos: egui::Pos2,
    /// Camera depth (smaller = farther back).
    pub depth: f32,
    pub color: egui::Color32,
    pub radius: f32,
}

/// Sorts back-to-front by depth and draws with `circle_filled` (painter's algorithm).
/// When `stroke` is `Some`, also overlays a circular stroke on each point (used to
/// emphasize predicted front points).
pub fn draw_depth_sorted_points(
    painter: &egui::Painter,
    points: &mut [DepthPoint],
    stroke: Option<egui::Stroke>,
) {
    points.sort_by(|a, b| {
        a.depth
            .partial_cmp(&b.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for p in points.iter() {
        painter.circle_filled(p.pos, p.radius, p.color);
        if let Some(s) = stroke {
            painter.circle_stroke(p.pos, p.radius, s);
        }
    }
}

/// Normalizes a point `[x, y, z]` in value space to clip space [-1, 1] using each
/// axis's range, then projects it and returns `(screen coordinates, depth)`
/// (the normalize_to_clip x3 -> project boilerplate, D-1).
pub fn project_value_3d(
    project: &impl Fn([f32; 3]) -> (egui::Pos2, f32),
    v: [f64; 3],
    ranges: [(f64, f64); 3],
) -> (egui::Pos2, f32) {
    project([
        normalize_to_clip(v[0], ranges[0].0, ranges[0].1),
        normalize_to_clip(v[1], ranges[1].0, ranges[1].1),
        normalize_to_clip(v[2], ranges[2].0, ranges[2].1),
    ])
}

// ── UI helpers ────────────────────────────────────────────────────

/// Index-based objective selection combo box.
pub fn show_objective_combo(
    ui: &mut egui::Ui,
    label: &str,
    id: &str,
    selected: &mut usize,
    obj_names: &[String],
) {
    ui.label(label);
    egui::ComboBox::from_id_salt(id)
        .selected_text(obj_names.get(*selected).map(|s| s.as_str()).unwrap_or("?"))
        .show_ui(ui, |ui| {
            for (i, name) in obj_names.iter().enumerate() {
                ui.selectable_value(selected, i, name);
            }
        });
}

// ── Canvas setup ──────────────────────────────────────────────────

/// Handles camera interaction and returns a render-ready painter, rect, project
/// closure, left-click position, and hover position.
/// - Right-drag -> rotate
/// - Middle-drag / Shift+right-drag -> pan (translate)
/// - Scroll -> zoom
/// - Left-click -> returns the click position in `click_pos` (for point-click hit testing)
/// - Hover -> returns the pointer position in `hover_pos` (`None` while dragging; used
///   for point-hover tooltip hit testing)
/// - Background already filled
///
/// `project` is a pure function (Copy-capture only) returning screen coordinates and
/// depth (Pos2, depth).
#[allow(clippy::type_complexity)]
pub fn setup_3d_canvas(
    ui: &mut egui::Ui,
    camera: &mut ArcballCamera,
) -> (
    egui::Painter,
    egui::Rect,
    impl Fn([f32; 3]) -> (egui::Pos2, f32),
    Option<egui::Pos2>,
    Option<egui::Pos2>,
) {
    let available = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());
    let shift = ui.input(|i| i.modifiers.shift);
    if response.dragged_by(egui::PointerButton::Middle)
        || (shift && response.dragged_by(egui::PointerButton::Secondary))
    {
        // Pan on middle-drag, or right-drag while holding Shift.
        let d = response.drag_delta();
        camera.pan_by_drag(d.x, d.y);
    } else if response.dragged_by(egui::PointerButton::Secondary) {
        // Rotate on right-drag.
        let d = response.drag_delta();
        camera.rotate_by_drag(d.x, d.y);
    }
    // Apply scroll-zoom only while the mouse is over this widget. smooth_scroll_delta
    // is global input, so without gating on hover, all 3D widgets on the canvas
    // would zoom simultaneously. Consume the applied scroll amount so it doesn't
    // propagate to other widgets/canvases.
    let scroll = if response.hovered() {
        ui.input_mut(|i| {
            let s = i.smooth_scroll_delta.y;
            i.smooth_scroll_delta.y = 0.0;
            s
        })
    } else {
        0.0
    };
    if scroll.abs() > f32::EPSILON {
        camera.apply_zoom(scroll * 0.01);
    }
    // Left-click position (used to show the detail modal on point click).
    let click_pos = if response.clicked_by(egui::PointerButton::Primary) {
        response.interact_pointer_pos()
    } else {
        None
    };
    // Hover position (for point-hover tooltips). Suppressed while dragging (rotate/pan).
    let hover_pos = if response.dragged() {
        None
    } else {
        response.hover_pos()
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, COLOR_3D_BG());
    let center = rect.center();
    let scale = rect.size().min_elem() * 0.5 * (camera.zoom / 3.0) * 0.7;
    let cam_rot = camera.rotation;
    let pan = camera.pan;
    let project = move |p: [f32; 3]| -> (egui::Pos2, f32) {
        let r = rotate_by_quaternion(p, cam_rot);
        (
            egui::pos2(
                center.x + r[0] * scale + pan[0],
                center.y - r[1] * scale + pan[1],
            ),
            r[2],
        )
    };
    (painter, rect, project, click_pos, hover_pos)
}

/// Returns the 3D point nearest to the given coordinates as `(trial_id, row_index)`
/// (shared by click and hover). `candidates` is each rendered point's
/// `(trial_id, row_index, screen coordinates)`. Returns `None` if no point is
/// within `HIT_THRESHOLD` px.
pub fn pick_nearest_3d(
    candidates: &[(u32, usize, egui::Pos2)],
    pos: egui::Pos2,
) -> Option<(u32, usize)> {
    let pts: Vec<egui::Pos2> = candidates.iter().map(|c| c.2).collect();
    nearest_within(&pts, pos, HIT_THRESHOLD).map(|i| (candidates[i].0, candidates[i].1))
}

/// Shared "hover for tooltip, click for detail modal" flow for 3D scatter plots.
/// - Suppresses the hover tooltip while the modal is open.
/// - `hover_rows`/`click_context` are closures that build the displayed rows from the
///   hit row index (kept separate since the content shown on hover vs. click differs
///   by widget).
#[allow(clippy::too_many_arguments)]
pub fn show_hover_and_click_detail(
    ui: &mut egui::Ui,
    view: &StudyView,
    candidates: &[(u32, usize, egui::Pos2)],
    hover_pos: Option<egui::Pos2>,
    click_pos: Option<egui::Pos2>,
    tooltip_id: &str,
    detail_modal: &mut TrialDetailModal,
    hover_rows: impl Fn(usize) -> Vec<(String, String)>,
    click_context: impl Fn(usize) -> Vec<(String, String)>,
) {
    if !detail_modal.is_open() {
        if let Some(hover) = hover_pos {
            if let Some((_, row)) = pick_nearest_3d(candidates, hover) {
                let trial_number = view.df.get_trial_number(row).unwrap_or(row as u32);
                let rows = hover_rows(row);
                show_hover_tooltip(ui, tooltip_id, trial_number, &rows);
            }
        }
    }

    if let Some(click) = click_pos {
        if let Some((trial_id, row)) = pick_nearest_3d(candidates, click) {
            let context = click_context(row);
            detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }
    }
}

// ── Grid / axis rendering ─────────────────────────────────────────

/// Draws the 3D grid (the XY/XZ/YZ planes).
pub fn draw_3d_grid(painter: &egui::Painter, project: &impl Fn([f32; 3]) -> (egui::Pos2, f32)) {
    let stroke = egui::Stroke::new(0.5, COLOR_3D_GRID());
    const N: i32 = 4;
    for i in 0..=N {
        let t = -1.0 + 2.0 * i as f32 / N as f32;
        let (p0, _) = project([t, -1.0, -1.0]);
        let (p1, _) = project([t, 1.0, -1.0]);
        painter.line_segment([p0, p1], stroke);
        let (p2, _) = project([-1.0, t, -1.0]);
        let (p3, _) = project([1.0, t, -1.0]);
        painter.line_segment([p2, p3], stroke);
        let (p4, _) = project([t, -1.0, -1.0]);
        let (p5, _) = project([t, -1.0, 1.0]);
        painter.line_segment([p4, p5], stroke);
        let (p6, _) = project([-1.0, -1.0, t]);
        let (p7, _) = project([1.0, -1.0, t]);
        painter.line_segment([p6, p7], stroke);
        let (p8, _) = project([-1.0, t, -1.0]);
        let (p9, _) = project([-1.0, t, 1.0]);
        painter.line_segment([p8, p9], stroke);
        let (p10, _) = project([-1.0, -1.0, t]);
        let (p11, _) = project([-1.0, 1.0, t]);
        painter.line_segment([p10, p11], stroke);
    }
}

/// Draws the axis lines (-1 -> +1) plus name and value labels.
///
/// `names` is `[x_name, y_name, z_name]`, `ranges` is `[(x_min, x_max), ...]`.
pub fn draw_3d_axes(
    painter: &egui::Painter,
    project: &impl Fn([f32; 3]) -> (egui::Pos2, f32),
    names: [&str; 3],
    ranges: [(f64, f64); 3],
) {
    let neg_eps: [[f32; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
    let pos_eps: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let colors = [COLOR_AXIS_X(), COLOR_AXIS_Y(), COLOR_AXIS_Z()];
    for i in 0..3 {
        let (neg_pos, _) = project(neg_eps[i]);
        let (pos_pos, _) = project(pos_eps[i]);
        painter.line_segment([neg_pos, pos_pos], egui::Stroke::new(1.5, colors[i]));
    }
    draw_3d_axis_labels(painter, project, names, ranges);
}

/// Draws only the axis name/value labels (does not draw the axis lines).
/// Used to bring labels to the front when axis lines are drawn mixed into the
/// depth-sorted pass.
pub fn draw_3d_axis_labels(
    painter: &egui::Painter,
    project: &impl Fn([f32; 3]) -> (egui::Pos2, f32),
    names: [&str; 3],
    ranges: [(f64, f64); 3],
) {
    let neg_eps: [[f32; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
    let pos_eps: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let colors = [COLOR_AXIS_X(), COLOR_AXIS_Y(), COLOR_AXIS_Z()];
    for i in 0..3 {
        let (neg_pos, _) = project(neg_eps[i]);
        let (pos_pos, _) = project(pos_eps[i]);
        let color = colors[i];
        let (val_min, val_max) = ranges[i];
        painter.text(
            pos_pos + egui::vec2(4.0, -4.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{} ({:.3})", names[i], val_max),
            egui::FontId::proportional(11.0),
            color,
        );
        painter.text(
            neg_pos + egui::vec2(-4.0, 4.0),
            egui::Align2::RIGHT_TOP,
            format!("{:.3}", val_min),
            egui::FontId::proportional(10.0),
            color.gamma_multiply(0.7),
        );
    }
}

/// Subdivides the axis lines (-1 -> +1) and returns them as clip-space segments
/// (start, end, color). Mixing these into depth-sorted rendering (e.g. with a
/// surface) correctly represents front/back ordering relative to the surface
/// (`draw_3d_axes` draws a single line with no depth ordering).
pub fn axis_segments_3d(subdivisions: usize) -> Vec<([f32; 3], [f32; 3], egui::Color32)> {
    let colors = [COLOR_AXIS_X(), COLOR_AXIS_Y(), COLOR_AXIS_Z()];
    let n = subdivisions.max(1);
    let mut segments = Vec::with_capacity(3 * n);
    for (axis, color) in colors.into_iter().enumerate() {
        for k in 0..n {
            let t0 = -1.0 + 2.0 * k as f32 / n as f32;
            let t1 = -1.0 + 2.0 * (k + 1) as f32 / n as f32;
            let mut a = [0.0_f32; 3];
            let mut b = [0.0_f32; 3];
            a[axis] = t0;
            b[axis] = t1;
            segments.push((a, b, color));
        }
    }
    segments
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcball_camera_default_is_identity() {
        let cam = ArcballCamera::default();
        assert_eq!(cam.rotation, [0.0, 0.0, 0.0, 1.0]);
        assert!((cam.zoom - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_zoom_clamps_to_min() {
        let mut cam = ArcballCamera {
            zoom: 0.6,
            ..Default::default()
        };
        cam.apply_zoom(1.0);
        assert!((cam.zoom - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_zoom_clamps_to_max() {
        let mut cam = ArcballCamera {
            zoom: 9.5,
            ..Default::default()
        };
        cam.apply_zoom(-1.0);
        assert!((cam.zoom - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn normalize_to_clip_min_maps_to_minus_one() {
        let v = normalize_to_clip(0.0, 0.0, 10.0);
        assert!((v - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn normalize_to_clip_max_maps_to_plus_one() {
        let v = normalize_to_clip(10.0, 0.0, 10.0);
        assert!((v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_to_clip_equal_range_returns_zero() {
        let v = normalize_to_clip(5.0, 5.0, 5.0);
        assert!((v - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn axis_angle_to_quat_zero_axis_returns_identity() {
        let q = axis_angle_to_quat([0.0, 0.0, 0.0], 1.0);
        assert!((q[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn quat_mul_identity_preserves_quat() {
        let q = [0.1_f32, 0.2, 0.3, 0.927];
        let id = [0.0, 0.0, 0.0, 1.0];
        let r = quat_mul(id, q);
        for i in 0..4 {
            assert!((r[i] - q[i]).abs() < 1e-5, "i={i}: {} != {}", r[i], q[i]);
        }
    }

    #[test]
    fn rotate_by_quaternion_identity_is_noop() {
        let p = [1.0, 2.0, 3.0];
        let id = [0.0, 0.0, 0.0, 1.0];
        let r = rotate_by_quaternion(p, id);
        for i in 0..3 {
            assert!((r[i] - p[i]).abs() < 1e-5, "i={i}: {} != {}", r[i], p[i]);
        }
    }

    #[test]
    fn rotate_90_deg_around_z_maps_x_to_y() {
        use std::f32::consts::FRAC_PI_2;
        let q = axis_angle_to_quat([0.0, 0.0, 1.0], FRAC_PI_2);
        let r = rotate_by_quaternion([1.0, 0.0, 0.0], q);
        assert!((r[0] - 0.0).abs() < 1e-5, "x={}", r[0]);
        assert!((r[1] - 1.0).abs() < 1e-5, "y={}", r[1]);
        assert!((r[2] - 0.0).abs() < 1e-5, "z={}", r[2]);
    }

    #[test]
    fn axis_segments_3d_returns_three_axes_subdivided() {
        let segs = axis_segments_3d(8);
        assert_eq!(segs.len(), 3 * 8);
        // Each axis's first segment starts at -1 and the last segment ends at +1.
        for axis in 0..3 {
            let first = &segs[axis * 8];
            let last = &segs[axis * 8 + 7];
            assert!((first.0[axis] - (-1.0)).abs() < 1e-6);
            assert!((last.1[axis] - 1.0).abs() < 1e-6);
            // The other components are 0 (the axis passes through the origin).
            for c in 0..3 {
                if c != axis {
                    assert_eq!(first.0[c], 0.0);
                    assert_eq!(last.1[c], 0.0);
                }
            }
        }
    }

    #[test]
    fn axis_segments_3d_clamps_zero_subdivisions_to_one() {
        let segs = axis_segments_3d(0);
        assert_eq!(segs.len(), 3);
    }

    #[test]
    fn pan_by_drag_accumulates_offset() {
        let mut cam = ArcballCamera::default();
        cam.pan_by_drag(10.0, -5.0);
        cam.pan_by_drag(2.0, 3.0);
        assert!((cam.pan[0] - 12.0).abs() < f32::EPSILON);
        assert!((cam.pan[1] - (-2.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn pick_nearest_3d_returns_id_within_threshold() {
        let candidates = vec![
            (10u32, 0usize, egui::pos2(0.0, 0.0)),
            (20u32, 1usize, egui::pos2(50.0, 50.0)),
        ];
        assert_eq!(
            pick_nearest_3d(&candidates, egui::pos2(2.0, 2.0)),
            Some((10, 0))
        );
        assert_eq!(pick_nearest_3d(&candidates, egui::pos2(200.0, 200.0)), None);
    }

    #[test]
    fn rotate_by_drag_changes_rotation_from_identity() {
        let mut cam = ArcballCamera::default();
        cam.rotate_by_drag(100.0, 0.0);
        assert_ne!(cam.rotation, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn rotate_by_drag_preserves_unit_quaternion_length() {
        let mut cam = ArcballCamera::default();
        for _ in 0..100 {
            cam.rotate_by_drag(13.7, -7.3);
        }
        let [x, y, z, w] = cam.rotation;
        let len = (x * x + y * y + z * z + w * w).sqrt();
        assert!(
            (len - 1.0).abs() < 1e-4,
            "quaternion length drifted to {len}"
        );
    }
}
