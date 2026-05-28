// ============================================================
// chart-csv-export 型定義（設計文書用）
//
// 作成日: 2026-05-28
// 関連設計: architecture.md
//
// 信頼性レベル:
// - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
// - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
// - 🔴 赤信号: 推測による型定義
// ============================================================

// ============================================================
// CellToolbarAction 拡張（egui-app/src/ui/grid_canvas.rs）
// ============================================================

/// セルツールバーで発生するアクション。
/// 🔵 信頼性: 既存 CellToolbarAction に SaveAsCsv を追加 — ユーザヒアリングQ1より
pub enum CellToolbarAction {
    None,
    Close,
    Help(PanelItem),
    SaveAsPng(PanelItem),
    SaveAsCsv(PanelItem), // 🔵 新規追加 - REQ-003より
}

// ============================================================
// io/csv_export.rs 公開インターフェース
// ============================================================

/// チャートIDに対応するCSV文字列を生成する。
/// データが存在しない場合や Surface Plot のようにスキップ対象の場合は None を返す。
///
/// 🔵 信頼性: REQ-010・ユーザヒアリング（io層一括dispatch）より
///
/// # 引数
/// - `chart_id`: 出力対象チャートのID
/// - `app_state`: アプリケーション状態（試行データ・分析結果）への参照
/// - `widgets`: UIウィジェット状態（PDP・Importance等の計算キャッシュ）への参照
///
/// # 戻り値
/// - `Some(String)`: CSVデータ文字列（先頭行はヘッダー）
/// - `None`: データ未準備、またはスキップ対象チャート
pub fn build_chart_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String>;

/// チャートのCSVデータが現在利用可能かどうかを返す。
/// ボタングレーアウト判定に使用する。
///
/// 🔵 信頼性: REQ-201・ユーザヒアリングQ4より
///
/// # 引数
/// - `chart_id`: 判定対象チャートのID
/// - `app_state`: アプリケーション状態への参照
/// - `widgets`: UIウィジェット状態への参照
///
/// # 戻り値
/// - `true`: データあり、ボタンをアクティブに
/// - `false`: データなし/計算中/スキップ対象、ボタンをグレーアウト
pub fn has_csv_data(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> bool;

/// チャートIDからデフォルトのCSVファイル名を生成する。
///
/// 🟡 信頼性: REQ-012より（スネークケース変換は妥当な推測）
///
/// # 例
/// - ChartId::OptimizationHistory → "optimization_history.csv"
/// - ChartId::ImportanceChart → "importance_chart.csv"
pub fn csv_export_filename(chart_id: &ChartId) -> String;

// ============================================================
// io/export.rs 追加関数
// ============================================================

/// デフォルトファイル名を指定してCSVをファイルダイアログ経由で保存する。
/// ダイアログキャンセル時は Ok(()) を返す。
///
/// 🟡 信頼性: 既存 save_csv_to_file() の自然な拡張
pub fn save_csv_to_file_named(csv: &str, default_name: &str) -> Result<(), String>;

// ============================================================
// 内部: チャート別 CSV 生成関数シグネチャ
// ============================================================

/// OptimizationHistory: 試行インデックス + 目的値 + 累積ベスト値
/// 🔵 信頼性: REQ-020・optimization_history.rs の compute_best_values() より
///
/// 列: trial_index, objective_value, best_value
fn build_optimization_history_csv(
    trial_rows: &[TrialRow],
    obj_idx: usize,
    is_minimize: bool,
) -> String;

/// HvHistory: 試行インデックス + ハイパーボリューム値
/// 🔵 信頼性: REQ-021・HvHistory 型定義より
///
/// 列: trial_index, hypervolume
fn build_hv_history_csv(hv: &HvHistory) -> String;

/// ImportanceChart: 変数名 + 重要度スコア + メソッド名
/// 🔵 信頼性: REQ-022・SensitivityResult / SobolResult 型定義より
///
/// 列: variable, importance_score, method
fn build_importance_csv(
    result: &SensitivityResult,
    obj_idx: usize,
    metric: &ImportanceMetric,
) -> Option<String>;

/// PdpChart: 変数値 + 予測目的値 + 信頼区間
/// 🔵 信頼性: REQ-023・PdpResult 型定義より
///
/// 列: variable, variable_value, predicted_objective, lower_ci, upper_ci
fn build_pdp_csv(
    pdp_cache: &HashMap<String, PdpResult>,
    selected_param: &str,
    selected_obj: &str,
    model_type: &str,
) -> Option<String>;

/// ParallelCoordinates / ScatterMatrix: 全試行データ
/// 🔵 信頼性: REQ-025・REQ-026・TrialRow 型定義より
///
/// 列: trial_id, trial_number, {param_names...}, {objective_names...}
fn build_trial_based_csv(
    trial_rows: &[TrialRow],
    param_names: &[String],
    objective_names: &[String],
) -> String;

/// ClusterScatter: クラスタID付き全試行データ
/// 🔵 信頼性: REQ-028・ClusterResult 型定義より
///
/// 列: trial_id, trial_number, {param_names...}, {objective_names...}, cluster_id
fn build_cluster_csv(
    trial_rows: &[TrialRow],
    param_names: &[String],
    objective_names: &[String],
    cluster_result: Option<&ClusterResult>,
) -> String;

/// SensitivityHeatmap: 変数 × 目的関数の感度行列
/// 🔵 信頼性: REQ-027・SensitivityResult 型定義より
///
/// 列: variable, {objective_names...}
/// 各セルは Spearman 相関係数
fn build_sensitivity_csv(result: &SensitivityResult) -> String;

/// ParetoScatter2D / 3D: パレートフロント試行のみ
/// 🔵 信頼性: REQ-029・REQ-030・pareto_indices より
///
/// 列: trial_id, trial_number, {objective_names...}, pareto_rank
fn build_pareto_csv(
    trial_rows: &[TrialRow],
    pareto_indices: &[u32],
    objective_names: &[String],
) -> String;

/// McdmRankChart: ランキング + スコア
/// 🔵 信頼性: REQ-031・McdmResult 型定義より
///
/// 列: trial_id, rank, score, method
fn build_mcdm_rank_csv(result: &McdmResult) -> String;

/// McdmTable: バリアント別詳細スコア
/// 🔵 信頼性: REQ-032・McdmResult バリアント定義より
///
/// TOPSIS: trial_id, rank, topsis_score
/// VIKOR:  trial_id, rank, s_value, r_value, q_value
/// PROMETHEE I:  trial_id, rank, phi_plus, phi_minus
/// PROMETHEE II: trial_id, rank, phi_net
fn build_mcdm_table_csv(result: &McdmResult) -> String;

/// AhpRankChart: AHP スコア + ランキング
/// 🔵 信頼性: REQ-034・AhpResult 型定義より
///
/// 列: trial_id, rank, ahp_score
fn build_ahp_rank_csv(result: &AhpResult) -> String;

/// SliceChart: 選択変数 × 選択目的関数の散布データ
/// 🔵 信頼性: REQ-036・SliceChart 型定義より
///
/// 列: trial_id, {param_name}, {objective_name}, is_pareto
fn build_slice_csv(
    trial_rows: &[TrialRow],
    param_names: &[String],
    obj_names: &[String],
    selected_param_idx: usize,
    selected_obj_idx: usize,
) -> Option<String>;

// ============================================================
// 信頼性レベルサマリー
// ============================================================
// - 🔵 青信号: 14件 (87%)
// - 🟡 黄信号: 2件 (13%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: 高品質
