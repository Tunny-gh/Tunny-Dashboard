//! `StudyReport` の構造体ツリー（言語非依存の構造化ファクト）。
//!
//! ここに定義する型はすべて `serde::Serialize` を導出し、JSON へそのまま出力できる。
//! 文章化（en / ja）はレンダラ（`markdown` / `html`）のテンプレートが担当し、
//! モデル自体は言語に依存しない。数値は f64 のまま保持し、丸め・整形は
//! レンダラの共通フォーマッタ（[`crate::report::format_number`]）で行う。
//!
//! 決定論性のため、辞書的な集合は [`std::collections::BTreeMap`] または
//! ソート済み `Vec` で保持し、`HashMap` の反復順に依存する出力を作らない。

use std::collections::BTreeMap;

/// スキーマのバージョン。破壊的変更のたびに増やす。
pub const SCHEMA_VERSION: u32 = 1;

/// 目的の最適化方向（`serde` 出力用の言語非依存表現）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Direction {
    /// 最小化。
    Minimize,
    /// 最大化。
    Maximize,
}

impl Direction {
    /// 最小化方向か。
    pub fn is_minimize(self) -> bool {
        matches!(self, Direction::Minimize)
    }
}

/// レポートのルート構造体。
#[derive(Debug, Clone, serde::Serialize)]
pub struct StudyReport {
    /// スキーマバージョン（[`SCHEMA_VERSION`]）。
    pub schema_version: u32,
    /// ソース情報（ストレージ表示名・生成日時）。
    pub source: ReportSourceInfo,
    /// スタディ概要。
    pub overview: Overview,
    /// Key Findings（まとめ）。決定論的に自動生成される。
    pub key_findings: Vec<KeyFinding>,
    /// 最適化結果（単目的 / 多目的）。
    pub outcome: Outcome,
    /// 収束セクション（best-so-far / HV 推移）。
    pub convergence: ConvergenceSection,
    /// パラメータ重要度（計算不能なら `None`）。
    pub importance: Option<ImportanceSection>,
    /// 目的値の分布統計（目的ごと）。
    pub objective_stats: Vec<ObjectiveStats>,
    /// パラメータ×目的の相関（計算不能なら `None`）。
    pub correlations: Option<CorrelationSection>,
    /// 多目的の意思決定分析（MCDM）。単目的なら `None`。
    pub mcdm: Option<McdmSection>,
    /// 実行時情報（extras がある場合のみ）。
    pub execution: Option<ExecutionSection>,
    /// 再現情報。
    pub reproduction: Reproduction,
}

/// ソース情報（`ReportSource` のスナップショット）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReportSourceInfo {
    /// ストレージ表示名（RDB URL の場合はマスク済み）。
    pub storage_display: String,
    /// 生成日時（unix 秒）。`None` なら日時欄を省略する。
    pub generated_at_unix: Option<i64>,
}

/// スタディ概要。
#[derive(Debug, Clone, serde::Serialize)]
pub struct Overview {
    /// スタディ名。
    pub name: String,
    /// 目的ごとの最適化方向。
    pub directions: Vec<Direction>,
    /// 目的名。
    pub objective_names: Vec<String>,
    /// パラメータ名。
    pub param_names: Vec<String>,
    /// user_attr 名。
    pub user_attr_names: Vec<String>,
    /// state ラベルごとの trial 数（決定論のため BTreeMap）。
    pub state_counts: BTreeMap<String, usize>,
    /// COMPLETE trial 数（解析対象の行数）。
    pub complete_trials: usize,
    /// 全 trial 数（meta 由来）。
    pub total_trials: usize,
    /// 実測所要時間（秒）。extras の日時から算出。無ければ `None`。
    pub wall_clock_seconds: Option<f64>,
    /// パラメータの宣言レンジ `(name, low, high)`。名前昇順。
    pub param_bounds: Vec<(String, f64, f64)>,
    /// 制約が定義されているか。
    pub has_constraints: bool,
}

/// Key Finding（まとめの1項目）。
///
/// `kind` は固定 enum で、レンダラが網羅 `match` して文章化する。`metrics` /
/// `labels` はテンプレートに埋める数値・文字列。決定論のため BTreeMap を使う。
#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyFinding {
    /// 種類。
    pub kind: FindingKind,
    /// テンプレートに埋める数値。
    pub metrics: BTreeMap<String, f64>,
    /// テンプレートに埋める文字列（param 名等）。
    pub labels: BTreeMap<String, String>,
}

/// Key Finding の種類（固定 enum）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum FindingKind {
    /// 単目的の最良値・trial 番号・発見時点。
    BestSingle,
    /// パレート前面のサイズと各目的の極値。
    ParetoSummary,
    /// 収束判定（Converged / StillImproving / Insufficient）。
    ConvergenceStatus,
    /// 上位パラメータ（method 名付き）。
    TopImportance,
    /// 目的間のトレードオフ（最も負の Spearman ペア）。
    TradeOff,
    /// 制約充足率と最良 feasible trial。
    Feasibility,
    /// 枝刈り効率（prune 率と中央値 step）。
    PruningEfficiency,
    /// データ品質（FAIL / NaN 目的値の注意喚起）。
    DataQuality,
}

/// 収束判定の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ConvergenceStatus {
    /// 収束済み（後半20%で best 更新なし）。
    Converged,
    /// なお改善中（後半20%で best 更新あり）。
    StillImproving,
    /// データ不足（COMPLETE < 10）。
    Insufficient,
}

/// 単一 trial の要約。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TrialSummary {
    /// Study 内 0 始まりの trial.number。
    pub trial_number: u32,
    /// 目的値（目的順）。
    pub objectives: Vec<f64>,
    /// パラメータ `(name, value)`（meta のパラメータ順）。
    pub params: Vec<(String, ParamValue)>,
    /// 制約違反量（制約ありスタディのみ。全制約値の合計）。
    pub constraint_violation: Option<f64>,
    /// user_attr `(name, value)`（名前昇順）。
    pub user_attrs: Vec<(String, String)>,
}

/// パラメータ値（数値 / カテゴリ）。
#[derive(Debug, Clone, serde::Serialize)]
pub enum ParamValue {
    /// 数値パラメータ。
    Num(f64),
    /// カテゴリカルパラメータ（表示ラベル）。
    Cat(String),
}

/// 最適化結果。
#[derive(Debug, Clone, serde::Serialize)]
pub enum Outcome {
    /// 単目的。
    SingleObj {
        /// 最良 trial（COMPLETE が無ければ `None`）。
        best_trial: Option<TrialSummary>,
        /// 上位 trial（最良順、`top_n` 件）。
        top_n: Vec<TrialSummary>,
    },
    /// 多目的。
    MultiObj {
        /// パレート前面のサイズ。
        pareto_size: usize,
        /// COMPLETE 数。
        complete_count: usize,
        /// 目的数。
        objective_count: usize,
        /// 目的ごとの極値。
        per_objective_extremes: Vec<ObjectiveExtreme>,
        /// パレート前面の trial 表（TOPSIS 順、`top_n*2` で cap）。
        pareto_table: Vec<TrialSummary>,
        /// 散布図点（全 COMPLETE + front 判定、先頭2目的軸）。
        scatter: Vec<ParetoPoint>,
        /// 散布図の軸に用いた目的インデックス `(x, y)`。
        scatter_axes: (usize, usize),
    },
}

/// 目的ごとの極値。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectiveExtreme {
    /// 目的インデックス。
    pub objective_index: usize,
    /// 目的名。
    pub objective_name: String,
    /// 方向。
    pub direction: Direction,
    /// 最良値。
    pub best_value: f64,
    /// 最良値を達成した trial.number。
    pub best_trial_number: u32,
    /// 最悪値。
    pub worst_value: f64,
}

/// 散布図の1点（パレート前面判定付き）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ParetoPoint {
    /// trial.number。
    pub trial_number: u32,
    /// x 軸値（先頭目的）。
    pub x: f64,
    /// y 軸値（2番目の目的）。
    pub y: f64,
    /// パレート前面上の点か。
    pub on_front: bool,
}

/// 収束セクション。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConvergenceSection {
    /// 系列が表す指標。
    pub metric: ConvergenceMetric,
    /// 系列（trial.number, 値）。≤500 点に間引き済み。
    pub series: Vec<ConvergencePoint>,
    /// best が発見された trial.number（データ不足なら `None`）。
    pub found_at_trial_number: Option<u32>,
    /// 直近20%の試行で best が更新されたか。
    pub improved_in_last_20pct: bool,
    /// 収束判定。
    pub status: ConvergenceStatus,
}

/// 収束系列の指標種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ConvergenceMetric {
    /// 単目的の best-so-far。
    BestSoFar,
    /// 多目的の Hypervolume 推移。
    Hypervolume,
}

/// 収束系列の1点。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConvergencePoint {
    /// trial.number。
    pub trial_number: u32,
    /// 指標値。
    pub value: f64,
}

/// パラメータ重要度セクション。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportanceSection {
    /// 重要度の算出手法名（例: `"spearman_abs"`）。
    pub method: String,
    /// 重要度を評価した対象の目的名。
    pub objective_name: String,
    /// `(param, score)` を降順（score 大きい順）に並べたもの。
    pub scores: Vec<(String, f64)>,
}

/// 目的値の分布統計。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjectiveStats {
    /// 目的名。
    pub name: String,
    /// 方向。
    pub direction: Direction,
    /// 有限値の件数。
    pub n: usize,
    /// 平均。
    pub mean: f64,
    /// 母標準偏差。
    pub std: f64,
    /// 最小。
    pub min: f64,
    /// 第1四分位。
    pub q1: f64,
    /// 中央値。
    pub median: f64,
    /// 第3四分位。
    pub q3: f64,
    /// 最大。
    pub max: f64,
    /// ヒストグラム（≤20 ビン）。有限値が無ければ `None`。
    pub histogram: Option<HistogramData>,
}

/// ヒストグラムのビン境界と度数。
#[derive(Debug, Clone, serde::Serialize)]
pub struct HistogramData {
    /// ビン境界（昇順、`len() == counts.len() + 1`）。
    pub bin_edges: Vec<f64>,
    /// 度数。
    pub counts: Vec<usize>,
}

/// パラメータ×目的の相関セクション。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CorrelationSection {
    /// 相関手法名（`"spearman"`）。
    pub method: String,
    /// 行に対応するパラメータ名（|ρ| 最大値降順、`max_heatmap_params` で cap）。
    pub params: Vec<String>,
    /// 列に対応する目的名。
    pub objectives: Vec<String>,
    /// `matrix[i][j]` = params[i] と objectives[j] の相関。計算不能は NaN。
    pub matrix: Vec<Vec<f64>>,
}

/// 多目的の意思決定分析（MCDM）セクション。
#[derive(Debug, Clone, serde::Serialize)]
pub struct McdmSection {
    /// 重み付けの方式（`"equal"` = 等重み）。
    pub weight_scheme: String,
    /// 各目的の重み。
    pub weights: Vec<f64>,
    /// TOPSIS の上位 trial。
    pub topsis_top: Vec<McdmEntry>,
    /// VIKOR の上位 trial。
    pub vikor_top: Vec<McdmEntry>,
    /// PROMETHEE II の上位 trial。
    pub promethee_top: Vec<McdmEntry>,
    /// 3手法すべての top10 に入る trial.number（昇順）。
    pub consensus_trials: Vec<u32>,
}

/// MCDM ランキングの1エントリ。
#[derive(Debug, Clone, serde::Serialize)]
pub struct McdmEntry {
    /// 順位（1 始まり）。
    pub rank: usize,
    /// trial.number。
    pub trial_number: u32,
    /// 目的値。
    pub objectives: Vec<f64>,
}

/// 実行時情報セクション。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExecutionSection {
    /// state ラベルごとの trial 数。
    pub state_counts: BTreeMap<String, usize>,
    /// 枝刈り率（PRUNED / 全終了 trial）。
    pub pruned_rate: f64,
    /// 枝刈り step の中央値（PRUNED の最終中間値 step）。無ければ `None`。
    pub median_prune_step: Option<f64>,
    /// 1 trial あたり平均所要秒。無ければ `None`。
    pub mean_trial_seconds: Option<f64>,
    /// 1 trial あたり所要秒の母標準偏差。無ければ `None`。
    pub std_trial_seconds: Option<f64>,
    /// 総所要時間（秒）。無ければ `None`。
    pub total_seconds: Option<f64>,
}

/// 再現情報。
#[derive(Debug, Clone, serde::Serialize)]
pub struct Reproduction {
    /// スタディ ID。
    pub study_id: u32,
    /// ストレージ表示名（マスク済み）。
    pub storage_display: String,
    /// 上位表の件数（options のエコー）。
    pub top_n: usize,
    /// 相関ヒートマップの最大パラメータ数（options のエコー）。
    pub max_heatmap_params: usize,
    /// スキーマバージョン。
    pub schema_version: u32,
}
