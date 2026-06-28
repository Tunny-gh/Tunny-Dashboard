//! 3D 散布図描画の共有インフラ。
//! Pareto・Clustering・MCDM の各 3D ウィジェットが参照する。

use crate::theme::chart_colors::{
    COLOR_3D_BG, COLOR_3D_GRID, COLOR_AXIS_X, COLOR_AXIS_Y, COLOR_AXIS_Z,
};
use crate::ui::widgets::trial_detail_modal::{nearest_within, HIT_THRESHOLD};

// ── クォータニオン計算 ────────────────────────────────────────────

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

// ── ArcballCamera ─────────────────────────────────────────────────

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

    /// ドラッグ量（ピクセル）を画面パン（平行移動）として累積する
    pub fn pan_by_drag(&mut self, dx: f32, dy: f32) {
        self.pan[0] += dx;
        self.pan[1] += dy;
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

// ── データ範囲 ────────────────────────────────────────────────────

/// 列スライスから [min, max] を計算する（StudyView の列を直接受け取る・MEM-002）。
pub fn compute_range_from_col(col: Option<&[f64]>) -> (f64, f64) {
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

/// 3D 正規化座標: データ範囲 [v_min, v_max] を [-1, 1] に変換する
pub fn normalize_to_clip(v: f64, v_min: f64, v_max: f64) -> f32 {
    if (v_max - v_min).abs() < f64::EPSILON {
        return 0.0;
    }
    (2.0 * (v - v_min) / (v_max - v_min) - 1.0).clamp(-1.0, 1.0) as f32
}

/// ズーム値を有効範囲にクランプする
pub fn clamp_zoom(zoom: f32, min: f32, max: f32) -> f32 {
    zoom.clamp(min, max)
}

// ── UI ヘルパー ───────────────────────────────────────────────────

/// インデックスベースの目的関数選択コンボボックス
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

// ── キャンバス初期化 ──────────────────────────────────────────────

/// カメラ操作を処理し、描画準備済みの painter・rect・project クロージャと
/// 左クリック位置・ホバー位置を返す。
/// - 右ドラッグ → 回転
/// - 中ドラッグ / Shift+右ドラッグ → パン（平行移動）
/// - スクロール → ズーム
/// - 左クリック → 戻り値の `click_pos` にクリック位置を返す（点クリック判定用）
/// - ホバー → 戻り値の `hover_pos` にポインタ位置を返す（ドラッグ中は `None`。点ホバー
///   ツールチップ判定用）
/// - 背景塗りつぶし済み
///
/// `project` はスクリーン座標と深度 (Pos2, depth) を返す純粋関数（Copy キャプチャのみ）
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
        // 中ドラッグ、または Shift を押しながらの右ドラッグでパン
        let d = response.drag_delta();
        camera.pan_by_drag(d.x, d.y);
    } else if response.dragged_by(egui::PointerButton::Secondary) {
        // 右ドラッグで回転
        let d = response.drag_delta();
        camera.rotate_by_drag(d.x, d.y);
    }
    // スクロールズームはこのウィジェットにマウスがあるときだけ適用する。
    // smooth_scroll_delta はグローバル入力のため、ホバー判定でゲートしないと
    // キャンバス上の全 3D ウィジェットが同時にズームしてしまう。
    // 適用したスクロール量は消費し、他のウィジェット／キャンバスへ伝播させない。
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
    // 左クリック位置（点クリックでの詳細モーダル表示用）
    let click_pos = if response.clicked_by(egui::PointerButton::Primary) {
        response.interact_pointer_pos()
    } else {
        None
    };
    // ホバー位置（点ホバーツールチップ用）。回転・パン中のドラッグは抑止する。
    let hover_pos = if response.dragged() {
        None
    } else {
        response.hover_pos()
    };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, COLOR_3D_BG);
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

/// 指定座標に最も近い 3D 点を `(trial_id, row_index)` で返す（クリック・ホバー共用）。
/// `candidates` は描画した各点の `(trial_id, row_index, スクリーン座標)`。
/// `HIT_THRESHOLD` px 以内に点がなければ `None`。
pub fn pick_nearest_3d(
    candidates: &[(u32, usize, egui::Pos2)],
    pos: egui::Pos2,
) -> Option<(u32, usize)> {
    let pts: Vec<egui::Pos2> = candidates.iter().map(|c| c.2).collect();
    nearest_within(&pts, pos, HIT_THRESHOLD).map(|i| (candidates[i].0, candidates[i].1))
}

// ── グリッド・軸描画 ──────────────────────────────────────────────

/// 3Dグリッド（XY/XZ/YZ の 3 面）を描画する
pub fn draw_3d_grid(painter: &egui::Painter, project: &impl Fn([f32; 3]) -> (egui::Pos2, f32)) {
    let stroke = egui::Stroke::new(0.5, COLOR_3D_GRID);
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

/// 軸線（-1→+1）と名前・値ラベルを描画する。
///
/// `names` は `[x_name, y_name, z_name]`、`ranges` は `[(x_min, x_max), ...]`。
pub fn draw_3d_axes(
    painter: &egui::Painter,
    project: &impl Fn([f32; 3]) -> (egui::Pos2, f32),
    names: [&str; 3],
    ranges: [(f64, f64); 3],
) {
    let neg_eps: [[f32; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
    let pos_eps: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let colors = [COLOR_AXIS_X, COLOR_AXIS_Y, COLOR_AXIS_Z];
    for i in 0..3 {
        let (neg_pos, _) = project(neg_eps[i]);
        let (pos_pos, _) = project(pos_eps[i]);
        painter.line_segment([neg_pos, pos_pos], egui::Stroke::new(1.5, colors[i]));
    }
    draw_3d_axis_labels(painter, project, names, ranges);
}

/// 軸の名前・値ラベルのみを描画する（軸線は描かない）。
/// 軸線を深度ソートに混ぜて描く場合に、ラベルだけ最前面へ出す用途。
pub fn draw_3d_axis_labels(
    painter: &egui::Painter,
    project: &impl Fn([f32; 3]) -> (egui::Pos2, f32),
    names: [&str; 3],
    ranges: [(f64, f64); 3],
) {
    let neg_eps: [[f32; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]];
    let pos_eps: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let colors = [COLOR_AXIS_X, COLOR_AXIS_Y, COLOR_AXIS_Z];
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

/// 軸線（-1→+1）を細分化し、クリップ空間の線分 (始点, 終点, 色) として返す。
/// サーフェスなどの深度ソート描画に混ぜることで、面との前後関係を正しく表現できる
/// （`draw_3d_axes` は前後関係を持たない一本線として描く）。
pub fn axis_segments_3d(subdivisions: usize) -> Vec<([f32; 3], [f32; 3], egui::Color32)> {
    let colors = [COLOR_AXIS_X, COLOR_AXIS_Y, COLOR_AXIS_Z];
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

// ── テスト ────────────────────────────────────────────────────────

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
        // 各軸の最初の線分は -1 から、最後の線分は +1 で終わる
        for axis in 0..3 {
            let first = &segs[axis * 8];
            let last = &segs[axis * 8 + 7];
            assert!((first.0[axis] - (-1.0)).abs() < 1e-6);
            assert!((last.1[axis] - 1.0).abs() < 1e-6);
            // 他の成分は 0（軸は原点を通る）
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
