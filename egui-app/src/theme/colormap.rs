/// Colormap interpolation utility
#[derive(Clone)]
pub struct ColorMap {
    /// List of (t, color) stops. t is in the range [0.0, 1.0].
    pub stops: Vec<(f32, egui::Color32)>,
}

impl ColorMap {
    /// Viridis colormap (5-stop approximation)
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

    /// Plasma colormap (5-stop approximation) - used for visualizing
    /// uncertainty (standard deviation)
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

    /// Blue-to-Yellow colormap (for Pareto rank)
    pub fn blue_yellow() -> Self {
        Self {
            stops: vec![
                (0.0, egui::Color32::from_rgb(255, 220, 0)),
                (1.0, egui::Color32::from_rgb(0, 80, 200)),
            ],
        }
    }

    /// Jet colormap (7-stop approximation)
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

    /// Turbo colormap (7-stop approximation)
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

    /// Inferno colormap (5-stop approximation)
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

    /// Coolwarm colormap (5-stop approximation, diverging)
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

    /// Spectral colormap (7-stop approximation)
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

    /// Cividis colormap (5-stop approximation, colorblind-friendly)
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

    /// Returns the color assigned to the `idx`-th of `count` categories via
    /// even-spaced sampling (D-11). Returns the midpoint (t=0.5) as the
    /// degenerate case when `count <= 1`. Otherwise, evenly distributes
    /// over `[0, 1]` via `idx / (count - 1)`.
    pub fn sample_categorical(&self, idx: usize, count: usize) -> egui::Color32 {
        if count <= 1 {
            self.interpolate(0.5)
        } else {
            self.interpolate(idx as f32 / (count - 1) as f32)
        }
    }

    /// Clamps t to [0.0, 1.0] and linearly interpolates between stops.
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
    fn sample_categorical_single_is_midpoint() {
        let cmap = ColorMap::viridis();
        assert_eq!(cmap.sample_categorical(0, 1), cmap.interpolate(0.5));
    }

    #[test]
    fn sample_categorical_spans_full_range() {
        let cmap = ColorMap::viridis();
        assert_eq!(cmap.sample_categorical(0, 3), cmap.interpolate(0.0));
        assert_eq!(cmap.sample_categorical(1, 3), cmap.interpolate(0.5));
        assert_eq!(cmap.sample_categorical(2, 3), cmap.interpolate(1.0));
    }

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
