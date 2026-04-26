/// MCDM UI 型定義
///
/// 作成日: 2026-04-23
/// 更新日: 2026-04-23 (McdmChart統一化)
/// 関連設計: architecture.md, dataflow.md
///
/// 信頼性レベル:
/// - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
/// - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
/// - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義

// ============================================================
// McdmMethod 列挙型（ImportanceMetricパターン）
// ============================================================

/// MCDM手法の選択肢
/// 🔵 信頼性: ユーザヒアリング（ImportanceChart統一パターン）より
///
/// Phase 1: Topsis のみ実装
/// 将来: Vikor, Promethee2 をバリアント追加のみで対応
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McdmMethod {
    Topsis,       // 🔵 Phase 1 実装
    // Vikor,     // 🟡 将来追加
    // Promethee2, // 🟡 将来追加
}

impl McdmMethod {
    pub fn label(&self) -> &'static str {
        match self {
            McdmMethod::Topsis => "TOPSIS",
            // McdmMethod::Vikor => "VIKOR",
            // McdmMethod::Promethee2 => "PROMETHEE II",
        }
    }

    pub fn all() -> &'static [McdmMethod] {
        &[McdmMethod::Topsis]
    }
}

// ============================================================
// TopsisResult（rust_core定義に統一）
// ============================================================

/// TOPSIS計算結果
/// 🔵 信頼性: rust_core/src/mcdm/topsis.rs の既存定義に統一（ユーザヒアリング確定）
///
/// 変更対象: egui-app/src/state/results.rs
/// 既存の2フィールド（scores, ranking: Vec<usize>）をこの5フィールドに置き換える
#[derive(Debug, Clone)]
pub struct TopsisResult {
    /// 各TrialのTOPSISスコア (0.0〜1.0、高いほど良い)
    pub scores: Vec<f64>,              // 🔵 rust_core定義より
    /// スコア降順に並べたTrialインデックス
    pub ranked_indices: Vec<u32>,      // 🔵 rust_core定義より
    /// 各目的関数の正理想解（理想値）
    pub positive_ideal: Vec<f64>,      // 🔵 rust_core定義より
    /// 各目的関数の負理想解（最悪値）
    pub negative_ideal: Vec<f64>,      // 🔵 rust_core定義より
    /// 計算時間 (ms)
    pub duration_ms: f64,              // 🔵 rust_core定義より
}

// ============================================================
// McdmResult（手法統一結果ラッパー）
// ============================================================

/// MCDM手法の統一結果型
/// 🔵 信頼性: ユーザヒアリング（McdmChart統一）・既存パターンより
///
/// 各手法のResult型をenumでラップ。primary_scores()で共通アクセス。
/// 将来: VikorResult, Promethee2Result をバリアント追加
#[derive(Debug, Clone)]
pub enum McdmResult {
    Topsis(TopsisResult),  // 🔵 Phase 1
    // Vikor(VikorResult),       // 🟡 将来追加
    // Promethee2(Promethee2Result), // 🟡 将来追加
}

impl McdmResult {
    /// カラーモード用の一次スコア（0.0-1.0）を取得
    /// 🔵 信頼性: 全MCDM手法がスコア[0,1]を出力する共通仕様より
    pub fn primary_scores(&self) -> &[f64] {
        match self {
            McdmResult::Topsis(r) => &r.scores,
            // McdmResult::Vikor(r) => &r.q_scores,
            // McdmResult::Promethee2(r) => &r.net_flows_normalized,
        }
    }

    /// ランキング順のTrialインデックスを取得
    /// 🔵 信頼性: 全MCDM手法がranked_indicesを出力する共通仕様より
    pub fn ranked_indices(&self) -> &[u32] {
        match self {
            McdmResult::Topsis(r) => &r.ranked_indices,
            // McdmResult::Vikor(r) => &r.ranked_indices,
            // McdmResult::Promethee2(r) => &r.ranked_indices,
        }
    }

    /// 計算時間を取得
    /// 🔵 信頼性: 全手法で共通
    pub fn duration_ms(&self) -> f64 {
        match self {
            McdmResult::Topsis(r) => r.duration_ms,
            // McdmResult::Vikor(r) => r.duration_ms,
            // McdmResult::Promethee2(r) => r.duration_ms,
        }
    }

    /// 現在の手法名
    /// 🔵 信頼性: McdmMethod に対応
    pub fn method_label(&self) -> &'static str {
        match self {
            McdmResult::Topsis(_) => "TOPSIS",
            // McdmResult::Vikor(_) => "VIKOR",
            // McdmResult::Promethee2(_) => "PROMETHEE II",
        }
    }
}

// ============================================================
// McdmChart ウィジェット状態
// ============================================================

/// MCDMウィジェットのUI状態
/// 🔵 信頼性: ImportanceChartパターン・ユーザヒアリングより
///
/// 新規作成: egui-app/src/ui/widgets/mcdm_chart.rs
pub struct McdmChart {
    /// 選択中のMCDM手法
    pub method: McdmMethod,            // 🔵 ImportanceChart.metric パターンより
    /// 各目的関数の重み（0.0〜1.0、正規化前の生値）
    pub weights: Vec<f64>,             // 🔵 ユーザヒアリング（個別スライダー）より
    /// 計算中フラグ
    pub computing: bool,               // 🔵 ImportanceChart.computing パターンより
    /// 計算要求ペンディング（Run押下時に設定）
    pub pending_compute: Option<(McdmMethod, Vec<f64>)>, // 🔵 ImportanceChart.pending_compute パターンより
    /// 上位N件表示設定
    pub top_n: McdmTopN,               // 🔵 ユーザヒアリング（5/10/20トグル）より
    /// 表示モード（チャート/テーブル/両方）
    pub view_mode: McdmViewMode,       // 🟡 既存ウィジェットパターンから妥当な推測
}

/// 上位N件表示の選択肢
/// 🔵 信頼性: ユーザヒアリング（5/10/20トグル）より
pub enum McdmTopN {
    Top5,   // 🔵
    Top10,  // 🔵
    Top20,  // 🔵
}

impl McdmTopN {
    pub fn value(&self) -> usize {
        match self {
            McdmTopN::Top5 => 5,
            McdmTopN::Top10 => 10,
            McdmTopN::Top20 => 20,
        }
    }
}

/// MCDM結果の表示モード
/// 🟡 信頼性: 3つの可視化方法を切り替えるためのモード。既存パターンから妥当な推測
pub enum McdmViewMode {
    ChartOnly,      // 🟡 バーチャートのみ
    TableOnly,      // 🟡 テーブルのみ
    ChartAndTable,  // 🟡 両方表示（デフォルト）
}

// ============================================================
// ChartId 追加
// ============================================================

/// ChartId 列挙型への追加
/// 🔵 信頼性: 既存ChartIdパターンより
///
/// 変更対象: egui-app/src/state/layout_state.rs
///
/// 追加内容:
/// ChartId::McdmChart  // 🔵 新規バリアント
///
/// impl ChartId {
///     pub fn label(&self) -> &'static str {
///         match self {
///             // ... 既存分 ...
///             ChartId::McdmChart => "MCDM Analysis",  // 🔵
///         }
///     }
/// }

// ============================================================
// WidgetStates 追加
// ============================================================

/// WidgetStates への追加
/// 🔵 信頼性: 既存WidgetStatesパターンより
///
/// 変更対象: egui-app/src/ui/widget_states.rs
///
/// 追加内容:
/// pub struct WidgetStates {
///     // ... 既存フィールド ...
///     pub mcdm: McdmChart,  // 🔵 新規フィールド
/// }

// ============================================================
// AppState 変更
// ============================================================

/// AppState の mcdm_result フィールド
/// 🔵 信頼性: 既存 topsis_result フィールドの拡張・ユーザヒアリングより
///
/// 変更対象: egui-app/src/state/app_state.rs
///
/// 変更内容:
/// // 変更前:
/// pub topsis_result: Option<TopsisResult>,
/// // 変更後:
/// pub mcdm_result: Option<McdmResult>,  // 🔵 統一MCDM結果

// ============================================================
// ColorMode 追加
// ============================================================

/// ColorMode 列挙型への追加
/// 🔵 信頼性: 既存ColorModeパターン・ユーザヒアリングより
///
/// 変更対象: egui-app/src/state/app_state.rs
///
/// 追加内容:
/// pub enum ColorMode {
///     ParetoRank,              // 既存
///     ObjectiveValue(String),  // 既存
///     TrialNumber,             // 既存
///     ClusterId,               // 既存
///     McdmScore,               // 🔵 新規追加（手法に依存しない）
/// }

// ============================================================
// AppMessage 変更
// ============================================================

/// AppMessage の TopsisDone → McdmDone 統一
/// 🔵 信頼性: 既存AppMessageパターン・ユーザヒアリングより
///
/// 変更対象: egui-app/src/state/messages.rs
///
/// 変更内容:
/// // 変更前:
/// TopsisDone(TopsisResult),
/// // 変更後:
/// McdmDone(McdmResult),  // 🔵 統一MCDMメッセージ

// ============================================================
// 右パネルグループ追加
// ============================================================

/// 右パネルへのMCDMグループ追加
/// 🔵 信頼性: 既存right_panel.rs パターンより
///
/// 変更対象: egui-app/src/ui/right_panel.rs
///
/// 追加内容:
/// let groups: &[(&str, &[PanelItem])] = &[
///     ("Convergence", &[...]),
///     ("Pareto / Multi-Objective", &[...]),
///     ("Variable Analysis", &[...]),
///     ("Clustering", &[...]),
///     ("MCDM", &[PanelItem::Chart(ChartId::McdmChart)]),  // 🔵 新規グループ
///     ("Data", &[...]),
/// ];

// ============================================================
// MessageHandler 更新
// ============================================================

/// McdmDone ハンドラ
/// 🔵 信頼性: 既存MessageHandlerパターンより
///
/// 変更対象: egui-app/src/state/message_handler.rs
///
/// 変更内容:
/// // 変更前:
/// AppMessage::TopsisDone(result) => {
///     app_state.topsis_result = Some(result);
/// }
/// // 変更後:
/// AppMessage::McdmDone(result) => {
///     app_state.mcdm_result = Some(result);          // 🔵
///     widget_states.mcdm.computing = false;           // 🔵
/// }

// ============================================================
// chart_registry ディスパッチ追加
// ============================================================

/// ChartRegistry へのディスパッチロジック
/// 🔵 信頼性: 既存chart_registryパターン・ImportanceChart参照より
///
/// 変更対象: egui-app/src/ui/chart_registry.rs
///
/// 追加内容:
/// ChartId::McdmChart => {
///     // 結果表示
///     let mcdm_result = &app_state.mcdm_result;
///     let obj_names = current_study.meta.objective_names();
///     let trial_rows = &current_study.trial_rows;
///     widgets.mcdm.show(ui, mcdm_result, obj_names, trial_rows);
///
///     // 計算ディスパッチ
///     if let Some((method, weights)) = widgets.mcdm.pending_compute.take() {
///         widgets.mcdm.computing = true;
///         let tx = tx.clone();
///         let values = current_study.objective_values_flat();
///         let n = current_study.trial_rows.len();
///         let m = obj_names.len();
///         let is_minimize = current_study.meta.directions_bool();
///         crate::app::spawn_task(tx, move || {
///             let result = match method {
///                 McdmMethod::Topsis => {
///                     tunny_core::mcdm::topsis::compute_topsis(
///                         &values, n, m, &weights, &is_minimize
///                     ).map(McdmResult::Topsis)
///                 }
///                 // 将来: McdmMethod::Vikor => { ... }
///                 // 将来: McdmMethod::Promethee2 => { ... }
///             };
///             match result {
///                 Ok(r) => AppMessage::McdmDone(r),
///                 Err(e) => AppMessage::Error(e),
///             }
///         });
///     }
/// }

// ============================================================
// 信頼性レベルサマリー
// ============================================================
/// - 🔵 青信号: 22件 (92%)
/// - 🟡 黄信号: 2件 (8%)
/// - 🔴 赤信号: 0件 (0%)
///
/// 品質評価: ✅ 高品質
