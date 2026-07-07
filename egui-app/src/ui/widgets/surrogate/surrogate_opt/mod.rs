//! サロゲート最適化ウィジェット。
//!
//! サンプリング結果（trial 群）から応答曲面（サロゲートモデル）を学習し、
//! その曲面上で最適化を実行して推定最適点を表示する。計算は
//! `tunny_core::surrogate_opt` がバックグラウンドで行う（poll_chart.rs 参照）。
//!
//! 2 段階フロー:
//!   1. Fit & Validate: ホールドアウト + 5-fold CV で検証指標を表示。
//!   2. Run Optimization: 学習済みモデル上で最適化を実行。
//!
//! レイアウト:
//!   全幅前段: 数値パラメータ無し / trial 数不足 の早期リターン。
//!   左列 (Fit & Validate): Objective + Model コンボ → Fit & Validate ボタン →
//!       フィット中スピナー → 検証指標グリッド + 品質判定 + OOF 散布図。
//!   右列 (Optimization): Optimizer / Surface X / Y コンボ →
//!       Run Optimization ボタン → 最適化中スピナー → 結果。
//!
//! モジュール構成（責務ごとに分割。外部から見える公開 API は本 mod で維持する）:
//!   - `labels`: ラベル・選択肢・品質判定の純粋ヘルパー。
//!   - `fit`: 左列（フィット・検証・OOF プロット）。
//!   - `optimize`: 右列（最適化列・結果サマリ・履歴プロット）。
//!   - `front_view`: 予測パレートフロントの 2D/3D 散布図。
//!   - `tables`: 推定最適点・フロント点の表形式レンダリング。
//!   - `suggest`: 候補提案（EI/LCB・EHVI）の結果テーブル。

mod fit;
mod front_view;
mod labels;
mod optimize;
mod suggest;
mod tables;

use crate::ui::widget_states::SurrogateOptState;
use tunny_core::surrogate_opt::{TrainedSurrogate, MIN_TRIALS_FOR_SURROGATE_OPT};

use fit::{render_fit_column, render_fit_column_multi};
use optimize::{render_optimize_column, render_optimize_column_multi};

// 分割前と同じ `surrogate_opt::model_label` のパスを維持するための再エクスポート。
// 他ウィジェット（robustness / compare / response_surface）・CSV エクスポートが参照する。
pub(crate) use labels::model_label;

/// 多目的フロント散布図に重ねる観測（既存 trial）データ。
/// すべて trial 行順に整列し、`objective_cols` は `multi_result` の目的順に並ぶ。
/// 観測点を ParetoScatter と同様にパレートフロント / 被支配 / 実行不可能へ分類するために使う。
pub struct ObservedData<'a> {
    /// 目的ごとの全 trial 観測値（`multi_result.objective_names` の順）。
    pub objective_cols: &'a [Vec<f64>],
    /// 各 trial の Pareto ランク（0 = 観測フロント）。
    pub pareto_rank: &'a [u32],
    /// 各 trial が feasible か。
    pub feasible: &'a [bool],
}

/// `param_names` は数値パラメータのみ（カテゴリカル列は最適化対象にしない）。
/// `obj_history` は現在の結果が参照する目的列の全値（trial 順）。プロット用。
/// `observed` は多目的フロント散布図に重ねる観測点（結果が無いときは None）。
/// `constraint_col_names` は制約列名（制約付き Study のみ非空）。
#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    state: &mut SurrogateOptState,
    param_names: &[String],
    obj_names: &[String],
    trial_count: usize,
    obj_history: Option<&[f64]>,
    observed: Option<&ObservedData>,
    constraint_col_names: &[String],
) {
    // ── 全幅前段: 数値パラメータ無し ──────────────────────────────
    if param_names.is_empty() {
        ui.label("No numeric parameters available for surrogate optimization.");
        return;
    }

    // ── 全幅前段: trial 数不足 ─────────────────────────────────────
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

    // ── 多目的モード切替チェックボックス（目的が 2 つ以上の時のみ表示） ──
    if obj_names.len() >= 2 {
        let prev_multi = state.multi_objective;
        ui.checkbox(
            &mut state.multi_objective,
            "Multi-objective (all objectives)",
        );
        if state.multi_objective != prev_multi {
            // モード切替時にエラーをクリアする。
            state.error_message = None;
        }
    }

    let busy = state.fitting
        || state.optimizing
        || state.suggesting
        || state.multi_suggesting
        || state.pending_fit.is_some()
        || state.pending_optimize.is_some()
        || state.pending_multi_fit.is_some()
        || state.pending_multi_optimize.is_some()
        || state.pending_suggest.is_some()
        || state.pending_multi_suggest.is_some();

    let has_matching_trained = state
        .trained
        .as_deref()
        .map(|t| trained_matches(t, state, obj_names))
        .unwrap_or(false);

    let has_matching_multi_trained = state
        .multi_trained
        .as_deref()
        .map(|v| multi_trained_matches(v, state, obj_names))
        .unwrap_or(false);

    // ── 2 列レイアウト ─────────────────────────────────────────────
    // trial_detail_modal と同じ慣用: horizontal_top + allocate_ui_with_layout で
    // 各列を等幅に区切る。
    let available_w = ui.available_width();
    let col_w = (available_w / 2.0).max(200.0);

    // エラー表示（フィット・最適化どちらの失敗も全幅で出す）。
    if let Some(ref err) = state.error_message.clone() {
        ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
    }

    ui.horizontal_top(|ui| {
        // ── 左列: Fit & Validate ──────────────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if state.multi_objective {
                    render_fit_column_multi(ui, state, obj_names, busy);
                } else {
                    render_fit_column(ui, state, obj_names, busy, constraint_col_names);
                }
            },
        );

        ui.separator();

        // ── 右列: Optimization ───────────────────────────────────
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if state.multi_objective {
                    render_optimize_column_multi(
                        ui,
                        state,
                        busy,
                        has_matching_multi_trained,
                        observed,
                    );
                } else {
                    render_optimize_column(ui, state, busy, has_matching_trained, obj_history);
                }
            },
        );
    });
}

/// 学習済みモデルが現在の UI 選択（目的・モデル種別）と一致するか判定する。
pub(super) fn trained_matches(
    trained: &TrainedSurrogate,
    state: &SurrogateOptState,
    obj_names: &[String],
) -> bool {
    let selected_obj = obj_names
        .get(state.selected_objective)
        .map(|s| s.as_str())
        .unwrap_or("");
    if trained.objective_name != selected_obj {
        return false;
    }
    // Auto モードでは具体的なモデル種別は core が選ぶため、model_kind ではなく
    // 「Auto で学習されたか（model_selection が Some）」で一致を判定する。
    if state.auto_select {
        trained.model_selection.is_some()
    } else {
        trained.model_selection.is_none() && trained.model_kind == state.model
    }
}

/// 多目的学習済みモデル群が現在の UI 選択（モデル・目的集合）と一致するか判定する。
pub(crate) fn multi_trained_matches(
    trained: &[TrainedSurrogate],
    state: &SurrogateOptState,
    obj_names: &[String],
) -> bool {
    if trained.len() != obj_names.len() {
        return false;
    }
    let trained_obj_names: Vec<&str> = trained.iter().map(|t| t.objective_name.as_str()).collect();
    let expected_obj_names: Vec<&str> = obj_names.iter().map(|s| s.as_str()).collect();
    if trained_obj_names != expected_obj_names {
        return false;
    }
    trained.iter().all(|t| t.model_kind == state.model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tunny_core::surrogate_opt::SurrogateModelKind;

    #[test]
    fn optimize_click_requires_matching_trained() {
        let state = SurrogateOptState::default();
        let obj_names = ["obj0".to_string()];
        // trained が None のため has_matching_trained は false。
        let has_matching = state
            .trained
            .as_deref()
            .map(|t| trained_matches(t, &state, &obj_names))
            .unwrap_or(false);
        assert!(!has_matching);
    }

    // ── multi_trained_matches のユニットテスト ────────────────────────

    fn make_dummy_trained(
        obj_name: &str,
        model: SurrogateModelKind,
    ) -> tunny_core::surrogate_opt::TrainedSurrogate {
        // 最低限のフィールドだけ埋めた TrainedSurrogate をフィットして作る。
        let xs: Vec<Vec<f64>> = (0..12)
            .map(|i| vec![i as f64 / 12.0, (i as f64 / 12.0) * 0.5])
            .collect();
        let ys: Vec<f64> = (0..12).map(|i| i as f64).collect();
        let req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix: xs,
            y: ys,
            param_names: vec!["x".to_string(), "y".to_string()],
            objective_name: obj_name.to_string(),
            model,
            auto_select: false,
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: None,
        };
        tunny_core::surrogate_opt::fit_surrogate_with_validation(&req)
            .expect("dummy fit should succeed")
    }

    #[test]
    fn multi_trained_matches_correct() {
        let obj_names = vec!["f0".to_string(), "f1".to_string()];
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_model() {
        let obj_names = vec!["f0".to_string(), "f1".to_string()];
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        // モデルが Ridge に変わった場合
        let state = SurrogateOptState {
            model: SurrogateModelKind::Ridge,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_objectives() {
        let obj_names = vec!["f0".to_string(), "f2".to_string()]; // f2 が違う
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn multi_trained_matches_wrong_length() {
        let obj_names = vec!["f0".to_string()]; // 目的数が 1
        let trained = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let state = SurrogateOptState {
            model: SurrogateModelKind::GpFitc,
            ..Default::default()
        };
        assert!(!multi_trained_matches(&trained, &state, &obj_names));
    }

    #[test]
    fn adopt_compute_state_propagates_multi_trained() {
        use std::sync::Arc;

        let trained_vec = vec![
            make_dummy_trained("f0", SurrogateModelKind::GpFitc),
            make_dummy_trained("f1", SurrogateModelKind::GpFitc),
        ];
        let arc = Arc::new(trained_vec);

        let global = SurrogateOptState {
            fitting: false,
            optimizing: false,
            multi_trained: Some(arc.clone()),
            ..Default::default()
        };

        let mut item = SurrogateOptState {
            fitting: true,
            ..Default::default()
        };
        item.adopt_compute_state(&global);

        assert!(!item.fitting);
        assert!(item.multi_trained.is_some());
        assert_eq!(item.multi_trained.as_ref().unwrap().len(), 2);
    }
}
