/// カラーマップ補間ユーティリティ
#[derive(Clone)]
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

    /// Plasma カラーマップ（5停止点近似）— 不確実性（標準偏差）の可視化に使用
    pub fn plasma() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(13, 8, 135)),
                (0.25, egui::Color32::from_rgb(126, 3, 168)),
                (0.5, egui::Color32::from_rgb(204, 71, 120)),
                (0.75, egui::Color32::from_rgb(248, 149, 64)),
                (1.0, egui::Color32::from_rgb(240, 249, 33)),
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

    /// Jet カラーマップ（7停止点近似）
    pub fn jet() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(0, 0, 143)),
                (0.17, egui::Color32::from_rgb(0, 0, 255)),
                (0.33, egui::Color32::from_rgb(0, 200, 255)),
                (0.5, egui::Color32::from_rgb(100, 255, 0)),
                (0.67, egui::Color32::from_rgb(255, 255, 0)),
                (0.83, egui::Color32::from_rgb(255, 100, 0)),
                (1.0, egui::Color32::from_rgb(128, 0, 0)),
            ],
        }
    }

    /// Turbo カラーマップ（7停止点近似）
    pub fn turbo() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(48, 18, 59)),
                (0.17, egui::Color32::from_rgb(70, 108, 228)),
                (0.33, egui::Color32::from_rgb(30, 195, 149)),
                (0.5, egui::Color32::from_rgb(163, 222, 30)),
                (0.67, egui::Color32::from_rgb(249, 160, 27)),
                (0.83, egui::Color32::from_rgb(220, 50, 32)),
                (1.0, egui::Color32::from_rgb(122, 4, 3)),
            ],
        }
    }

    /// Inferno カラーマップ（5停止点近似）
    pub fn inferno() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(0, 0, 4)),
                (0.25, egui::Color32::from_rgb(87, 16, 110)),
                (0.5, egui::Color32::from_rgb(188, 55, 84)),
                (0.75, egui::Color32::from_rgb(249, 142, 9)),
                (1.0, egui::Color32::from_rgb(252, 255, 164)),
            ],
        }
    }

    /// Coolwarm カラーマップ（5停止点近似、発散型）
    pub fn coolwarm() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(59, 76, 192)),
                (0.25, egui::Color32::from_rgb(141, 176, 254)),
                (0.5, egui::Color32::from_rgb(237, 237, 237)),
                (0.75, egui::Color32::from_rgb(252, 146, 114)),
                (1.0, egui::Color32::from_rgb(180, 4, 38)),
            ],
        }
    }

    /// Spectral カラーマップ（7停止点近似）
    pub fn spectral() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(158, 1, 66)),
                (0.17, egui::Color32::from_rgb(213, 62, 79)),
                (0.33, egui::Color32::from_rgb(244, 109, 67)),
                (0.5, egui::Color32::from_rgb(253, 200, 128)),
                (0.67, egui::Color32::from_rgb(171, 222, 164)),
                (0.83, egui::Color32::from_rgb(53, 151, 143)),
                (1.0, egui::Color32::from_rgb(94, 79, 162)),
            ],
        }
    }

    /// Cividis カラーマップ（5停止点近似、色覚多様性対応）
    pub fn cividis() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(0, 32, 76)),
                (0.25, egui::Color32::from_rgb(57, 89, 129)),
                (0.5, egui::Color32::from_rgb(126, 160, 150)),
                (0.75, egui::Color32::from_rgb(204, 213, 122)),
                (1.0, egui::Color32::from_rgb(253, 252, 47)),
            ],
        }
    }

    /// t を [0.0, 1.0] にクランプして停止点間を線形補間する
    pub fn interpolate(&self, t: f32) -> egui::Color32 {
        if self.stops.is_empty() {
            return egui::Color32::WHITE;
        }
        let t = t.clamp(0.0, 1.0);

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
}
