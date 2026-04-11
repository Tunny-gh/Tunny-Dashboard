use crate::state::app_state::SensitivityResult;

/// 発散型カラーマップ: -1.0 → 青, 0.0 → 白, +1.0 → 赤
pub fn diverging_colormap(score: f64) -> egui::Color32 {
    let t = ((score + 1.0) / 2.0).clamp(0.0, 1.0) as f32;
    if t < 0.5 {
        // -1 → blue, 0 → white
        let f = t * 2.0;
        egui::Color32::from_rgb((255.0 * f) as u8, (255.0 * f) as u8, 255)
    } else {
        // 0 → white, +1 → red
        let f = (t - 0.5) * 2.0;
        egui::Color32::from_rgb(255, (255.0 * (1.0 - f)) as u8, (255.0 * (1.0 - f)) as u8)
    }
}

/// 感度ヒートマップウィジェット
#[derive(Default)]
pub struct SensitivityHeatmap {
    pub computing: bool,
    pub result: Option<SensitivityResult>,
}

impl SensitivityHeatmap {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diverging_colormap_negative_one_is_blue() {
        let color = diverging_colormap(-1.0);
        assert!(color.b() > color.r(), "score=-1 should be blue-dominant");
    }

    #[test]
    fn diverging_colormap_zero_is_white() {
        let color = diverging_colormap(0.0);
        assert_eq!(color.r(), 255);
        assert_eq!(color.g(), 255);
        assert_eq!(color.b(), 255);
    }

    #[test]
    fn diverging_colormap_positive_one_is_red() {
        let color = diverging_colormap(1.0);
        assert!(color.r() > color.b(), "score=+1 should be red-dominant");
    }

    #[test]
    fn diverging_colormap_intermediate_values_bounded() {
        for i in -10..=10 {
            let score = i as f64 / 10.0;
            let _ = diverging_colormap(score); // must not panic
        }
    }

    #[test]
    fn sensitivity_heatmap_default() {
        let hm = SensitivityHeatmap::default();
        assert!(!hm.computing);
        assert!(hm.result.is_none());
    }
}
