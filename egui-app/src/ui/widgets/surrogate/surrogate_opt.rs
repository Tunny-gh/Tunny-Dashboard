//! サロゲート最適化ウィジェット。
//!
//! サンプリング結果（trial 群）から応答曲面（サロゲートモデル）を学習し、
//! その曲面上で最適化を実行して推定最適点を表示する。計算は
//! `tunny_core::surrogate_opt` がバックグラウンドで行う（poll_chart.rs 参照）。

use crate::state::messages::SurrogateOptUiResult;
use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{SurrogateOptComputeRequest, SurrogateOptState};
use crate::ui::widgets::surface_plot::{draw_colorbar_simple, draw_heatmap, value_range};
use tunny_core::surrogate_opt::{OptimizerKind, SurrogateModelKind, MIN_TRIALS_FOR_SURROGATE_OPT};

/// モデル選択肢（コンボ表示順）。新モデル追加時はここへ並べる。
const MODEL_CHOICES: [SurrogateModelKind; 3] = [
    SurrogateModelKind::Kriging,
    SurrogateModelKind::SparseKriging,
    SurrogateModelKind::Ridge,
];

/// 最適化手法の選択肢（コンボ表示順）。
const OPTIMIZER_CHOICES: [OptimizerKind; 4] = [
    OptimizerKind::MultiStartLbfgs,
    OptimizerKind::Nsga2,
    OptimizerKind::CmaEs,
    OptimizerKind::RandomSearch,
];

pub(crate) fn model_label(kind: SurrogateModelKind) -> &'static str {
    match kind {
        SurrogateModelKind::Ridge => "Ridge",
        SurrogateModelKind::Kriging => "Kriging",
        SurrogateModelKind::SparseKriging => "Sparse Kriging",
    }
}

pub(crate) fn optimizer_label(kind: OptimizerKind) -> &'static str {
    match kind {
        OptimizerKind::MultiStartLbfgs => "Multi-start L-BFGS",
        OptimizerKind::Nsga2 => "NSGA-II",
        OptimizerKind::CmaEs => "CMA-ES",
        OptimizerKind::RandomSearch => "Random Search",
    }
}

/// `param_names` は数値パラメータのみ（カテゴリカル列は最適化対象にしない）。
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    obj_names: &[String],
    cmap: ColorMap,
    trial_count: usize,
) {
    if param_names.is_empty() {
        ui.label("No numeric parameters available for surrogate optimization.");
        return;
    }

    // ── 1段目: 目的・モデル・最適化手法 ─────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Objective:");
        let obj_text = obj_names
            .get(state.selected_objective)
            .map(|s| s.as_str())
            .unwrap_or("—");
        egui::ComboBox::from_id_salt("surrogate_obj")
            .selected_text(obj_text)
            .show_ui(ui, |ui| {
                for (i, name) in obj_names.iter().enumerate() {
                    if ui
                        .selectable_label(state.selected_objective == i, name)
                        .clicked()
                    {
                        state.selected_objective = i;
                    }
                }
            });

        ui.label("Model:");
        egui::ComboBox::from_id_salt("surrogate_model")
            .selected_text(model_label(state.model))
            .show_ui(ui, |ui| {
                for kind in MODEL_CHOICES {
                    ui.selectable_value(&mut state.model, kind, model_label(kind));
                }
            });

        ui.label("Optimizer:");
        egui::ComboBox::from_id_salt("surrogate_optimizer")
            .selected_text(optimizer_label(state.optimizer))
            .show_ui(ui, |ui| {
                for kind in OPTIMIZER_CHOICES {
                    ui.selectable_value(&mut state.optimizer, kind, optimizer_label(kind));
                }
            });
    });

    // スライス軸のデフォルト（先頭 2 パラメータ）。Study 切替で消えた名前もリセットする。
    if !param_names.contains(&state.slice_x) {
        state.slice_x = param_names.first().cloned().unwrap_or_default();
    }
    if !param_names.contains(&state.slice_y) || state.slice_y == state.slice_x {
        state.slice_y = param_names
            .iter()
            .find(|p| **p != state.slice_x)
            .cloned()
            .unwrap_or_default();
    }

    // ── 2段目: スライス表示軸 + 実行ボタン ──────────────────────────
    ui.horizontal(|ui| {
        ui.label("Surface X:");
        egui::ComboBox::from_id_salt("surrogate_slice_x")
            .selected_text(&state.slice_x)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.slice_x, name.clone(), name);
                }
            });
        ui.label("Y:");
        egui::ComboBox::from_id_salt("surrogate_slice_y")
            .selected_text(&state.slice_y)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.slice_y, name.clone(), name);
                }
            });

        let can_run = !obj_names.is_empty()
            && trial_count >= MIN_TRIALS_FOR_SURROGATE_OPT
            && !state.computing
            && state.pending_compute.is_none();
        if ui
            .add_enabled(can_run, egui::Button::new("Run Optimization"))
            .clicked()
        {
            if let Some(obj_name) = obj_names.get(state.selected_objective) {
                state.error_message = None;
                state.pending_compute = Some(SurrogateOptComputeRequest {
                    objective: obj_name.clone(),
                    model: state.model,
                    optimizer: state.optimizer,
                    slice_x: state.slice_x.clone(),
                    slice_y: state.slice_y.clone(),
                });
            }
        }
    });

    if trial_count < MIN_TRIALS_FOR_SURROGATE_OPT {
        ui.colored_label(
            egui::Color32::RED,
            format!(
                "At least {} trials required (current: {})",
                MIN_TRIALS_FOR_SURROGATE_OPT, trial_count
            ),
        );
        return;
    }

    if state.computing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Fitting surrogate and optimizing…");
        });
        return;
    }

    if let Some(ref err) = state.error_message.clone() {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }

    let Some(result) = &state.result else {
        ui.label("No result. Choose a model and click Run Optimization.");
        return;
    };

    render_result(ui, result, cmap);
}

fn render_result(ui: &mut egui::Ui, result: &SurrogateOptUiResult, cmap: ColorMap) {
    let direction = if result.minimize {
        "minimize"
    } else {
        "maximize"
    };
    let value_text = match result.predicted_std {
        Some(std) => format!("{:.6} ± {:.6}", result.best_value, 1.96 * std),
        None => format!("{:.6}", result.best_value),
    };
    ui.horizontal(|ui| {
        ui.strong(format!(
            "Predicted optimum of {} ({}):",
            result.objective_name, direction
        ));
        ui.monospace(value_text);
    });
    ui.label(format!("Surrogate R² = {:.3}", result.r_squared));

    // ── 推定最適点のパラメータ値テーブル ────────────────────────────
    ui.add_space(4.0);
    egui::Grid::new("surrogate_best_params")
        .striped(true)
        .min_col_width(80.0)
        .show(ui, |ui| {
            ui.strong("Parameter");
            ui.strong("Best value");
            ui.end_row();
            for (name, value) in &result.best_params {
                ui.label(name);
                ui.monospace(format!("{:.6}", value));
                ui.end_row();
            }
        });

    // ── 最適点を通る応答曲面スライス（ヒートマップ＋最適点マーカー） ──
    let Some(slice) = &result.slice else {
        return;
    };
    if slice.z_values.is_empty() {
        return;
    }
    let (Some(px_name), Some(py_name)) = (
        result.best_params.get(slice.param_x_idx),
        result.best_params.get(slice.param_y_idx),
    ) else {
        return;
    };

    ui.add_space(6.0);
    ui.label(format!(
        "Response surface through the optimum — X: {} (→), Y: {} (↑)",
        px_name.0, py_name.0
    ));

    let available = ui.available_rect_before_wrap();
    let plot_size = egui::vec2(
        (available.width() - 32.0).max(100.0),
        available.height().clamp(60.0, 300.0),
    );
    let (rect, _) = ui.allocate_exact_size(plot_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // 表示用に向きを揃える: 横 = param_x（左→右で増加）、縦 = param_y（上 = 最大）。
    // core の slice は z[i][j] = f(x_i, y_j) なので disp[r][c] = z[c][ny-1-r]。
    let nx = slice.x_values.len();
    let ny = slice.y_values.len();
    let display: Vec<Vec<f64>> = (0..ny)
        .map(|r| (0..nx).map(|c| slice.z_values[c][ny - 1 - r]).collect())
        .collect();
    draw_heatmap(&painter, rect, &display, cmap.clone());

    // 最適点マーカー（白丸＋黒縁）。
    let (x_min, x_max) = (slice.x_values[0], slice.x_values[nx - 1]);
    let (y_min, y_max) = (slice.y_values[0], slice.y_values[ny - 1]);
    if x_max > x_min && y_max > y_min {
        let fx = ((px_name.1 - x_min) / (x_max - x_min)).clamp(0.0, 1.0) as f32;
        let fy = ((py_name.1 - y_min) / (y_max - y_min)).clamp(0.0, 1.0) as f32;
        let marker = egui::pos2(
            rect.left() + fx * rect.width(),
            rect.bottom() - fy * rect.height(),
        );
        painter.circle_filled(marker, 5.0, egui::Color32::WHITE);
        painter.circle_stroke(marker, 5.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
    }

    let (v_min, v_max) = value_range(&display);
    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() + 4.0, rect.top()),
        egui::vec2(16.0, rect.height()),
    );
    draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widget_states::SurrogateOptState;

    fn ui_result(slice: Option<tunny_core::surrogate_opt::SurfaceSlice>) -> SurrogateOptUiResult {
        SurrogateOptUiResult {
            best_params: vec![("x".to_string(), 0.3), ("y".to_string(), 0.7)],
            best_value: 0.01,
            predicted_std: Some(0.005),
            r_squared: 0.95,
            objective_name: "obj0".to_string(),
            minimize: true,
            slice,
        }
    }

    #[test]
    fn run_click_builds_request_from_selections() {
        let mut state = SurrogateOptState {
            slice_x: "x".to_string(),
            slice_y: "y".to_string(),
            ..Default::default()
        };
        let obj_names = ["obj0".to_string()];

        // show() の Run ボタン押下と同じロジック
        if let Some(obj_name) = obj_names.get(state.selected_objective) {
            state.pending_compute = Some(SurrogateOptComputeRequest {
                objective: obj_name.clone(),
                model: state.model,
                optimizer: state.optimizer,
                slice_x: state.slice_x.clone(),
                slice_y: state.slice_y.clone(),
            });
        }

        let req = state.pending_compute.as_ref().unwrap();
        assert_eq!(req.objective, "obj0");
        assert_eq!(req.model, SurrogateModelKind::Kriging);
        assert_eq!(req.optimizer, OptimizerKind::MultiStartLbfgs);
        assert_eq!(req.slice_x, "x");
        assert_eq!(req.slice_y, "y");
    }

    #[test]
    fn result_arrival_switches_spinner_off() {
        let mut state = SurrogateOptState {
            computing: true,
            ..Default::default()
        };
        state.result = Some(ui_result(None));
        state.computing = false;
        assert!(!state.computing);
        assert!(state.result.is_some());
    }

    #[test]
    fn adopt_compute_state_keeps_selections() {
        let global = SurrogateOptState {
            result: Some(ui_result(None)),
            computing: false,
            error_message: Some("err".to_string()),
            ..Default::default()
        };

        let mut item = SurrogateOptState {
            model: SurrogateModelKind::Ridge,
            optimizer: OptimizerKind::RandomSearch,
            selected_objective: 1,
            computing: true,
            ..Default::default()
        };
        item.adopt_compute_state(&global);

        assert!(!item.computing);
        assert!(item.result.is_some());
        assert_eq!(item.error_message.as_deref(), Some("err"));
        // 選択は維持される
        assert_eq!(item.model, SurrogateModelKind::Ridge);
        assert_eq!(item.optimizer, OptimizerKind::RandomSearch);
        assert_eq!(item.selected_objective, 1);
    }

    #[test]
    fn labels_cover_all_choices() {
        for kind in MODEL_CHOICES {
            assert!(!model_label(kind).is_empty());
        }
        for kind in OPTIMIZER_CHOICES {
            assert!(!optimizer_label(kind).is_empty());
        }
    }
}
