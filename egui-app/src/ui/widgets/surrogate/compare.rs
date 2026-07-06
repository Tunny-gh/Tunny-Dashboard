//! Compare Surrogates ウィジェット。
//!
//! 選択した目的について全サロゲートモデル種別（Ridge / GP-FITC / GP-VFE / LightGBM、
//! および任意で GP-MOE）を一括でフィットし、CV 指標の比較表と、ベスト観測 trial を
//! アンカーとした 1 パラメータ方向の予測スライスのオーバーレイを表示する。HEEDS の
//! "Compare Surrogates" 相当。フィットは非同期（poll_chart.rs 経由）。
//!
//! アンカーは常にベスト観測 trial（`anchor::best_trial_row` で方向を考慮して解決）で、
//! `response_surface.rs` / `robustness.rs` と異なり pin 留め trial の選択 UI は持たない
//! （複数モデルを横並びで比較する用途では中心点を固定した方が比較しやすいため）。

use std::sync::Arc;

use tunny_core::surrogate_opt::{SurrogateModelKind, MIN_TRIALS_FOR_SURROGATE_OPT};

use crate::state::messages::{SurrogateCompareRow, SurrogateCompareUiResult};
use crate::theme::chart_colors::{
    COLOR_BAR_ACCENT, COLOR_BAR_NEGATIVE, COLOR_FIT_HIGH, COLOR_HIGHLIGHT_PT, COLOR_OPT_RUNNING,
    COLOR_OPT_TRIAL, COLOR_SCATTER_DOT,
};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};

/// 常に比較する基本モデル種別（表示・フィット順）。GP-MOE は計算コストが高いため
/// `include_moe` が有効なときのみ追加する。
const BASE_KINDS: [SurrogateModelKind; 4] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::Lgbm,
];

/// `include_moe` に応じてフィットするモデル種別の一覧を返す。
pub fn model_kinds(include_moe: bool) -> Vec<SurrogateModelKind> {
    let mut kinds = BASE_KINDS.to_vec();
    if include_moe {
        kinds.push(SurrogateModelKind::GpMoe);
    }
    kinds
}

/// オーバーレイプロットでのモデル別固定配色。
fn model_color(kind: SurrogateModelKind) -> egui::Color32 {
    match kind {
        SurrogateModelKind::Ridge => COLOR_BAR_ACCENT(),
        SurrogateModelKind::GpFitc => COLOR_OPT_TRIAL(),
        SurrogateModelKind::GpVfe => COLOR_HIGHLIGHT_PT(),
        SurrogateModelKind::Lgbm => COLOR_OPT_RUNNING(),
        SurrogateModelKind::GpMoe => COLOR_BAR_NEGATIVE(),
    }
}

/// 比較実行の計算リクエスト。poll_chart が消費する。
pub struct SurrogateCompareRequest {
    pub objective_index: usize,
    pub slice_param: usize,
    pub include_moe: bool,
}

/// Compare Surrogates ウィジェットの UI 状態。
/// 全フィールドの既定値が型の `Default` と一致するため、`#[derive(Default)]` で導出する
/// （`response_surface.rs` / `robustness.rs` は既定値が型の Default と異なるため手動実装している）。
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SurrogateCompareChart {
    pub selected_objective: usize,
    /// 1D オーバーレイのスライス対象パラメータ index（数値パラメータ一覧内）。
    pub slice_param: usize,
    /// GP-MOE は低速なため既定オフ。
    pub include_moe: bool,

    #[serde(skip)]
    pub pending: Option<SurrogateCompareRequest>,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub error: Option<String>,
    #[serde(skip)]
    pub result: Option<Arc<SurrogateCompareUiResult>>,
}

impl SurrogateCompareChart {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む
    /// （`ComputeSyncKind::SurrogateCompare` から呼ぶ。`robustness.rs` と同じ規約）。
    pub fn adopt_compute_state(&mut self, global: &Self) {
        self.computing = global.computing;
        self.error = global.error.clone();
        self.result = global.result.clone();
    }
}

/// `obj_names` は現在の Study の全目的。`param_names` は数値パラメータ一覧
/// （スライス対象コンボの候補、カテゴリカル列は除外済みのものを渡すこと）。
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurrogateCompareChart,
    obj_names: &[String],
    param_names: &[String],
    trial_count: usize,
) {
    if obj_names.is_empty() {
        ui.label("No objectives available.");
        return;
    }
    if param_names.is_empty() {
        ui.label("No numeric parameters available for surrogate comparison.");
        return;
    }
    if state.selected_objective >= obj_names.len() {
        state.selected_objective = 0;
    }
    if state.slice_param >= param_names.len() {
        state.slice_param = 0;
    }

    let can_compare =
        !state.computing && state.pending.is_none() && trial_count >= MIN_TRIALS_FOR_SURROGATE_OPT;

    ui.horizontal(|ui| {
        ui.label("Objective:");
        egui::ComboBox::from_id_salt("surrogate_compare_obj")
            .selected_text(obj_names[state.selected_objective].as_str())
            .show_ui(ui, |ui| {
                for (i, name) in obj_names.iter().enumerate() {
                    ui.selectable_value(&mut state.selected_objective, i, name);
                }
            });

        ui.label("Slice param:");
        egui::ComboBox::from_id_salt("surrogate_compare_slice_param")
            .selected_text(param_names[state.slice_param].as_str())
            .show_ui(ui, |ui| {
                for (i, name) in param_names.iter().enumerate() {
                    ui.selectable_value(&mut state.slice_param, i, name);
                }
            });

        ui.checkbox(&mut state.include_moe, "include MoE");

        if ui
            .add_enabled(can_compare, egui::Button::new("Compare"))
            .clicked()
        {
            state.error = None;
            state.computing = true;
            state.pending = Some(SurrogateCompareRequest {
                objective_index: state.selected_objective,
                slice_param: state.slice_param,
                include_moe: state.include_moe,
            });
        }
    });

    if trial_count < MIN_TRIALS_FOR_SURROGATE_OPT {
        ui.label(
            egui::RichText::new(format!(
                "At least {} trials required (current: {})",
                MIN_TRIALS_FOR_SURROGATE_OPT, trial_count
            ))
            .weak(),
        );
        return;
    }

    if state.computing {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Fitting surrogate models...");
        });
    }

    if let Some(err) = state.error.clone() {
        ui.colored_label(egui::Color32::RED, err);
    }

    let Some(result) = state.result.clone() else {
        return;
    };

    render_metrics_table(ui, &result);
    ui.add_space(6.0);
    render_overlay_plot(ui, &result);
    let anchor_text = result
        .anchor
        .iter()
        .map(|v| format!("{v:.4}"))
        .collect::<Vec<_>>()
        .join(", ");
    ui.label(
        egui::RichText::new("Slice through the best observed trial (other parameters fixed)")
            .weak(),
    )
    .on_hover_text(format!(
        "Anchor (all parameters, original units): [{anchor_text}]"
    ));
}

/// CV 指標比較表を描画する（CV R² 降順、失敗モデルは末尾）。
fn render_metrics_table(ui: &mut egui::Ui, result: &SurrogateCompareUiResult) {
    ui.strong(format!("CV metrics — {}", result.objective_name));

    let mut ranked: Vec<&SurrogateCompareRow> = result.rows.iter().collect();
    ranked.sort_by(|a, b| match (a.error.is_none(), b.error.is_none()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b
            .cv_r2_mean
            .partial_cmp(&a.cv_r2_mean)
            .unwrap_or(std::cmp::Ordering::Equal),
    });
    let best_kind = ranked.first().filter(|r| r.error.is_none()).map(|r| r.kind);

    egui::Grid::new("surrogate_compare_metrics")
        .striped(true)
        .min_col_width(90.0)
        .show(ui, |ui| {
            ui.strong("Model");
            ui.strong("CV R² (±std)");
            ui.strong("Holdout R²");
            ui.strong("Holdout RMSE");
            ui.strong("Train R²");
            ui.end_row();

            for row in &ranked {
                let is_best = row.error.is_none() && Some(row.kind) == best_kind;
                let name = if is_best {
                    format!("✓ {}", super::surrogate_opt::model_label(row.kind))
                } else {
                    super::surrogate_opt::model_label(row.kind).to_string()
                };
                if is_best {
                    ui.colored_label(COLOR_FIT_HIGH(), name);
                } else {
                    ui.label(name);
                }

                if let Some(err) = &row.error {
                    ui.colored_label(crate::theme::TEXT_SECONDARY(), err);
                    ui.label("—");
                    ui.label("—");
                    ui.label("—");
                } else {
                    ui.monospace(format!("{:.3} ± {:.3}", row.cv_r2_mean, row.cv_r2_std));
                    ui.monospace(format!("{:.3}", row.holdout_r2));
                    ui.monospace(format!("{:.6}", row.holdout_rmse));
                    ui.monospace(format!("{:.3}", row.train_r2));
                }
                ui.end_row();
            }
        });
}

/// 観測データ + モデルごとの 1D 予測スライスを重ねたオーバーレイプロットを描画する。
fn render_overlay_plot(ui: &mut egui::Ui, result: &SurrogateCompareUiResult) {
    let observed_pts: Vec<[f64; 2]> = result.observed.iter().map(|&(x, y)| [x, y]).collect();

    egui_plot::Plot::new("surrogate_compare_overlay")
        .unified_nav()
        .x_axis_label(&result.param_name)
        .y_axis_label(&result.objective_name)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            plot_ui.points(
                egui_plot::Points::new("Observed", observed_pts)
                    .radius(2.5)
                    .color(COLOR_SCATTER_DOT().gamma_multiply(0.5)),
            );
            for (kind, slice) in &result.slices {
                let pts: Vec<[f64; 2]> = slice
                    .x_values
                    .iter()
                    .zip(&slice.y_values)
                    .map(|(&x, &y)| [x, y])
                    .collect();
                let cv_r2 = result
                    .rows
                    .iter()
                    .find(|r| r.kind == *kind && r.error.is_none())
                    .map(|r| r.cv_r2_mean);
                let label = match cv_r2 {
                    Some(r2) => format!(
                        "{} (CV R² {:.2})",
                        super::surrogate_opt::model_label(*kind),
                        r2
                    ),
                    None => super::surrogate_opt::model_label(*kind).to_string(),
                };
                plot_ui.line(egui_plot::Line::new(label, pts).color(model_color(*kind)));
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surrogate_compare_chart_default_values() {
        let s = SurrogateCompareChart::default();
        assert_eq!(s.selected_objective, 0);
        assert_eq!(s.slice_param, 0);
        assert!(!s.include_moe);
        assert!(s.pending.is_none());
        assert!(!s.computing);
        assert!(s.error.is_none());
        assert!(s.result.is_none());
    }

    #[test]
    fn adopt_compute_state_propagates_and_keeps_selection() {
        let src = SurrogateCompareChart {
            computing: false,
            error: Some("err".into()),
            ..Default::default()
        };
        let mut dst = SurrogateCompareChart {
            computing: true,
            selected_objective: 2,
            slice_param: 1,
            ..Default::default()
        };
        dst.adopt_compute_state(&src);
        assert!(!dst.computing);
        assert_eq!(dst.error.as_deref(), Some("err"));
        // UI 選択は維持される。
        assert_eq!(dst.selected_objective, 2);
        assert_eq!(dst.slice_param, 1);
    }

    #[test]
    fn model_kinds_includes_moe_only_when_requested() {
        assert_eq!(model_kinds(false).len(), 4);
        assert!(!model_kinds(false).contains(&SurrogateModelKind::GpMoe));
        assert_eq!(model_kinds(true).len(), 5);
        assert!(model_kinds(true).contains(&SurrogateModelKind::GpMoe));
    }
}
