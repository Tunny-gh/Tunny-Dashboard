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
            rotation: [0.0, 0.0, 0.0, 1.0], // identity quaternion
            zoom: 3.0,
            pan: [0.0, 0.0],
        }
    }
}

impl ArcballCamera {
    /// ズーム値を clamp する
    pub fn apply_zoom(&mut self, delta: f32) {
        self.zoom = (self.zoom - delta).clamp(0.5, 10.0);
    }

    /// アイデンティティ回転かどうか
    pub fn is_identity_rotation(&self) -> bool {
        let [x, y, z, w] = self.rotation;
        x.abs() < f32::EPSILON && y.abs() < f32::EPSILON && z.abs() < f32::EPSILON
            && (w - 1.0).abs() < 1e-6
    }
}

/// Pareto 3D チャートウィジェット
pub struct Pareto3dChart {
    pub x_objective: usize,
    pub y_objective: usize,
    pub z_objective: usize,
    pub camera: ArcballCamera,
}

impl Default for Pareto3dChart {
    fn default() -> Self {
        Self {
            x_objective: 0,
            y_objective: 1,
            z_objective: 2,
            camera: ArcballCamera::default(),
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
        let mut cam = ArcballCamera::default();
        cam.zoom = 0.6;
        cam.apply_zoom(1.0); // 0.6 - 1.0 = -0.4 → clamp to 0.5
        assert!((cam.zoom - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_zoom_clamps_to_max() {
        let mut cam = ArcballCamera::default();
        cam.zoom = 9.5;
        cam.apply_zoom(-1.0); // 9.5 + 1.0 = 10.5 → clamp to 10.0
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
    }
}
