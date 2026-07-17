/// Converts an RGBA byte array (non-premultiplied alpha, order [R, G, B, A]) to
/// egui's Color32.
/// The state layer doesn't depend on egui, so it holds colors as `[u8; 4]`;
/// use this function to convert to Color32 at UI drawing time.
pub fn rgba_to_color32(rgba: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(rgba[0], rgba[1], rgba[2], rgba[3])
}

/// Converts `Color32` to an array of its stored bytes as-is (premultiplied alpha, order
/// [R, G, B, A]) (D-11). Used as the key of `HashMap`/`BTreeMap` for grouping points by color.
/// The inverse conversion is [`key_to_color32`] (lossless identity). Note that [`rgba_to_color32`]
/// is for non-premultiplied input and is not the inverse conversion (components won't match for
/// semi-transparent colors).
pub fn rgba_key(color: egui::Color32) -> [u8; 4] {
    color.to_array()
}

/// Inverse conversion of [`rgba_key`]. Since it restores the byte sequence in premultiplied
/// space as-is, `key_to_color32(rgba_key(c)) == c` holds exactly for every color.
pub fn key_to_color32(key: [u8; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_premultiplied(key[0], key[1], key[2], key[3])
}

/// Palette of representative colors assigned to comparison Studies (a different hue family
/// from the baseline Study's green tones).
/// Each element is `[R, G, B, A]` with non-premultiplied alpha.
const COMPARISON_PALETTE: [[u8; 4]; 6] = [
    [66, 133, 244, 255], // blue
    [234, 67, 53, 255],  // red
    [251, 188, 4, 255],  // yellow
    [171, 71, 188, 255], // purple
    [255, 112, 67, 255], // orange
    [0, 172, 193, 255],  // cyan
];

/// Returns the color assigned to the `idx`-th comparison Study (cycles through the palette).
pub fn comparison_color_at(idx: usize) -> [u8; 4] {
    COMPARISON_PALETTE[idx % COMPARISON_PALETTE.len()]
}

/// Computes the alpha value based on whether trial_id is included in selected_indices.
/// If selected_indices is empty, all points return opaque (255).
pub fn compute_point_alpha(trial_id: u32, selected_indices: &[u32]) -> u8 {
    if selected_indices.is_empty() || selected_indices.contains(&trial_id) {
        255
    } else {
        50
    }
}

/// `HashSet` version of `compute_point_alpha` (M-16).
/// Used when the caller builds the selection set into a `HashSet<u32>` once at the start of the
/// frame, replacing the per-point `contains()` linear scan (O(n*s)) with an O(n) set lookup.
/// If `selected` is empty, all points return opaque (255) (same behavior as `compute_point_alpha`).
pub fn point_alpha_in_set(trial_id: u32, selected: &std::collections::HashSet<u32>) -> u8 {
    if selected.is_empty() || selected.contains(&trial_id) {
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

/// Converts a value in [-1, +1] into a 3-point gradient of low(-1) -> white(0) -> high(+1).
fn signed_to_diverging_color(v: f64, low: egui::Color32, high: egui::Color32) -> egui::Color32 {
    let t = ((v + 1.0) / 2.0).clamp(0.0, 1.0) as f32;
    if t < 0.5 {
        lerp_color(low, egui::Color32::WHITE, t * 2.0)
    } else {
        lerp_color(egui::Color32::WHITE, high, (t - 0.5) * 2.0)
    }
}

/// Diverging colormap: score=-1.0 -> blue, 0.0 -> white, +1.0 -> red
pub fn diverging_colormap(score: f64) -> egui::Color32 {
    signed_to_diverging_color(score, egui::Color32::BLUE, egui::Color32::RED)
}

/// Converts a correlation coefficient to Color32 (red=negative correlation, white=no correlation, blue=positive correlation)
pub fn correlation_color(corr: f64) -> egui::Color32 {
    signed_to_diverging_color(corr, egui::Color32::RED, egui::Color32::BLUE)
}

/// Sequential colormap: converts a non-negative normalized value t in [0,1] into a
/// white(0) -> red(1) gradient. Used to display unsigned quantities like importance
/// (tree-based, Sobol, etc.).
pub fn sequential_colormap(t: f64) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0) as f32;
    lerp_color(egui::Color32::WHITE, egui::Color32::RED, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_key_roundtrips_through_key_to_color32() {
        // Even for semi-transparent colors, the round trip through premultiplied space is exactly identity.
        let color = egui::Color32::from_rgba_unmultiplied(12, 34, 56, 78);
        assert_eq!(key_to_color32(rgba_key(color)), color);
        // For opaque colors, the component bytes become the key as-is.
        let opaque = egui::Color32::from_rgb(12, 34, 56);
        assert_eq!(rgba_key(opaque), [12, 34, 56, 255]);
        assert_eq!(key_to_color32(rgba_key(opaque)), opaque);
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

    #[test]
    fn point_alpha_in_set_matches_slice_version() {
        use std::collections::HashSet;
        let empty: HashSet<u32> = HashSet::new();
        assert_eq!(point_alpha_in_set(0, &empty), 255);
        let sel: HashSet<u32> = [1u32, 5, 10].into_iter().collect();
        assert_eq!(point_alpha_in_set(5, &sel), 255);
        assert_eq!(point_alpha_in_set(3, &sel), 50);
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
