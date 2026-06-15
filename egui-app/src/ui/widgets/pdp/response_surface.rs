//! ResponseSurfacePlot ウィジェット。
//!
//! 学習済みサロゲート（Optimizer と同じ `tunny_core::surrogate_opt` エンジン）から
//! `目的関数 = f(全パラメータ)` の応答曲面を生成し、目的関数を縦軸にとった3D
//! サーフェスとして描画する。X / Y はパラメータ、縦軸は予測した目的値。
//!
//! 学習結果（`TrainedSurrogate`）は `WidgetStates.surrogate_cache` を介して Optimizer と
//! 共有され、どちらでフィットしても相互に再利用される（再フィット不要）。3D 描画は
//! `pdp_2d` のメッシュ描画ヘルパーを再利用する。

use crate::theme::colormap::ColorMap;
use crate::ui::widget_states::{
    ResponseSurfacePlotState, ResponseSurfaceSliceRequest, SurrogateCache, SurrogateKey,
};
use crate::ui::widgets::pdp::pdp_2d::{
    axis_range_of, draw_colorbar, draw_surface_mesh, value_range_of,
};
use crate::ui::widgets::scatter_3d::{
    axis_segments_3d, draw_3d_axis_labels, draw_3d_grid, setup_3d_canvas,
};
use crate::ui::widgets::surrogate::surrogate_opt::model_label;
use tunny_core::surrogate_opt::{SurrogateModelKind, MIN_TRIALS_FOR_SURROGATE_OPT};

/// スライス格子の一辺の点数。
const N_GRID: usize = 24;

/// 95% CI バンドの (下限グリッド, 上限グリッド)。
type BandPair = (Vec<Vec<f64>>, Vec<Vec<f64>>);

/// Model コンボ（コンボ表示順）。Auto は別エントリで先頭に出す。
const MODEL_CHOICES: [SurrogateModelKind; 5] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::GpMoe,
    SurrogateModelKind::Lgbm,
];

const AUTO_MODEL_LABEL: &str = "Auto (cross-validated)";

/// 95% CI バンドの (下限, 上限) グリッドを z_std から直接作る。
/// `lower[i][j] = z - 1.96σ`, `upper[i][j] = z + 1.96σ`。
fn band_grids_from_std(z_values: &[Vec<f64>], z_std: &[Vec<f64>]) -> BandPair {
    let mut lower = Vec::with_capacity(z_values.len());
    let mut upper = Vec::with_capacity(z_values.len());
    for (z_row, std_row) in z_values.iter().zip(z_std.iter()) {
        let mut l_row = Vec::with_capacity(z_row.len());
        let mut u_row = Vec::with_capacity(z_row.len());
        for (&z, &std) in z_row.iter().zip(std_row.iter()) {
            let s = std.max(0.0);
            l_row.push(z - 1.96 * s);
            u_row.push(z + 1.96 * s);
        }
        lower.push(l_row);
        upper.push(u_row);
    }
    (lower, upper)
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut ResponseSurfacePlotState,
    cache: &SurrogateCache,
    param_names: &[String],
    obj_names: &[String],
    minimize_for_obj: &[bool],
    cmap: ColorMap,
    trial_count: usize,
    has_constraints: bool,
) {
    // ── 早期リターン ─────────────────────────────────────────────
    if param_names.len() < 2 {
        ui.label("Need at least 2 numeric parameters for a response surface.");
        return;
    }
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

    // スライス軸のデフォルト（Study 切替で消えた名前もリセット）。
    if !param_names.contains(&state.selected_x) {
        state.selected_x = param_names[0].clone();
    }
    if !param_names.contains(&state.selected_y) || state.selected_y == state.selected_x {
        state.selected_y = param_names
            .iter()
            .find(|p| **p != state.selected_x)
            .cloned()
            .unwrap_or_default();
    }
    if state.selected_objective >= obj_names.len() {
        state.selected_objective = 0;
    }

    // ── X / Y パラメータ ─────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("X:");
        egui::ComboBox::from_id_salt("rsp_x")
            .selected_text(&state.selected_x)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.selected_x, name.clone(), name);
                }
            });
        ui.label("Y:");
        egui::ComboBox::from_id_salt("rsp_y")
            .selected_text(&state.selected_y)
            .show_ui(ui, |ui| {
                for name in param_names {
                    ui.selectable_value(&mut state.selected_y, name.clone(), name);
                }
            });
    });

    // ── Objective / Model ────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label("Objective:");
        let obj_text = obj_names
            .get(state.selected_objective)
            .map(|s| s.as_str())
            .unwrap_or("—");
        egui::ComboBox::from_id_salt("rsp_obj")
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
        let selected_text = if state.auto_select {
            AUTO_MODEL_LABEL
        } else {
            model_label(state.model)
        };
        egui::ComboBox::from_id_salt("rsp_model")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(state.auto_select, AUTO_MODEL_LABEL)
                    .clicked()
                {
                    state.auto_select = true;
                }
                for kind in MODEL_CHOICES {
                    let selected = !state.auto_select && state.model == kind;
                    if ui.selectable_label(selected, model_label(kind)).clicked() {
                        state.auto_select = false;
                        state.model = kind;
                    }
                }
            });

        if has_constraints {
            ui.separator();
            ui.toggle_value(&mut state.use_constraints, "Use constraints")
                .on_hover_text("Fit constraint surrogates and account for feasibility");
        }
    });

    // 同一パラメータ警告。
    if state.selected_x == state.selected_y {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Warning: same parameter selected for X and Y",
        );
    }

    // ── 現在の選択に対応する共有キャッシュキー ───────────────────
    let objective = obj_names
        .get(state.selected_objective)
        .cloned()
        .unwrap_or_default();
    let use_constraints = has_constraints && state.use_constraints;
    let key = SurrogateKey {
        objective: objective.clone(),
        model: if state.auto_select {
            None
        } else {
            Some(state.model)
        },
        use_constraints,
    };

    // ── Fit ボタン ───────────────────────────────────────────────
    let can_fit = !state.fitting
        && !state.computing_slice
        && state.selected_x != state.selected_y
        && !obj_names.is_empty();
    if ui
        .add_enabled(can_fit, egui::Button::new("Fit & show surface"))
        .clicked()
    {
        state.error_message = None;
        state.result = None;
        state.fitting = true;
        state.pending_fit = Some(crate::ui::widget_states::SurrogateFitComputeRequest {
            objective: objective.clone(),
            model: state.model,
            auto_select: state.auto_select,
            use_constraints,
        });
    }

    if state.fitting {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Fitting surrogate…");
        });
        return;
    }

    // ── 学習済みモデルの参照（Optimizer 側でフィット済みでも再利用）──
    let Some(trained) = cache.get(&key) else {
        if let Some(ref err) = state.error_message {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
        } else {
            ui.label("Fit a surrogate (here or in the Optimizer) to see the response surface.");
        }
        return;
    };

    // 学習済みモデルがあれば、現在の X/Y/目的に対応する曲面スライスを用意する。
    let desired_label = model_label(trained.model_kind).to_string();
    let stale = state.result.as_ref().is_none_or(|r| {
        r.objective_name != objective
            || r.param_x_name != state.selected_x
            || r.param_y_name != state.selected_y
            || r.model_label != desired_label
    });
    if stale && !state.computing_slice {
        let minimize = minimize_for_obj
            .get(state.selected_objective)
            .copied()
            .unwrap_or(true);
        state.computing_slice = true;
        state.pending_slice = Some(ResponseSurfaceSliceRequest {
            key: key.clone(),
            param_x: state.selected_x.clone(),
            param_y: state.selected_y.clone(),
            objective: objective.clone(),
            model_label: desired_label,
            minimize,
            n_grid: N_GRID,
        });
    }

    if state.computing_slice {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Computing response surface…");
        });
        return;
    }

    if let Some(ref err) = state.error_message {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }

    // ── 3D 応答曲面の描画（result が現在の選択に一致しているとき）──
    let has_std = state
        .result
        .as_ref()
        .is_some_and(|r| r.slice.z_std.as_ref().is_some_and(|g| !g.is_empty()));
    if has_std {
        ui.checkbox(&mut state.show_uncertainty, "95% CI (±1.96σ)")
            .on_hover_text("Overlay the GP posterior uncertainty band");
    }

    let camera = &mut state.camera;
    let Some(result) = state.result.as_ref() else {
        return;
    };
    let slice = &result.slice;
    let values: &[Vec<f64>] = &slice.z_values;
    if values.len() < 2 || values[0].len() < 2 {
        ui.label("Not enough grid data for a 3D surface.");
        return;
    }
    let value_label = result.objective_name.clone();

    // 不確実性バンド（GP 系で z_std があり、トグル ON のときのみ）。
    let bands: Option<BandPair> = if state.show_uncertainty && has_std {
        slice
            .z_std
            .as_ref()
            .map(|std| band_grids_from_std(values, std))
    } else {
        None
    };

    // 色は予測平均（z_values）の値域で正規化する。
    let (c_min, c_max) = value_range_of(values);
    let (mut v_min, mut v_max) = (c_min, c_max);
    if let Some((lower, upper)) = &bands {
        let (l_min, _) = value_range_of(lower);
        let (_, u_max) = value_range_of(upper);
        v_min = v_min.min(l_min);
        v_max = v_max.max(u_max);
    }
    let (x_min, x_max) = axis_range_of(&slice.x_values);
    let (z_min, z_max) = axis_range_of(&slice.y_values);

    let avail = ui.available_size();
    let canvas_size = egui::vec2((avail.x - 72.0).max(120.0), avail.y.max(160.0));
    ui.allocate_ui(canvas_size, |ui| {
        ui.set_min_size(canvas_size);
        let (painter, rect, project) = setup_3d_canvas(ui, camera);
        draw_3d_grid(&painter, &project);
        // 観測点は重ねない（純粋なサロゲート曲面）。
        let no_points: Vec<([f32; 3], egui::Color32)> = Vec::new();
        draw_surface_mesh(
            &painter,
            &project,
            values,
            (v_min, v_max),
            (c_min, c_max),
            &cmap,
            bands.as_ref().map(|(l, u)| (l, u)),
            &no_points,
            &axis_segments_3d(24),
        );
        // X = param_x, Y(縦) = 目的値, Z = param_y。
        draw_3d_axis_labels(
            &painter,
            &project,
            [&result.param_x_name, &value_label, &result.param_y_name],
            [(x_min, x_max), (v_min, v_max), (z_min, z_max)],
        );
        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() + 4.0, rect.top()),
            egui::vec2(16.0, rect.height()),
        );
        draw_colorbar(ui, bar_rect, c_min, c_max, cmap.clone());
    });

    ui.label(format!(
        "Model: {} | CV R² = {:.3}",
        result.model_label, result.cv_r2_mean
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::messages::ResponseSurfaceResult;

    #[test]
    fn band_grids_from_std_computes_95_ci() {
        // σ = 2 → ±1.96×2 = ±3.92
        let z = vec![vec![10.0, 20.0]];
        let std = vec![vec![2.0, 0.0]];
        let (lower, upper) = band_grids_from_std(&z, &std);
        assert!((lower[0][0] - (10.0 - 3.92)).abs() < 1e-9);
        assert!((upper[0][0] - (10.0 + 3.92)).abs() < 1e-9);
        // σ = 0 → バンドは平均に一致。
        assert_eq!(lower[0][1], 20.0);
        assert_eq!(upper[0][1], 20.0);
    }

    #[test]
    fn band_grids_from_std_clamps_negative_std() {
        let z = vec![vec![5.0]];
        let std = vec![vec![-1.0]];
        let (lower, upper) = band_grids_from_std(&z, &std);
        assert_eq!(lower[0][0], 5.0);
        assert_eq!(upper[0][0], 5.0);
    }

    #[test]
    fn default_state_is_not_busy() {
        let state = ResponseSurfacePlotState::default();
        assert!(!state.fitting);
        assert!(!state.computing_slice);
        assert!(state.result.is_none());
        assert!(state.show_uncertainty);
    }

    #[test]
    fn result_signature_helps_detect_staleness() {
        // 同一署名なら stale でない、目的が変われば stale。
        let r = ResponseSurfaceResult {
            slice: tunny_core::surrogate_opt::SurfaceSlice {
                param_x_idx: 0,
                param_y_idx: 1,
                x_values: vec![0.0, 1.0],
                y_values: vec![0.0, 1.0],
                z_values: vec![vec![0.0, 1.0], vec![1.0, 2.0]],
                z_std: None,
            },
            param_x_name: "x".into(),
            param_y_name: "y".into(),
            objective_name: "f".into(),
            model_label: "Ridge".into(),
            cv_r2_mean: 0.5,
        };
        let same = r.objective_name == "f" && r.param_x_name == "x" && r.param_y_name == "y";
        assert!(same);
    }
}
