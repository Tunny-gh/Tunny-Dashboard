//! Observed Contour ウィジェット。
//!
//! 観測トライアル点だけから補間した等高線を 2D で描く（サロゲート非依存）。
//! PDP / サロゲート応答曲面と異なりモデルを学習せず、データの無い領域はマスクして
//! **外挿を見せない**。X / Y / 値（色）はパラメータ・目的関数のどちらでも選べるため、
//! 目的関数空間のトレードオフ面も honest に描ける。計算は `tunny_core::contour` が
//! バックグラウンドで行う（poll_chart.rs 参照）。

use crate::state::types::StudyView;
use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{ObservedContourComputeRequest, ObservedContourState};
use crate::ui::widgets::common::heatmap::{
    draw_colorbar_simple, draw_heatmap_masked, value_range_masked,
};

/// スライス格子の一辺の点数。
const N_GRID: usize = 60;

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut ObservedContourState,
    param_names: &[String],
    obj_names: &[String],
    cmap: ColorMap,
    _view: &StudyView,
    has_constraints: bool,
) {
    // 選択可能な列（数値パラメータ ∪ 目的関数）。
    let columns: Vec<String> = param_names.iter().chain(obj_names).cloned().collect();
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
        // 既定は最初の目的関数、無ければ最初の列。
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
        // ドラッグ中は再計算を抑止し、離した瞬間に 1 回だけ計算する。
        slider_dragging = resp.dragged();

        ui.separator();
        ui.checkbox(&mut state.show_points, "Show points");
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

    // ── 自動再計算（選択が変わったら発行）─────────────────────────
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

    let Some(result) = &state.result else {
        ui.label("Select columns to see the observed contour.");
        return;
    };

    render_2d(ui, result, cmap, state.show_points);

    ui.label("Interpolated from observed trials; blank = no data (not extrapolated).");
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

fn render_2d(
    ui: &mut egui::Ui,
    result: &crate::state::messages::ObservedContourResult,
    cmap: ColorMap,
    show_points: bool,
) {
    let surf = &result.surface;
    let nx = surf.x_values.len();
    let ny = surf.y_values.len();
    if nx < 2 || ny < 2 {
        ui.label("Not enough data to interpolate a contour.");
        return;
    }

    // 表示向き: 横 = X（左→右で増加）、縦 = Y（上 = 最大）。
    // core の z[i][j] = f(x_i, y_j) を disp[r][c] = z[c][ny-1-r] に並べ替える。
    let display: Vec<Vec<Option<f64>>> = (0..ny)
        .map(|r| (0..nx).map(|c| surf.z[c][ny - 1 - r]).collect())
        .collect();

    let (v_min, v_max) = value_range_masked(&display);

    let available = ui.available_rect_before_wrap();
    let plot_size = egui::vec2(
        (available.width() - 40.0).max(120.0),
        available.height().clamp(120.0, 360.0),
    );
    let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // 背景（マスク領域が分かるよう薄い枠）。
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
    );
    draw_heatmap_masked(&painter, rect, &display, v_min, v_max, cmap.clone());

    // 観測点の重畳。
    if show_points {
        let (x_min, x_max) = (surf.x_values[0], surf.x_values[nx - 1]);
        let (y_min, y_max) = (surf.y_values[0], surf.y_values[ny - 1]);
        if x_max > x_min && y_max > y_min {
            for p in &result.points {
                let fx = ((p[0] - x_min) / (x_max - x_min)).clamp(0.0, 1.0) as f32;
                let fy = ((p[1] - y_min) / (y_max - y_min)).clamp(0.0, 1.0) as f32;
                let pos = egui::pos2(
                    rect.left() + fx * rect.width(),
                    rect.bottom() - fy * rect.height(),
                );
                let t = if (v_max - v_min).abs() < f64::EPSILON {
                    0.5
                } else {
                    ((p[2] - v_min) / (v_max - v_min)).clamp(0.0, 1.0) as f32
                };
                painter.circle_filled(pos, 2.5, cmap.interpolate(t));
                painter.circle_stroke(pos, 2.5, egui::Stroke::new(0.6, egui::Color32::BLACK));
            }
        }
    }

    // カラーバー。
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap);

    ui.label(format!(
        "X: {}   Y: {}   Value: {}",
        result.x_name, result.y_name, result.value_name
    ));
}
