/// カラーマップ補間ユーティリティ
pub struct ColorMap {
    /// (t, color) の停止点リスト。t は [0.0, 1.0] の範囲。
    pub stops: Vec<(f32, egui::Color32)>,
}

impl ColorMap {
    /// Viridis カラーマップ（5停止点近似）
    pub fn viridis() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(68, 1, 84)),
                (0.25, egui::Color32::from_rgb(58, 82, 139)),
                (0.5, egui::Color32::from_rgb(32, 144, 140)),
                (0.75, egui::Color32::from_rgb(94, 201, 98)),
                (1.0, egui::Color32::from_rgb(253, 231, 37)),
            ],
        }
    }

    /// Blue-to-Yellow カラーマップ（Pareto ランク用）
    pub fn blue_yellow() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(255, 220, 0)),
                (1.0, egui::Color32::from_rgb(0, 80, 200)),
            ],
        }
    }

    /// t を [0.0, 1.0] にクランプして停止点間を線形補間する
    pub fn interpolate(&self, t: f32) -> egui::Color32 {
        if self.stops.is_empty() {
            return egui::Color32::WHITE;
        }
        let t = t.clamp(0.0, 1.0);

        // t 以下の最後の停止点と t 以上の最初の停止点を見つける
        let n = self.stops.len();
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops[n - 1].0 {
            return self.stops[n - 1].1;
        }

        for i in 0..n - 1 {
            let (t0, c0) = self.stops[i];
            let (t1, c1) = self.stops[i + 1];
            if t0 <= t && t <= t1 {
                let frac = if (t1 - t0).abs() < f32::EPSILON {
                    0.0
                } else {
                    (t - t0) / (t1 - t0)
                };
                let lerp = |a: u8, b: u8| -> u8 {
                    (a as f32 + frac * (b as f32 - a as f32)).round() as u8
                };
                return egui::Color32::from_rgb(
                    lerp(c0.r(), c1.r()),
                    lerp(c0.g(), c1.g()),
                    lerp(c0.b(), c1.b()),
                );
            }
        }
        self.stops[n - 1].1
    }
}

/// trial_id が selected_indices に含まれるかでアルファ値を計算する。
/// selected_indices が空の場合は全点が不透明（255）を返す。
pub fn compute_point_alpha(trial_id: u32, selected_indices: &[u32]) -> u8 {
    if selected_indices.is_empty() || selected_indices.contains(&trial_id) {
        255
    } else {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolate_at_zero_returns_first_stop() {
        let cmap = ColorMap::viridis();
        let color = cmap.interpolate(0.0);
        assert_eq!(color, egui::Color32::from_rgb(68, 1, 84));
    }

    #[test]
    fn interpolate_at_one_returns_last_stop() {
        let cmap = ColorMap::viridis();
        let color = cmap.interpolate(1.0);
        assert_eq!(color, egui::Color32::from_rgb(253, 231, 37));
    }

    #[test]
    fn interpolate_at_half_returns_midpoint() {
        let cmap = ColorMap::blue_yellow();
        let color = cmap.interpolate(0.5);
        // lerp: r = 255 + 0.5*(0-255) = 127, g = 220 + 0.5*(80-220) = 150, b = 0 + 0.5*(200-0) = 100
        assert_eq!(color.r(), 128);
        assert_eq!(color.g(), 150);
        assert_eq!(color.b(), 100);
    }

    #[test]
    fn interpolate_clamped_negative() {
        let cmap = ColorMap::viridis();
        let color_neg = cmap.interpolate(-0.1);
        let color_zero = cmap.interpolate(0.0);
        assert_eq!(color_neg, color_zero);
    }

    #[test]
    fn interpolate_clamped_above_one() {
        let cmap = ColorMap::viridis();
        let color_over = cmap.interpolate(1.1);
        let color_one = cmap.interpolate(1.0);
        assert_eq!(color_over, color_one);
    }

    #[test]
    fn compute_point_alpha_empty_selected_returns_opaque() {
        assert_eq!(compute_point_alpha(0, &[]), 255);
        assert_eq!(compute_point_alpha(99, &[]), 255);
    }

    #[test]
    fn compute_point_alpha_selected_returns_opaque() {
        assert_eq!(compute_point_alpha(5, &[1, 5, 10]), 255);
    }

    #[test]
    fn compute_point_alpha_not_selected_returns_transparent() {
        assert_eq!(compute_point_alpha(3, &[1, 5, 10]), 50);
    }
}
