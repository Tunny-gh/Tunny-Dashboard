//! egui UI リファクタリング 型定義・関数シグネチャ
//!
//! 作成日: 2026-05-08
//! 関連設計: architecture.md
//!
//! 信頼性レベル:
//! - 🔵 青信号: EARS要件定義書・設計文書・ユーザヒアリングを参考にした確実な定義
//! - 🟡 黄信号: EARS要件定義書・設計文書・ユーザヒアリングから妥当な推測による定義
//! - 🔴 赤信号: EARS要件定義書・設計文書・ユーザヒアリングにない推測による定義

// ============================================================
// REQ-001: ui/render_chart.rs (新規)
// ============================================================

/// チャートウィジェットを描画する。
/// spawn_task・tx 引数を持たない。描画専用。
///
/// 🔵 信頼性: REQ-001・ユーザヒアリング Q4 より
pub(crate) fn render_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
) {
    // chart_registry.rs から render_chart の中身をそのまま移動
}

// ============================================================
// REQ-001: ui/poll_chart.rs (新規)
// ============================================================

/// バックグラウンド計算タスクを dispatch する。
/// egui::Ui 引数を持たない。ディスパッチ専用。
///
/// 🔵 信頼性: REQ-001・ユーザヒアリング Q4 より
pub(crate) fn poll_chart_work(
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &std::sync::mpsc::SyncSender<AppMessage>,
) {
    // chart_registry.rs から poll_chart_work の中身をそのまま移動
}

// ============================================================
// REQ-001: ui/chart_registry.rs (変更後 - 薄いラッパーのみ)
// ============================================================

/// タイトルと区切り線付きでチャートを描画する（シグネチャ不変）。
///
/// 🔵 信頼性: REQ-001・NFR-002（外部 API 互換性）より
pub fn show_cell_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &std::sync::mpsc::SyncSender<AppMessage>,
) {
    ui.label(egui::RichText::new(chart_id.label()).strong());
    ui.separator();
    show_chart(ui, app_state, widgets, chart_id, tx);
}

/// ChartId に対応するチャートを描画し dispatch する（シグネチャ不変）。
///
/// 🔵 信頼性: REQ-001・NFR-002（外部 API 互換性）より
pub fn show_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &std::sync::mpsc::SyncSender<AppMessage>,
) {
    crate::ui::render_chart::render_chart(ui, app_state, widgets, chart_id);
    crate::ui::poll_chart::poll_chart_work(app_state, widgets, chart_id, tx);
}

// ============================================================
// REQ-002: rust_core/src/convergence.rs (新規)
// ============================================================

/// 直近 last_n 試行で best 値が改善された割合を計算する。
///
/// - `history`: (trial_id, cumulative_best_value) のタプルスライス
/// - `last_n`: 最近 N 件を対象とする
/// - 戻り値: 改善割合 [0.0, 1.0]
///
/// 🔵 信頼性: REQ-002・既存 left_panel.rs::compute_improvement_rate より
pub fn compute_improvement_rate(history: &[(u32, f64)], last_n: usize) -> f64 {
    // 既存の left_panel.rs::compute_improvement_rate の実装をそのまま移植
    let window: Vec<_> = history.iter().rev().take(last_n).collect();
    if window.len() < 2 {
        return 0.0;
    }
    let mut best_so_far = f64::INFINITY;
    let mut improved_count = 0usize;
    for &(_, val) in window.iter().rev() {
        if *val < best_so_far {
            best_so_far = *val;
            improved_count += 1;
        }
    }
    (improved_count as f64) / (window.len() as f64)
}

/// 各試行時点でのベスト値の推移を計算する。
///
/// - `trial_ids`: 試行 ID スライス（egui-app 側で TrialRow から抽出）
/// - `objective_values`: 対応する目的関数値スライス（同一長）
/// - `is_minimize`: true = 最小化問題
/// - 戻り値: (trial_id, cumulative_best_value) のベクター
///
/// **設計ノート**: egui-app 側の `TrialRow` を直接受け取らず、プリミティブ型スライスで
/// rust_core と egui-app 間の型依存を排除する。
///
/// 🔵 信頼性: REQ-002・ユーザヒアリング Q3 の設計方針より
pub fn build_best_trial_history(
    trial_ids: &[u32],
    objective_values: &[f64],
    is_minimize: bool,
) -> Vec<(u32, f64)> {
    // left_panel.rs::build_best_trial_history を TrialRow 依存なしで移植
    let mut history = Vec::with_capacity(trial_ids.len());
    let mut best = if is_minimize { f64::INFINITY } else { f64::NEG_INFINITY };
    for (&id, &val) in trial_ids.iter().zip(objective_values.iter()) {
        let improved = if is_minimize { val < best } else { val > best };
        if improved {
            best = val;
        }
        history.push((id, best));
    }
    history
}

// ============================================================
// REQ-002: rust_core/src/multi_objective/weights.rs (新規)
// ============================================================

/// 重みベクターの合計が 1.0 になるよう正規化する。
/// 合計が 0 に近い場合は均等分割する。
///
/// 🔵 信頼性: REQ-002・ユーザヒアリング Q3 (multi_objective 配置)・
///            既存 left_panel.rs::normalize_weights より
pub fn normalize_weights(weights: &mut [f64]) {
    let sum: f64 = weights.iter().sum();
    if sum > f64::EPSILON {
        for w in weights.iter_mut() {
            *w /= sum;
        }
    } else if !weights.is_empty() {
        let uniform = 1.0 / weights.len() as f64;
        for w in weights.iter_mut() {
            *w = uniform;
        }
    }
}

// ============================================================
// REQ-002: egui-app 側の呼び出し例
// ============================================================
//
// convergence_card.rs 内での呼び出し:
//
// let trial_ids: Vec<u32> = trials.iter().map(|t| t.trial_id).collect();
// let obj_values: Vec<f64> = trials
//     .iter()
//     .filter_map(|t| t.objectives.get(objective_idx).copied())
//     .collect();
// let history = tunny_core::convergence::build_best_trial_history(
//     &trial_ids, &obj_values, is_minimize,
// );
// let rate = tunny_core::convergence::compute_improvement_rate(&history, 100);
//
// tradeoff_navigator.rs 内での呼び出し:
//
// tunny_core::multi_objective::weights::normalize_weights(&mut app_state.tradeoff_weights);

// ============================================================
// REQ-003: egui-app/src/io/html_report.rs (変更)
// ============================================================

/// StudyContext と選択試行インデックスから HTML レポートを構築して非同期送信する。
///
/// `app.rs` が `HtmlReportSnapshot` / `HtmlTrialRow` / `TrialStatistics` を
/// 直接扱わなくてよくなる。
///
/// 🔵 信頼性: REQ-003・ユーザヒアリング Q5・app.rs:77-108 分析より
pub fn build_and_send_report(
    ctx: &crate::state::types::StudyContext,
    selected_indices: &[u32],
    tx: std::sync::mpsc::SyncSender<crate::state::messages::AppMessage>,
) {
    let snap = HtmlReportSnapshot {
        study_name: ctx.meta.name.clone(),
        objective_names: ctx.meta.objective_names.clone(),
        param_names: ctx.meta.param_names.clone(),
        total_trials: ctx.trial_rows.len(),
        pareto_count: ctx.pareto_indices.len(),
        selected_trials: selected_indices
            .iter()
            .filter_map(|&id| ctx.trial_rows.iter().find(|r| r.trial_id == id))
            .map(|r| HtmlTrialRow {
                trial_id: r.trial_id,
                trial_number: r.trial_number,
                params: r.params.clone(),
                objectives: r.objectives.clone(),
                pareto_rank: r.pareto_rank,
            })
            .collect(),
        statistics: TrialStatistics {
            objective_means: vec![0.0; ctx.meta.objective_names.len()],
            objective_variances: vec![0.0; ctx.meta.objective_names.len()],
            pareto_count: ctx.pareto_indices.len(),
        },
    };
    generate_html_report_async(snap, tx);
}

// ============================================================
// REQ-004: egui-app/src/ui/widgets/tradeoff_navigator.rs (新規)
// ============================================================

/// Trade-off Navigator セクション（多目的 Study 時のみ表示）。
/// left_panel.rs から移動。シグネチャは変更なし（NFR-002）。
///
/// 🔵 信頼性: REQ-004・NFR-002・ユーザヒアリング Q6 より
pub fn show_tradeoff_navigator(
    ui: &mut egui::Ui,
    app_state: &mut crate::state::app_state::AppState,
    objective_names: &[String],
    is_minimize: &[bool],
    tx: &std::sync::mpsc::SyncSender<crate::state::messages::AppMessage>,
) {
    // left_panel.rs::show_tradeoff_navigator の実装をそのまま移動
    // normalize_weights の呼び出しを tunny_core 版に変更:
    // tunny_core::multi_objective::weights::normalize_weights(&mut app_state.tradeoff_weights);
}

// ============================================================
// REQ-004: egui-app/src/ui/widgets/convergence_card.rs (新規)
// ============================================================

/// 収束診断カード（単目的 Study 時のみ表示）。
/// left_panel.rs から移動。シグネチャは変更なし（NFR-002）。
///
/// 🔵 信頼性: REQ-004・NFR-002・ユーザヒアリング Q6 より
pub fn show_convergence_card(
    ui: &mut egui::Ui,
    app_state: &crate::state::app_state::AppState,
) {
    // left_panel.rs::show_convergence_card の実装をそのまま移動
    // compute_improvement_rate の呼び出しを tunny_core 版に変更:
    // let rate = tunny_core::convergence::compute_improvement_rate(&history, 100);
}

// ============================================================
// rust_core/src/lib.rs への追加エントリ
// ============================================================
//
// pub mod convergence;  // 新規追加
//
// multi_objective/mod.rs への追加:
// pub mod weights;  // 新規追加

// ============================================================
// 信頼性レベルサマリー
// ============================================================
//
// - 🔵 青信号: 10件 (100%)
// - 🟡 黄信号: 0件 (0%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
// 全シグネチャは既存コード・要件定義から機械的に導出可能
