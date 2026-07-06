//! 2D ヒートマップ描画の共有ヘルパー。
//!
//! Optimizer（`surrogate_opt`）の応答曲面スライス（真上から見た 2D ヒートマップ）で
//! 使う共有モジュール。

use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::range_math;

/// マスク対応ヒートマップ。`None` のセルは塗らない（パネル背景のまま＝データなし）。
/// `v_min` / `v_max` は呼び出し側で `Some` セルだけから求めた値域を渡す。
pub fn draw_heatmap_masked(
    painter: &egui::Painter,
    rect: egui::Rect,
    values: &[Vec<Option<f64>>],
    v_min: f64,
    v_max: f64,
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
    let cell_w = rect.width() / n_col as f32;
    let cell_h = rect.height() / n_row as f32;

    for (row, row_vals) in values.iter().enumerate() {
        for (col, cell) in row_vals.iter().enumerate() {
            let Some(val) = cell else {
                continue; // データなし → 塗らない。
            };
            let t = normalize(*val, v_min, v_max);
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

/// マスク対応グリッドの値域 [min, max] を `Some` セルだけから求める。空なら (0,1)。
pub fn value_range_masked(values: &[Vec<Option<f64>>]) -> (f64, f64) {
    let flat = values.iter().flatten().flatten().copied();
    match range_math::value_range(flat) {
        Some((mn, mx)) if mn.is_finite() && mx.is_finite() => range_math::expand_degenerate(mn, mx),
        _ => (0.0, 1.0),
    }
}

/// ヒートマップ脇の縦カラーバーを描く。
///
/// バー本体（上 = max / 下 = min）に加え、**数値目盛**（max / 中央 / min）をバー右脇に、
/// `title` が `Some` なら凡例が表す値の名前を縦書きでさらに右に添える。目盛・タイトルは
/// バー右側にはみ出すため `ui.painter()`（bar_rect にクリップしない）で描く。呼び出し側は
/// bar_rect の右に目盛（＋タイトル）ぶんの余白を確保すること（目盛のみ ~50px、タイトル込み ~80px）。
pub fn draw_colorbar_simple(
    ui: &mut egui::Ui,
    bar_rect: egui::Rect,
    v_min: f64,
    v_max: f64,
    cmap: ColorMap,
    title: Option<&str>,
) {
    let painter = ui.painter();
    let n_steps = 48;
    let step_h = bar_rect.height() / n_steps as f32;
    for i in 0..n_steps {
        let t = 1.0 - (i as f32 / (n_steps - 1).max(1) as f32);
        let step_rect = egui::Rect::from_min_size(
            egui::pos2(bar_rect.left(), bar_rect.top() + i as f32 * step_h),
            egui::vec2(bar_rect.width(), step_h + 1.0),
        );
        painter.rect_filled(step_rect, 0.0, cmap.interpolate(t));
    }
    painter.rect_stroke(
        bar_rect,
        0.0,
        egui::Stroke::new(0.5, egui::Color32::from_gray(90)),
        egui::StrokeKind::Inside,
    );

    // 数値目盛（max / 中央 / min）をバー右脇に置く。最大幅を測ってタイトル位置に使う。
    let text_color = crate::theme::CLOSE_BTN_TEXT();
    let tick_font = egui::FontId::proportional(10.0);
    let mid = (v_min + v_max) * 0.5;
    let ticks = [
        (bar_rect.top(), egui::Align2::LEFT_TOP, v_max),
        (bar_rect.center().y, egui::Align2::LEFT_CENTER, mid),
        (bar_rect.bottom(), egui::Align2::LEFT_BOTTOM, v_min),
    ];
    let mut tick_w = 0.0_f32;
    for (y, align, val) in ticks {
        let r = painter.text(
            egui::pos2(bar_rect.right() + 3.0, y),
            align,
            format!("{:.3}", val),
            tick_font.clone(),
            text_color,
        );
        tick_w = tick_w.max(r.width());
    }

    // 凡例が表す値の名前を縦書きで添える（数値目盛の右）。
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        let galley = painter.layout_no_wrap(
            title.to_owned(),
            egui::FontId::proportional(11.0),
            text_color,
        );
        let title_x = bar_rect.right() + 3.0 + tick_w + 6.0;
        let title_pos = egui::pos2(title_x, bar_rect.center().y + galley.size().x * 0.5);
        painter.add(
            egui::epaint::TextShape::new(title_pos, galley, text_color)
                .with_angle(-std::f32::consts::FRAC_PI_2),
        );
    }
}

/// 値を [0.0, 1.0] に正規化する（退化範囲は 0.5）。
pub fn normalize(v: f64, v_min: f64, v_max: f64) -> f32 {
    range_math::normalize01(v, v_min, v_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_clamps_to_unit_range() {
        assert!((normalize(0.5, 0.0, 1.0) - 0.5).abs() < 1e-6);
        assert!((normalize(-1.0, 0.0, 1.0) - 0.0).abs() < 1e-6);
        assert!((normalize(2.0, 0.0, 1.0) - 1.0).abs() < 1e-6);
    }
}
