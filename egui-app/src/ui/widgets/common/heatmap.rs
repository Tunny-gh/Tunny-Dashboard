//! 2D ヒートマップ描画の共有ヘルパー。
//!
//! Optimizer（`surrogate_opt`）の応答曲面スライス（真上から見た 2D ヒートマップ）で
//! 使う。ResponseSurfacePlot は 3D 描画なのでこれらは使わないが、Optimizer 側の
//! スライス描画が依存しているため共有モジュールとして切り出している。

use crate::theme::colormap::ColorMap;

/// 値グリッド（行 = Y、列 = X）をカラーマップで塗ったヒートマップを描く。
pub fn draw_heatmap(
    painter: &egui::Painter,
    rect: egui::Rect,
    values: &[Vec<f64>],
    cmap: ColorMap,
) {
    let n_row = values.len();
    if n_row == 0 {
        return;
    }
    let n_col = values[0].len();
    if n_col == 0 {
        return;
    }
    let (v_min, v_max) = value_range(values);
    let cell_w = rect.width() / n_col as f32;
    let cell_h = rect.height() / n_row as f32;

    for (row, row_vals) in values.iter().enumerate() {
        for (col, &val) in row_vals.iter().enumerate() {
            let t = normalize(val, v_min, v_max);
            let color = cmap.interpolate(t);
            let cell_rect = egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + col as f32 * cell_w,
                    rect.top() + row as f32 * cell_h,
                ),
                egui::vec2(cell_w + 1.0, cell_h + 1.0),
            );
            painter.rect_filled(cell_rect, 0.0, color);
        }
    }
}

/// ヒートマップ脇の縦カラーバー（上 = max / 下 = min）を描く。
pub fn draw_colorbar_simple(
    ui: &mut egui::Ui,
    bar_rect: egui::Rect,
    v_min: f64,
    v_max: f64,
    cmap: ColorMap,
) {
    let painter = ui.painter_at(bar_rect);
    let n_steps = 32;
    let step_h = bar_rect.height() / n_steps as f32;
    for i in 0..n_steps {
        let t = 1.0 - (i as f32 / n_steps as f32);
        let color = cmap.interpolate(t);
        let step_rect = egui::Rect::from_min_size(
            egui::pos2(bar_rect.left(), bar_rect.top() + i as f32 * step_h),
            egui::vec2(bar_rect.width(), step_h + 1.0),
        );
        painter.rect_filled(step_rect, 0.0, color);
    }
    ui.label(format!("{:.2}", v_max));
    ui.label(format!("{:.2}", v_min));
}

/// 値グリッドの [min, max] を返す。空・退化時はフォールバック範囲を返す。
pub fn value_range(values: &[Vec<f64>]) -> (f64, f64) {
    let flat: Vec<f64> = values.iter().flatten().copied().collect();
    if flat.is_empty() {
        return (0.0, 1.0);
    }
    let v_min = flat.iter().cloned().fold(f64::INFINITY, f64::min);
    let v_max = flat.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (v_max - v_min).abs() < f64::EPSILON {
        (v_min - 1.0, v_max + 1.0)
    } else {
        (v_min, v_max)
    }
}

/// 値を [0.0, 1.0] に正規化する（退化範囲は 0.5）。
pub fn normalize(v: f64, v_min: f64, v_max: f64) -> f32 {
    if (v_max - v_min).abs() < f64::EPSILON {
        return 0.5;
    }
    ((v - v_min) / (v_max - v_min)).clamp(0.0, 1.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_range_returns_correct_bounds() {
        let values = vec![vec![1.0, 3.0], vec![2.0, 4.0]];
        let (v_min, v_max) = value_range(&values);
        assert!((v_min - 1.0).abs() < 1e-9);
        assert!((v_max - 4.0).abs() < 1e-9);
    }

    #[test]
    fn normalize_clamps_to_unit_range() {
        assert!((normalize(0.5, 0.0, 1.0) - 0.5).abs() < 1e-6);
        assert!((normalize(-1.0, 0.0, 1.0) - 0.0).abs() < 1e-6);
        assert!((normalize(2.0, 0.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn value_range_empty_is_unit() {
        let empty: Vec<Vec<f64>> = vec![];
        assert_eq!(value_range(&empty), (0.0, 1.0));
    }
}
