use crate::state::app_state::AppState;
use crate::theme::chart_colors::{
    COLOR_3D_BG, COLOR_3D_GRID, COLOR_AXIS_X, COLOR_AXIS_Y, COLOR_AXIS_Z, COLOR_HIGHLIGHT_PT,
    COLOR_NON_PARETO, COLOR_PARETO,
};
use crate::theme::color_compute::compute_point_alpha;
use crate::theme::TOOLBAR_BTN_FG;

/// クォータニオン積（Hamilton product）
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

/// 軸・角度 → 単位クォータニオン
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

/// クォータニオンで点を回転（Rodrigues 最適化形式）
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

/// Arcball カメラ状態
#[derive(Debug, Clone)]
pub struct ArcballCamera {
    /// クォータニオン [x, y, z, w]
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
    pub fn apply_zoom(&mut self, delta: f32) {
        self.zoom = (self.zoom - delta).clamp(0.5, 10.0);
    }

    pub fn is_identity_rotation(&self) -> bool {
        let [x, y, z, w] = self.rotation;
        x.abs() < f32::EPSILON
            && y.abs() < f32::EPSILON
            && z.abs() < f32::EPSILON
            && (w - 1.0).abs() < 1e-6
    }

    /// ドラッグ量（ピクセル）をアークボール回転に変換して累積する
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

/// Pareto 3D チャートウィジェット
pub struct Pareto3dChart {
    pub x_objective: usize,
    pub y_objective: usize,
    pub z_objective: usize,
    pub camera: ArcballCamera,
    range_cache: [(f64, f64); 3],
    range_cache_key: (usize, usize, usize, usize), // (x_obj, y_obj, z_obj, trial_count)
}

impl Default for Pareto3dChart {
    fn default() -> Self {
        // Y軸45° + X軸-30° のアイソメトリック初期視点
        // quat_mul(q_y(45°), q_x(-30°)) ≈ [-0.239, 0.370, 0.099, 0.892]
        let camera = ArcballCamera {
            rotation: [-0.2391, 0.3696, 0.0990, 0.8924],
            ..Default::default()
        };
        Self {
            x_objective: 0,
            y_objective: 1,
            z_objective: 2,
            camera,
            range_cache: [(-1.0, 1.0); 3],
            range_cache_key: (usize::MAX, usize::MAX, usize::MAX, 0),
        }
    }
}

fn show_objective_combo(
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

/// 列スライスから [min, max] を計算する（StudyView の列を直接受け取る・MEM-002）。
fn compute_range_from_col(col: Option<&[f64]>) -> (f64, f64) {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    if let Some(c) = col {
        for &v in c {
            if v < mn {
                mn = v;
            }
            if v > mx {
                mx = v;
            }
        }
    }
    if !mn.is_finite() || !mx.is_finite() {
        (-1.0, 1.0)
    } else if (mx - mn).abs() < f64::EPSILON {
        (mn - 1.0, mx + 1.0)
    } else {
        (mn, mx)
    }
}

impl Pareto3dChart {
    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        let Some(ctx) = &app_state.current_study else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a study");
            });
            return;
        };

        let obj_names = &ctx.meta.objective_names;
        if obj_names.len() < 3 {
            ui.centered_and_justified(|ui| {
                ui.label("Need at least 3 objectives for 3D view");
            });
            return;
        }

        let downsample_indices = app_state.downsample_cache.scatter.clone();
        let ctx = app_state.current_study.as_ref().unwrap();
        let view = &ctx.view;
        let trial_count = view.row_count();

        let range_cache_key = (
            self.x_objective,
            self.y_objective,
            self.z_objective,
            trial_count,
        );
        if self.range_cache_key != range_cache_key {
            let col = |idx: usize| obj_names.get(idx).and_then(|n| view.numeric_column(n));
            self.range_cache = [
                compute_range_from_col(col(self.x_objective)),
                compute_range_from_col(col(self.y_objective)),
                compute_range_from_col(col(self.z_objective)),
            ];
            self.range_cache_key = range_cache_key;
        }
        let [(x_min, x_max), (y_min, y_max), (z_min, z_max)] = self.range_cache;

        ui.horizontal(|ui| {
            show_objective_combo(ui, "X:", "pareto3d_x", &mut self.x_objective, obj_names);
            show_objective_combo(ui, "Y:", "pareto3d_y", &mut self.y_objective, obj_names);
            show_objective_combo(ui, "Z:", "pareto3d_z", &mut self.z_objective, obj_names);
        });

        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click_and_drag());

        // 左ドラッグ → 回転
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.camera.rotate_by_drag(delta.x, delta.y);
        }

        // スクロール → ズーム
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta.abs() > f32::EPSILON {
            self.camera.apply_zoom(scroll_delta * 0.01);
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, COLOR_3D_BG);

        let center = rect.center();
        let half = rect.size().min_elem() * 0.5;
        let scale = half * (self.camera.zoom / 3.0) * 0.7;
        let cam_rot = self.camera.rotation;

        let project = |p_clip: [f32; 3]| -> (egui::Pos2, f32) {
            let r = rotate_by_quaternion(p_clip, cam_rot);
            let sx = center.x + r[0] * scale;
            let sy = center.y - r[1] * scale;
            (egui::pos2(sx, sy), r[2])
        };

        // グリッドプレーン（3面：XY@z=-1, XZ@y=-1, YZ@x=-1）
        let grid_stroke = egui::Stroke::new(0.5, COLOR_3D_GRID);
        const GRID_DIVS: i32 = 4;
        for i in 0..=GRID_DIVS {
            let t = -1.0 + 2.0 * i as f32 / GRID_DIVS as f32;
            // XY 平面 (z = -1)
            let (p0, _) = project([t, -1.0, -1.0]);
            let (p1, _) = project([t, 1.0, -1.0]);
            painter.line_segment([p0, p1], grid_stroke);
            let (p2, _) = project([-1.0, t, -1.0]);
            let (p3, _) = project([1.0, t, -1.0]);
            painter.line_segment([p2, p3], grid_stroke);
            // XZ 平面 (y = -1)
            let (p4, _) = project([t, -1.0, -1.0]);
            let (p5, _) = project([t, -1.0, 1.0]);
            painter.line_segment([p4, p5], grid_stroke);
            let (p6, _) = project([-1.0, -1.0, t]);
            let (p7, _) = project([1.0, -1.0, t]);
            painter.line_segment([p6, p7], grid_stroke);
            // YZ 平面 (x = -1)
            let (p8, _) = project([-1.0, t, -1.0]);
            let (p9, _) = project([-1.0, t, 1.0]);
            painter.line_segment([p8, p9], grid_stroke);
            let (p10, _) = project([-1.0, -1.0, t]);
            let (p11, _) = project([-1.0, 1.0, t]);
            painter.line_segment([p10, p11], grid_stroke);
        }

        // 目的関数名と値範囲付き軸線（-1 → +1 全長）
        let x_name = obj_names.get(self.x_objective).cloned().unwrap_or_default();
        let y_name = obj_names.get(self.y_objective).cloned().unwrap_or_default();
        let z_name = obj_names.get(self.z_objective).cloned().unwrap_or_default();
        let axes = [
            (
                [-1.0f32, 0.0, 0.0],
                [1.0f32, 0.0, 0.0],
                COLOR_AXIS_X,
                &x_name,
                x_min,
                x_max,
            ),
            (
                [0.0, -1.0f32, 0.0],
                [0.0, 1.0f32, 0.0],
                COLOR_AXIS_Y,
                &y_name,
                y_min,
                y_max,
            ),
            (
                [0.0, 0.0, -1.0f32],
                [0.0, 0.0, 1.0f32],
                COLOR_AXIS_Z,
                &z_name,
                z_min,
                z_max,
            ),
        ];
        for (neg_ep, pos_ep, color, name, val_min, val_max) in &axes {
            let (neg_pos, _) = project(*neg_ep);
            let (pos_pos, _) = project(*pos_ep);
            painter.line_segment([neg_pos, pos_pos], egui::Stroke::new(1.5, *color));
            painter.text(
                pos_pos + egui::vec2(4.0, -4.0),
                egui::Align2::LEFT_BOTTOM,
                format!("{} ({:.3})", name, val_max),
                egui::FontId::proportional(11.0),
                *color,
            );
            painter.text(
                neg_pos + egui::vec2(-4.0, 4.0),
                egui::Align2::RIGHT_TOP,
                format!("{:.3}", val_min),
                egui::FontId::proportional(10.0),
                color.gamma_multiply(0.7),
            );
        }

        // 点の収集（view の列スライスから直接・行クローンキャッシュを持たない・MEM-002）
        let selected = &app_state.selected_indices;
        let highlighted = app_state.highlighted_trial;
        let x_col = obj_names
            .get(self.x_objective)
            .and_then(|n| view.numeric_column(n));
        let y_col = obj_names
            .get(self.y_objective)
            .and_then(|n| view.numeric_column(n));
        let z_col = obj_names
            .get(self.z_objective)
            .and_then(|n| view.numeric_column(n));

        let displayed: Vec<usize> = match downsample_indices.as_deref() {
            Some(idx) => idx
                .iter()
                .map(|&i| i as usize)
                .filter(|&i| i < trial_count)
                .collect(),
            None => (0..trial_count).collect(),
        };
        let mut draw_calls: Vec<(egui::Pos2, f32, egui::Color32, f32)> =
            Vec::with_capacity(displayed.len());
        let mut highlight_call: Option<egui::Pos2> = None;

        for i in displayed {
            let xv = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let yv = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let zv = z_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let clip = [
                normalize_to_clip(xv, x_min, x_max),
                normalize_to_clip(yv, y_min, y_max),
                normalize_to_clip(zv, z_min, z_max),
            ];
            let (screen_pos, depth) = project(clip);
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);

            if highlighted == Some(trial_id) {
                highlight_call = Some(screen_pos);
                continue;
            }

            let alpha = compute_point_alpha(trial_id, selected);
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            let (base_color, radius) = if rank == 0 {
                (COLOR_PARETO, 5.0_f32)
            } else {
                (COLOR_NON_PARETO, 3.0_f32)
            };
            let color = if alpha == 255 {
                base_color
            } else {
                egui::Color32::from_rgba_unmultiplied(
                    base_color.r(),
                    base_color.g(),
                    base_color.b(),
                    60,
                )
            };
            draw_calls.push((screen_pos, depth, color, radius));
        }

        // 奥から手前の順（ペインターズアルゴリズム）
        draw_calls.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (pos, _, color, radius) in &draw_calls {
            painter.circle_filled(*pos, *radius, *color);
        }

        if let Some(pos) = highlight_call {
            painter.circle_filled(pos, 8.0, COLOR_HIGHLIGHT_PT);
            painter.circle_stroke(pos, 9.5, egui::Stroke::new(1.5, TOOLBAR_BTN_FG));
        }
    }
}

/// ズーム値を有効範囲にクランプする
pub fn clamp_zoom(zoom: f32, min: f32, max: f32) -> f32 {
    zoom.clamp(min, max)
}

/// 3D 正規化座標: データ範囲 [v_min, v_max] を [-1, 1] に変換する
pub fn normalize_to_clip(v: f64, v_min: f64, v_max: f64) -> f32 {
    if (v_max - v_min).abs() < f64::EPSILON {
        return 0.0;
    }
    (2.0 * (v - v_min) / (v_max - v_min) - 1.0).clamp(-1.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcball_camera_default_is_identity() {
        let cam = ArcballCamera::default();
        assert!(cam.is_identity_rotation());
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
    fn clamp_zoom_within_range() {
        assert!((clamp_zoom(3.0, 0.5, 10.0) - 3.0).abs() < f32::EPSILON);
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
    fn pareto_3d_chart_default_objectives() {
        let chart = Pareto3dChart::default();
        assert_eq!(chart.x_objective, 0);
        assert_eq!(chart.y_objective, 1);
        assert_eq!(chart.z_objective, 2);
        // 初期視点はアイソメトリック角度（identity ではない）
        assert!(!chart.camera.is_identity_rotation());
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
    fn rotate_by_drag_changes_rotation_from_identity() {
        let mut cam = ArcballCamera::default();
        cam.rotate_by_drag(100.0, 0.0);
        assert!(!cam.is_identity_rotation());
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
