//! 予測パレートフロントの目的空間散布図（2D / 3D）と観測点の重畳描画。

use crate::state::messages::SurrogateMultiOptUiResult;
use crate::ui::widget_states::SurrogateOptState;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};

use super::ObservedData;

/// 予測パレートフロントを目的空間の 2D 散布図として描画する。
/// 目的が 3 つ以上のときは X/Y 軸の目的を選択できる。フロント点は
/// X 軸目的でソートして折れ線で結び、`COLOR_SURROGATE_FRONT`（金色）で示す。
pub(super) fn render_front_scatter(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    state: &mut SurrogateOptState,
    observed: Option<&ObservedData>,
) {
    use crate::ui::widgets::scatter_3d::show_objective_combo;

    let n_obj = result.objective_names.len();
    if n_obj < 2 || result.front.is_empty() {
        return;
    }

    // インデックスのクランプ（目的数が変わった場合など）。
    if state.multi_front_x_obj >= n_obj {
        state.multi_front_x_obj = 0;
    }
    if state.multi_front_y_obj >= n_obj {
        state.multi_front_y_obj = 1.min(n_obj - 1);
    }
    if state.multi_front_z_obj >= n_obj {
        state.multi_front_z_obj = 2.min(n_obj - 1);
    }

    // ── 観測点の表示トグル（ParetoScatter と同様: フロント / 被支配 / 実行不可能） ──
    let any_infeasible = observed
        .map(|o| o.feasible.iter().any(|&f| !f))
        .unwrap_or(false);
    if observed.is_some() {
        ui.horizontal(|ui| {
            ui.label("Observed:");
            ui.checkbox(&mut state.show_observed_front, "Pareto front");
            ui.checkbox(&mut state.show_observed_dominated, "Others");
            if any_infeasible {
                ui.checkbox(&mut state.show_observed_infeasible, "Infeasible");
            }
        });
    }
    let toggles = ObservedToggles {
        front: state.show_observed_front,
        dominated: state.show_observed_dominated,
        infeasible: state.show_observed_infeasible,
    };

    // ── 目的が 2 つ: 固定軸の 2D 散布図のみ ─────────────────────────
    if n_obj == 2 {
        render_front_scatter_2d(ui, result, 0, 1, observed, toggles);
        return;
    }

    // ── 目的が 3 つ以上: 2D / 3D 切替 + 軸セレクタ ──────────────────
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.multi_front_3d, "3D view");
        ui.separator();
        if state.multi_front_3d {
            show_objective_combo(
                ui,
                "X:",
                "surrogate_front_x",
                &mut state.multi_front_x_obj,
                &result.objective_names,
            );
            show_objective_combo(
                ui,
                "Y:",
                "surrogate_front_y",
                &mut state.multi_front_y_obj,
                &result.objective_names,
            );
            show_objective_combo(
                ui,
                "Z:",
                "surrogate_front_z",
                &mut state.multi_front_z_obj,
                &result.objective_names,
            );
        } else {
            show_objective_combo(
                ui,
                "X:",
                "surrogate_front_x",
                &mut state.multi_front_x_obj,
                &result.objective_names,
            );
            show_objective_combo(
                ui,
                "Y:",
                "surrogate_front_y",
                &mut state.multi_front_y_obj,
                &result.objective_names,
            );
        }
    });

    if state.multi_front_3d {
        render_front_scatter_3d(ui, result, state, observed, toggles);
    } else {
        render_front_scatter_2d(
            ui,
            result,
            state.multi_front_x_obj,
            state.multi_front_y_obj,
            observed,
            toggles,
        );
    }
}

/// 観測点の分類別表示トグル（ParetoScatter と同じ 3 分類）。
#[derive(Clone, Copy)]
struct ObservedToggles {
    /// 観測パレートフロント（rank 0・feasible）を表示するか。
    front: bool,
    /// 観測の被支配点（rank>0・feasible）を表示するか。
    dominated: bool,
    /// 観測の実行不可能解を表示するか。
    infeasible: bool,
}

/// 観測点を目的 (xi, yi) で (パレートフロント, 被支配, 実行不可能) の 3 群に分類する。
#[allow(clippy::type_complexity)]
fn classify_observed_2d(
    obs: &ObservedData,
    xi: usize,
    yi: usize,
) -> (Vec<[f64; 2]>, Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let (Some(xc), Some(yc)) = (obs.objective_cols.get(xi), obs.objective_cols.get(yi)) else {
        return (Vec::new(), Vec::new(), Vec::new());
    };
    let n = xc
        .len()
        .min(yc.len())
        .min(obs.pareto_rank.len())
        .min(obs.feasible.len());
    let mut front = Vec::new();
    let mut dominated = Vec::new();
    let mut infeasible = Vec::new();
    for i in 0..n {
        let pt = [xc[i], yc[i]];
        if !obs.feasible[i] {
            infeasible.push(pt);
        } else if obs.pareto_rank[i] == 0 {
            front.push(pt);
        } else {
            dominated.push(pt);
        }
    }
    (front, dominated, infeasible)
}

/// 観測点を目的 (xi, yi, zi) で (パレートフロント, 被支配, 実行不可能) の 3 群に分類する。
#[allow(clippy::type_complexity)]
fn classify_observed_3d(
    obs: &ObservedData,
    xi: usize,
    yi: usize,
    zi: usize,
) -> (Vec<[f64; 3]>, Vec<[f64; 3]>, Vec<[f64; 3]>) {
    let (Some(xc), Some(yc), Some(zc)) = (
        obs.objective_cols.get(xi),
        obs.objective_cols.get(yi),
        obs.objective_cols.get(zi),
    ) else {
        return (Vec::new(), Vec::new(), Vec::new());
    };
    let n = xc
        .len()
        .min(yc.len())
        .min(zc.len())
        .min(obs.pareto_rank.len())
        .min(obs.feasible.len());
    let mut front = Vec::new();
    let mut dominated = Vec::new();
    let mut infeasible = Vec::new();
    for i in 0..n {
        let pt = [xc[i], yc[i], zc[i]];
        if !obs.feasible[i] {
            infeasible.push(pt);
        } else if obs.pareto_rank[i] == 0 {
            front.push(pt);
        } else {
            dominated.push(pt);
        }
    }
    (front, dominated, infeasible)
}

/// 予測パレートフロントを 2D 散布図（目的 xi × yi）で描画する。
/// 点は X 軸でソートして折れ線で結び、`COLOR_SURROGATE_FRONT`（金色ダイヤ）で示す。
fn render_front_scatter_2d(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    xi: usize,
    yi: usize,
    observed: Option<&ObservedData>,
    toggles: ObservedToggles,
) {
    use crate::theme::chart_colors::{
        COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_SURROGATE_FRONT,
    };

    let mut pts: Vec<[f64; 2]> = result
        .front
        .iter()
        .filter_map(|p| Some([*p.values.get(xi)?, *p.values.get(yi)?]))
        .collect();
    if pts.is_empty() {
        return;
    }
    pts.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap_or(std::cmp::Ordering::Equal));

    // 既存（観測）点を ParetoScatter と同じ 3 分類で射影する。
    let (obs_front, obs_dominated, obs_infeasible) = match observed {
        Some(obs) => classify_observed_2d(obs, xi, yi),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };

    let x_label = result.objective_names.get(xi).cloned().unwrap_or_default();
    let y_label = result.objective_names.get(yi).cloned().unwrap_or_default();

    egui_plot::Plot::new("surrogate_front_scatter_2d")
        .unified_nav()
        .height(220.0)
        .x_axis_label(&x_label)
        .y_axis_label(&y_label)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            // 観測点を背面に描く（実行不可能 → 被支配 → 観測フロントの順）。
            if toggles.infeasible && !obs_infeasible.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Infeasible", obs_infeasible)
                        .shape(egui_plot::MarkerShape::Circle)
                        .radius(2.5)
                        .color(COLOR_INFEASIBLE()),
                );
            }
            if toggles.dominated && !obs_dominated.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Observed (others)", obs_dominated)
                        .shape(egui_plot::MarkerShape::Circle)
                        .radius(2.5)
                        .color(COLOR_NON_PARETO()),
                );
            }
            if toggles.front && !obs_front.is_empty() {
                plot_ui.points(
                    egui_plot::Points::new("Observed Pareto front", obs_front)
                        .shape(egui_plot::MarkerShape::Circle)
                        .radius(3.5)
                        .color(COLOR_PARETO()),
                );
            }
            // フロントを結ぶ折れ線（点が 2 つ以上のとき）。
            if pts.len() >= 2 {
                plot_ui.line(
                    egui_plot::Line::new("Predicted Pareto front", pts.clone())
                        .color(COLOR_SURROGATE_FRONT())
                        .width(1.5),
                );
            }
            // 予測フロント点（金色ダイヤ）。
            plot_ui.points(
                egui_plot::Points::new("Predicted Pareto front", pts)
                    .shape(egui_plot::MarkerShape::Diamond)
                    .radius(4.5)
                    .color(COLOR_SURROGATE_FRONT()),
            );
        });
}

/// 予測パレートフロントを 3D 散布図（目的 X × Y × Z）で描画する。
/// `scatter_3d` の共有インフラ（アークボールカメラ・投影・グリッド・軸）を再利用する。
fn render_front_scatter_3d(
    ui: &mut egui::Ui,
    result: &SurrogateMultiOptUiResult,
    state: &mut SurrogateOptState,
    observed: Option<&ObservedData>,
    toggles: ObservedToggles,
) {
    use crate::theme::chart_colors::{
        COLOR_INFEASIBLE, COLOR_NON_PARETO, COLOR_PARETO, COLOR_SURROGATE_FRONT,
    };
    use crate::ui::widgets::scatter_3d::{
        compute_range_from_col, draw_3d_axes, draw_3d_grid, draw_depth_sorted_points,
        project_value_3d, setup_3d_canvas, DepthPoint,
    };

    let xi = state.multi_front_x_obj;
    let yi = state.multi_front_y_obj;
    let zi = state.multi_front_z_obj;

    // フロント点の各軸値。
    let axis_vals = |idx: usize| -> Vec<f64> {
        result
            .front
            .iter()
            .filter_map(|p| p.values.get(idx).copied())
            .collect()
    };
    let x_vals = axis_vals(xi);
    let y_vals = axis_vals(yi);
    let z_vals = axis_vals(zi);
    if x_vals.is_empty() || y_vals.is_empty() || z_vals.is_empty() {
        return;
    }

    // 観測（既存）点の各軸列。改善を見比べるための背景クラウド。
    let obs_col = |idx: usize| -> &[f64] {
        observed
            .and_then(|o| o.objective_cols.get(idx))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    };
    let obs_x = obs_col(xi);
    let obs_y = obs_col(yi);
    let obs_z = obs_col(zi);

    // 範囲はフロント点と全観測点を含むようにする（観測点がクリップで潰れず、
    // トグルで表示を切り替えても軸スケールが変わらないように）。
    let range_for = |front_vals: &[f64], obs: &[f64]| -> (f64, f64) {
        let combined: Vec<f64> = front_vals.iter().chain(obs.iter()).copied().collect();
        compute_range_from_col(Some(&combined))
    };
    let (x_min, x_max) = range_for(&x_vals, obs_x);
    let (y_min, y_max) = range_for(&y_vals, obs_y);
    let (z_min, z_max) = range_for(&z_vals, obs_z);

    // 観測点を 3 分類に分ける。
    let (obs_front, obs_dominated, obs_infeasible) = match observed {
        Some(obs) => classify_observed_3d(obs, xi, yi, zi),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };

    // 予測フロント点。
    let front_pts: Vec<[f64; 3]> = result
        .front
        .iter()
        .map(|p| {
            [
                p.values.get(xi).copied().unwrap_or(0.0),
                p.values.get(yi).copied().unwrap_or(0.0),
                p.values.get(zi).copied().unwrap_or(0.0),
            ]
        })
        .collect();

    let x_name = result.objective_names.get(xi).cloned().unwrap_or_default();
    let y_name = result.objective_names.get(yi).cloned().unwrap_or_default();
    let z_name = result.objective_names.get(zi).cloned().unwrap_or_default();

    // 高さを固定した領域内にキャンバスを確保する（setup_3d_canvas は available_size を使うため）。
    let width = ui.available_width();
    ui.allocate_ui(egui::vec2(width, 280.0), |ui| {
        let (painter, _rect, project, _click_pos, _hover_pos) =
            setup_3d_canvas(ui, &mut state.multi_front_camera);
        draw_3d_grid(&painter, &project);
        draw_3d_axes(
            &painter,
            &project,
            [&x_name, &y_name, &z_name],
            [(x_min, x_max), (y_min, y_max), (z_min, z_max)],
        );

        // 1 群を投影・深度ソートして描画するヘルパー（共通ヘルパー・D-1）。
        let ranges = [(x_min, x_max), (y_min, y_max), (z_min, z_max)];
        let draw_group = |group: &[[f64; 3]], color: egui::Color32, radius: f32, stroke: bool| {
            let mut calls: Vec<DepthPoint> = group
                .iter()
                .map(|&p| {
                    let (pos, depth) = project_value_3d(&project, p, ranges);
                    DepthPoint {
                        pos,
                        depth,
                        color,
                        radius,
                    }
                })
                .collect();
            let stroke = stroke.then(|| egui::Stroke::new(1.0, egui::Color32::BLACK));
            draw_depth_sorted_points(&painter, &mut calls, stroke);
        };

        // 観測点（背面）→ 予測フロント（手前）の順に描画する。
        if toggles.infeasible {
            draw_group(&obs_infeasible, COLOR_INFEASIBLE(), 2.5, false);
        }
        if toggles.dominated {
            draw_group(&obs_dominated, COLOR_NON_PARETO(), 2.5, false);
        }
        if toggles.front {
            draw_group(&obs_front, COLOR_PARETO(), 3.5, false);
        }
        draw_group(&front_pts, COLOR_SURROGATE_FRONT(), 4.0, true);
    });
}
