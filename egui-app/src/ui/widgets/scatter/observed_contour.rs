//! Observed Contour ウィジェット。
//!
//! 観測トライアル点だけから補間した等高線を描く（サロゲート非依存）。
//! PDP / サロゲート応答曲面と異なりモデルを学習せず、データの無い領域はマスクして
//! **外挿を見せない**。X / Y / 値（色）はパラメータ・目的関数のどちらでも選べるため、
//! 目的関数空間のトレードオフ面も honest に描ける。計算は `tunny_core::contour` が
//! バックグラウンドで行う（poll_chart.rs 参照）。

use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::messages::ObservedContourResult;
use crate::state::types::StudyView;
use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{ObservedContourComputeRequest, ObservedContourState};
use crate::ui::widgets::common::heatmap::{
    draw_colorbar_simple, draw_heatmap_masked, value_range_masked,
};
use crate::ui::widgets::trial_detail_modal::{TrialDetailTarget, HIT_THRESHOLD};

/// 格子の一辺の点数。
const N_GRID: usize = 60;
/// 等高線のレベル数。
const N_CONTOUR_LEVELS: usize = 6;

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut ObservedContourState,
    param_names: &[String],
    obj_names: &[String],
    cmap: ColorMap,
    view: &StudyView,
    artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    has_constraints: bool,
) {
    // 選択可能な列（数値パラメータ ∪ 目的関数）。カテゴリカル列は除外する。
    let columns: Vec<String> = param_names
        .iter()
        .filter(|p| view.numeric_column(p).is_some())
        .chain(obj_names.iter())
        .cloned()
        .collect();
    if columns.len() < 2 {
        ui.label("Need at least 2 numeric columns (parameters or objectives).");
        return;
    }

    // デフォルト選択（Study 切替で消えた名前もリセット）。
    if !columns.contains(&state.selected_x) {
        state.selected_x = columns[0].clone();
    }
    if !columns.contains(&state.selected_y) || state.selected_y == state.selected_x {
        state.selected_y = columns
            .iter()
            .find(|c| **c != state.selected_x)
            .cloned()
            .unwrap_or_default();
    }
    if !columns.contains(&state.selected_value) {
        state.selected_value = obj_names
            .first()
            .cloned()
            .unwrap_or_else(|| columns[0].clone());
    }

    // ── 軸・値セレクタ ───────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("X:");
        combo(ui, "oc_x", &mut state.selected_x, &columns);
        ui.label("Y:");
        combo(ui, "oc_y", &mut state.selected_y, &columns);
        ui.label("Value:");
        combo(ui, "oc_value", &mut state.selected_value, &columns);
    });

    // ── Coverage（疎ガード）・トグル ─────────────────────────────
    let mut slider_dragging = false;
    ui.horizontal(|ui| {
        ui.label("Coverage:");
        let resp = ui
            .add(
                egui::Slider::new(&mut state.max_edge_ratio, 0.03..=1.0)
                    .text("gap")
                    .fixed_decimals(2),
            )
            .on_hover_text(
                "小さいほどデータの薄い領域を厳しくマスク（外挿を見せない）、大きいほど広く塗る",
            );
        slider_dragging = resp.dragged();

        ui.separator();
        ui.checkbox(&mut state.show_points, "Show points");
        ui.checkbox(&mut state.show_contour_lines, "Contours");
        ui.checkbox(&mut state.log_scale, "Log color")
            .on_hover_text("正の値のときのみ有効");
        if has_constraints {
            ui.checkbox(&mut state.feasible_only, "Feasible only");
        }
    });

    // 同一軸の警告（許容するが縮退する）。
    if state.selected_x == state.selected_y {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Warning: same column selected for X and Y",
        );
    }

    // ── 自動再計算（軸・値・Coverage・feasible が変わったら発行）──────
    let cur = (
        state.selected_x.clone(),
        state.selected_y.clone(),
        state.selected_value.clone(),
        state.max_edge_ratio,
        state.feasible_only,
    );
    let changed = state.applied_sig.as_ref() != Some(&cur);
    if changed
        && !slider_dragging
        && !state.computing
        && state.pending_compute.is_none()
        && state.selected_x != state.selected_y
    {
        state.applied_sig = Some(cur.clone());
        state.computing = true;
        state.result = None;
        state.error_message = None;
        state.pending_compute = Some(ObservedContourComputeRequest {
            x: cur.0,
            y: cur.1,
            value: cur.2,
            n_grid: N_GRID,
            max_edge_ratio: cur.3,
            feasible_only: cur.4,
        });
    }

    if state.computing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Interpolating observed contour…");
        });
        return;
    }

    if let Some(ref err) = state.error_message {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
        return;
    }

    // result の不変借用をブロックに閉じ込め、クリック対象だけ取り出す。
    let clicked: Option<TrialDetailTarget> = {
        let Some(result) = state.result.as_ref() else {
            ui.label("Select columns to see the observed contour.");
            return;
        };
        let opts = RenderOpts {
            show_points: state.show_points,
            show_contour_lines: state.show_contour_lines,
            log_scale: state.log_scale,
        };
        let clicked = render_2d(ui, result, &cmap, &opts, view);
        ui.label("Interpolated from observed trials; blank = no data (not extrapolated).");
        clicked
    };

    if let Some(target) = clicked {
        state.detail_modal.open(target);
    }
    state
        .detail_modal
        .show(ui, view, param_names, obj_names, artifact_map);
}

fn combo(ui: &mut egui::Ui, salt: &str, selected: &mut String, columns: &[String]) {
    egui::ComboBox::from_id_salt(salt)
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for name in columns {
                ui.selectable_value(selected, name.clone(), name);
            }
        });
}

struct RenderOpts {
    show_points: bool,
    show_contour_lines: bool,
    log_scale: bool,
}

/// 等高線を描画し、観測点クリックがあればその対象を返す。
fn render_2d(
    ui: &mut egui::Ui,
    result: &ObservedContourResult,
    cmap: &ColorMap,
    opts: &RenderOpts,
    view: &StudyView,
) -> Option<TrialDetailTarget> {
    let surf = &result.surface;
    let nx = surf.x_values.len();
    let ny = surf.y_values.len();
    if nx < 2 || ny < 2 {
        ui.label("Not enough data to interpolate a contour.");
        return None;
    }

    // 表示向き: 横 = X（左→右で増加）、縦 = Y（上 = 最大）。
    // core の z[i][j] = f(x_i, y_j) を disp[r][c] = z[c][ny-1-r] に並べ替える。
    let display: Vec<Vec<Option<f64>>> = (0..ny)
        .map(|r| (0..nx).map(|c| surf.z[c][ny - 1 - r]).collect())
        .collect();

    // 元の値域（カラーバー表示・等高線レベル用）。
    let (v_min, v_max) = value_range_masked(&display);

    // 色用に対数変換するか（正の値域のみ）。
    let use_log = opts.log_scale && v_min > 0.0;
    let color_display: Vec<Vec<Option<f64>>> = if use_log {
        display
            .iter()
            .map(|row| row.iter().map(|c| c.map(|v| v.log10())).collect())
            .collect()
    } else {
        display.clone()
    };
    let (cv_min, cv_max) = value_range_masked(&color_display);

    let available = ui.available_rect_before_wrap();
    let plot_size = egui::vec2(
        (available.width() - 40.0).max(120.0),
        available.height().clamp(120.0, 360.0),
    );
    let (rect, response) = ui.allocate_exact_size(plot_size, egui::Sense::click());
    let painter = ui.painter_at(rect);

    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
    );
    draw_heatmap_masked(&painter, rect, &color_display, cv_min, cv_max, cmap.clone());

    if opts.show_contour_lines {
        draw_contour_lines(&painter, rect, &display, v_min, v_max);
    }

    // 観測点の重畳（クリックヒットテスト用に screen 位置も集める）。
    let (x_min, x_max) = (surf.x_values[0], surf.x_values[nx - 1]);
    let (y_min, y_max) = (surf.y_values[0], surf.y_values[ny - 1]);
    let mut screen_points: Vec<(egui::Pos2, usize)> = Vec::new();
    if x_max > x_min && y_max > y_min {
        for (idx, p) in result.points.iter().enumerate() {
            let fx = ((p[0] - x_min) / (x_max - x_min)).clamp(0.0, 1.0) as f32;
            let fy = ((p[1] - y_min) / (y_max - y_min)).clamp(0.0, 1.0) as f32;
            let pos = egui::pos2(
                rect.left() + fx * rect.width(),
                rect.bottom() - fy * rect.height(),
            );
            screen_points.push((pos, idx));
            if opts.show_points {
                let cv = if use_log {
                    p[2].max(1e-300).log10()
                } else {
                    p[2]
                };
                let t = if (cv_max - cv_min).abs() < f64::EPSILON {
                    0.5
                } else {
                    ((cv - cv_min) / (cv_max - cv_min)).clamp(0.0, 1.0) as f32
                };
                painter.circle_filled(pos, 2.5, cmap.interpolate(t));
                painter.circle_stroke(pos, 2.5, egui::Stroke::new(0.6, egui::Color32::BLACK));
            }
        }
    }

    // カラーバー（ラベルは元の値域。対数時も実値を表示）。
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap.clone());

    ui.label(format!(
        "X: {}   Y: {}   Value: {}{}",
        result.x_name,
        result.y_name,
        result.value_name,
        if use_log { " (log color)" } else { "" }
    ));

    // クリック → 最近傍の観測点を詳細表示。
    if response.clicked() {
        if let Some(click) = response.interact_pointer_pos() {
            if let Some(idx) = nearest_point(&screen_points, click, HIT_THRESHOLD) {
                let trial_id = result.point_trial_ids.get(idx).copied()?;
                let row_index = view.trial_ids.iter().position(|&t| t == trial_id)?;
                let value = result.points.get(idx).map(|p| p[2]).unwrap_or(f64::NAN);
                return Some(TrialDetailTarget {
                    trial_id,
                    row_index,
                    context: vec![(result.value_name.clone(), format!("{:.6}", value))],
                });
            }
        }
    }
    None
}

/// `screen_points`（位置, インデックス）の中で `click` に最も近く閾値内の点のインデックスを返す。
fn nearest_point(
    screen_points: &[(egui::Pos2, usize)],
    click: egui::Pos2,
    threshold: f32,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (pos, idx) in screen_points {
        let d = pos.distance(click);
        if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, *idx));
        }
    }
    best.map(|(_, idx)| idx)
}

/// マスク対応の等高線（marching squares）。4 隅とも `Some` のセルのみ描く。
fn draw_contour_lines(
    painter: &egui::Painter,
    rect: egui::Rect,
    display: &[Vec<Option<f64>>],
    v_min: f64,
    v_max: f64,
) {
    let ny = display.len();
    if ny < 2 {
        return;
    }
    let nx = display[0].len();
    if nx < 2 || (v_max - v_min).abs() < f64::EPSILON {
        return;
    }
    let cw = rect.width() / nx as f32;
    let ch = rect.height() / ny as f32;
    // セル中心をサンプル位置とする。
    let sx = |c: usize| rect.left() + (c as f32 + 0.5) * cw;
    let sy = |r: usize| rect.top() + (r as f32 + 0.5) * ch;
    let stroke = egui::Stroke::new(0.8, egui::Color32::from_white_alpha(150));

    for li in 1..=N_CONTOUR_LEVELS {
        let level = v_min + (v_max - v_min) * li as f64 / (N_CONTOUR_LEVELS + 1) as f64;
        for r in 0..ny - 1 {
            for c in 0..nx - 1 {
                let (Some(tl), Some(tr), Some(br), Some(bl)) = (
                    display[r][c],
                    display[r][c + 1],
                    display[r + 1][c + 1],
                    display[r + 1][c],
                ) else {
                    continue; // 不完全セルは等高線を描かない。
                };
                // 4 辺（上・右・下・左）の交点を集める。
                let mut pts: Vec<egui::Pos2> = Vec::with_capacity(4);
                let (x0, x1) = (sx(c), sx(c + 1));
                let (y0, y1) = (sy(r), sy(r + 1));
                if let Some(t) = edge_cross(tl, tr, level) {
                    pts.push(egui::pos2(lerp(x0, x1, t), y0));
                }
                if let Some(t) = edge_cross(tr, br, level) {
                    pts.push(egui::pos2(x1, lerp(y0, y1, t)));
                }
                if let Some(t) = edge_cross(bl, br, level) {
                    pts.push(egui::pos2(lerp(x0, x1, t), y1));
                }
                if let Some(t) = edge_cross(tl, bl, level) {
                    pts.push(egui::pos2(x0, lerp(y0, y1, t)));
                }
                match pts.len() {
                    2 => {
                        painter.line_segment([pts[0], pts[1]], stroke);
                    }
                    4 => {
                        painter.line_segment([pts[0], pts[1]], stroke);
                        painter.line_segment([pts[2], pts[3]], stroke);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// 辺の 2 端点 `a`,`b` が `level` を挟むなら、a→b 上の交点比率 `t`(0..1) を返す。
fn edge_cross(a: f64, b: f64, level: f64) -> Option<f64> {
    let above_a = a >= level;
    let above_b = b >= level;
    if above_a == above_b {
        return None;
    }
    let denom = b - a;
    if denom.abs() < f64::EPSILON {
        return None;
    }
    Some(((level - a) / denom).clamp(0.0, 1.0))
}

fn lerp(a: f32, b: f32, t: f64) -> f32 {
    a + (b - a) * t as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_cross_detects_straddle() {
        // 0 と 2 が level=1 を挟む → 中点 t=0.5。
        assert_eq!(edge_cross(0.0, 2.0, 1.0), Some(0.5));
        // 同符号は None。
        assert!(edge_cross(0.0, 0.5, 1.0).is_none());
        assert!(edge_cross(2.0, 3.0, 1.0).is_none());
    }

    #[test]
    fn nearest_point_within_threshold() {
        let pts = vec![
            (egui::pos2(0.0, 0.0), 0),
            (egui::pos2(10.0, 0.0), 1),
            (egui::pos2(100.0, 100.0), 2),
        ];
        assert_eq!(nearest_point(&pts, egui::pos2(11.0, 0.0), 12.0), Some(1));
        assert_eq!(nearest_point(&pts, egui::pos2(500.0, 500.0), 12.0), None);
    }
}
