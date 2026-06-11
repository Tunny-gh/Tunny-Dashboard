/// RGBA バイト配列（非プリマルチプライドアルファ、順序 [R, G, B, A]）を
/// egui の Color32 へ変換する。
/// state 層は egui 依存を持たないため `[u8; 4]` で色を保持しており、
/// UI 描画時にこの関数を使って Color32 へ変換する。
pub fn rgba_to_color32(rgba: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// 比較 Study に割り当てる代表色のパレット（基準 Study の緑系とは別の色相）。
/// 各要素は `[R, G, B, A]` の非プリマルチプライドアルファ。
const COMPARISON_PALETTE: [[u8; 4]; 6] = [
    [66, 133, 244, 255], // 青
    [234, 67, 53, 255],  // 赤
    [251, 188, 4, 255],  // 黄
    [171, 71, 188, 255], // 紫
    [255, 112, 67, 255], // オレンジ
    [0, 172, 193, 255],  // シアン
];

/// `idx` 番目の比較 Study に割り当てる色を返す（パレットを循環）。
pub fn comparison_color_at(idx: usize) -> [u8; 4] {
    COMPARISON_PALETTE[idx % COMPARISON_PALETTE.len()]
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

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + t * (b as f32 - a as f32)) as u8
}

fn lerp_color(c0: egui::Color32, c1: egui::Color32, t: f32) -> egui::Color32 {
    egui::Color32::from_rgb(
        lerp_u8(c0.r(), c1.r(), t),
        lerp_u8(c0.g(), c1.g(), t),
        lerp_u8(c0.b(), c1.b(), t),
    )
}

/// [-1, +1] の値を low(-1) → white(0) → high(+1) の3点グラデーションに変換する。
fn signed_to_diverging_color(v: f64, low: egui::Color32, high: egui::Color32) -> egui::Color32 {
    let t = ((v + 1.0) / 2.0).clamp(0.0, 1.0) as f32;
    if t < 0.5 {
        lerp_color(low, egui::Color32::WHITE, t * 2.0)
    } else {
        lerp_color(egui::Color32::WHITE, high, (t - 0.5) * 2.0)
    }
}

/// 発散型カラーマップ: score=-1.0 → 青, 0.0 → 白, +1.0 → 赤
pub fn diverging_colormap(score: f64) -> egui::Color32 {
    signed_to_diverging_color(score, egui::Color32::BLUE, egui::Color32::RED)
}

/// 相関係数を Color32 に変換する（赤=負相関, 白=無相関, 青=正相関）
pub fn correlation_color(corr: f64) -> egui::Color32 {
    signed_to_diverging_color(corr, egui::Color32::RED, egui::Color32::BLUE)
}

/// 逐次型カラーマップ: 非負の正規化値 t∈[0,1] を白(0) → 赤(1) のグラデーションに変換する。
/// 重要度のように符号を持たない量（木ベース・Sobol など）の表示に用いる。
pub fn sequential_colormap(t: f64) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0) as f32;
    lerp_color(egui::Color32::WHITE, egui::Color32::RED, t)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let _ = diverging_colormap(score);
        }
    }

    #[test]
    fn correlation_color_negative_is_reddish() {
        let color = correlation_color(-1.0);
        assert!(color.r() > color.b());
    }

    #[test]
    fn correlation_color_positive_is_bluish() {
        let color = correlation_color(1.0);
        assert!(color.b() > color.r());
    }
}
