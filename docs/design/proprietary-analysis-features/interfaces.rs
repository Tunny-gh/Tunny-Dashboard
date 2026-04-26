// ============================================================
// プロプライエタリ分析ツール不足機能 型定義（Rust）
//
// 作成日: 2026-04-26
// 関連設計: architecture.md
// 関連要件: docs/spec/proprietary-analysis-features/requirements.md
//
// 信頼性レベル:
// - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
// - 🟡 黄信号: 要件定義書・設計文書・既存実装から妥当な推測による型定義
// - 🔴 赤信号: 要件定義書・設計文書・既存実装にない推測による型定義
//
// 注意: このファイルはコード生成用の設計仕様であり、実際のファイルパスは
// 以下の通りに分散して追加・拡張する:
//   - egui-app/src/io/session.rs      (SessionSnapshot 拡張)
//   - egui-app/src/io/html_report.rs  (HtmlReportBuilder 新規)
//   - egui-app/src/io/artifacts.rs    (ArtifactsScanner 新規)
//   - egui-app/src/state/messages.rs  (AppMessage バリアント追加)
//   - egui-app/src/state/app_state.rs (AppState フィールド追加)
//   - egui-app/src/state/layout_state.rs (LayoutMode::Comparison 追加)
// ============================================================

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

// ============================================================
// REQ-002/003/004: SessionSnapshot 拡張
// egui-app/src/io/session.rs に追加
// ============================================================

/// 分析セッション保存用スナップショット (.tdash フォーマット)
///
/// 🔵 信頼性: REQ-004-B・`io/session.rs` 既存実装・note.md .tdash 仕様より
///
/// 既存の SessionSnapshot に以下のフィールドを追加拡張する。
/// serde(default) により、フィールドがない旧バージョンの JSON も読み込み可能。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    // --- 既存フィールド（変更なし）🔵 ---
    pub study_name: String,            // 🔵 既存 session.rs より
    pub filter_ranges: HashMap<String, (f64, f64)>,  // 🔵 REQ-002-B より
    pub selected_indices: Vec<u32>,    // 🔵 REQ-002-B より
    pub saved_at: String,              // 🔵 既存 session.rs より

    // --- 追加フィールド: フィルタ拡張 (REQ-002) 🔵 ---
    #[serde(default)]
    pub color_mode: String,            // 🔵 REQ-002-B より ("pareto_rank" | "cluster" | "objective_*")

    // --- 追加フィールド: セッション (REQ-004) 🔵 ---
    #[serde(default = "default_version")]
    pub version: String,               // 🔵 note.md .tdash 仕様より ("1.0")
    #[serde(default)]
    pub journal_filename: String,      // 🔵 REQ-004-B より (journal ファイル名のみ、パスなし)
    #[serde(default)]
    pub selected_study_id: Option<u32>, // 🔵 REQ-004-B より

    // --- 追加フィールド: Trade-off Navigator (REQ-001 + REQ-004) 🔵 ---
    #[serde(default)]
    pub tradeoff_weights: Vec<f64>,    // 🔵 REQ-001-G・REQ-004-B より

    // --- 追加フィールド: クラスタリング設定 (REQ-004) 🔵 ---
    #[serde(default)]
    pub cluster_config: Option<ClusterConfig>, // 🔵 REQ-004-B より

    // --- 追加フィールド: レイアウト設定 (REQ-003 + REQ-004) 🔵 ---
    #[serde(default)]
    pub layout_mode: String,           // 🔵 REQ-003-D より ("MultiObjective" | "FreeLayout" | ...)
    #[serde(default)]
    pub layout_config: Option<LayoutConfig>, // 🔵 REQ-003-B より

    // --- 追加フィールド: ピン留め試行 (REQ-004) 🔵 ---
    #[serde(default)]
    pub pinned_trials: Vec<u32>,       // 🔵 REQ-004-B より

    // --- 追加フィールド: PCP 軸状態 (REQ-009 + REQ-003) 🟡 ---
    #[serde(default)]
    pub pcp_axis_order: Vec<String>,   // 🟡 REQ-009-C から妥当な推測
    #[serde(default)]
    pub pcp_axis_visibility: HashMap<String, bool>, // 🟡 REQ-009-C から妥当な推測
}

fn default_version() -> String {
    "1.0".to_string()
}

// ============================================================
// REQ-003: LayoutConfig — グリッドレイアウト保存構造
// egui-app/src/io/session.rs に追加
// ============================================================

/// グリッドレイアウト設定のシリアライズ用構造体
///
/// 🔵 信頼性: REQ-003-B・`layout_state.rs` GridCell/GridLayout 調査より
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutConfig {
    pub rows: usize,                           // 🔵 GridLayout より
    pub cols: usize,                           // 🔵 GridLayout より
    pub cells: Vec<GridCellSnapshot>,          // 🔵 REQ-003-B より
}

/// グリッドセルのシリアライズ用スナップショット
///
/// 🔵 信頼性: REQ-003-B・GridCell 実装調査より
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCellSnapshot {
    pub row: usize,                            // 🔵 GridCell より
    pub col: usize,                            // 🔵 GridCell より
    pub content: Option<String>,               // 🔵 PanelItem::label() を文字列化
    pub col_span: u8,                          // 🔵 GridCell より
    pub row_span: u8,                          // 🔵 GridCell より
}

// ============================================================
// REQ-004: ClusterConfig — クラスタリング設定保存
// egui-app/src/io/session.rs に追加
// ============================================================

/// クラスタリング設定のシリアライズ用構造体
///
/// 🔵 信頼性: REQ-004-B・note.md .tdash 仕様より
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub space: String,                         // 🔵 note.md より ("objective" | "parameter")
    pub k: usize,                              // 🔵 note.md より
}

// ============================================================
// REQ-001: AppMessage 追加バリアント
// egui-app/src/state/messages.rs に追加
// ============================================================

// 以下のバリアントを AppMessage enum に追加する:
//
// /// Trade-off Navigator のスコアリング結果
// /// 🔵 信頼性: REQ-001-D・tradeoff.rs 実装調査・設計ヒアリング Q1 より
// TradeoffDone {
//     sorted_indices: Vec<u32>,   // チェビシェフスコア昇順（best が先頭）
// },
//
// /// 比較用 Study のロード完了
// /// 🔵 信頼性: REQ-006-A・設計ヒアリング Q3 より
// ComparisonStudyLoaded {
//     study_idx: usize,           // comparison_studies の格納インデックス (0-3)
//     context: Box<StudyContext>, // Box化でスタックサイズを抑制
// },
//
// /// Artifacts フォルダスキャン完了
// /// 🔵 信頼性: REQ-007-A・設計ヒアリング Q1 より
// ArtifactsDirScanned {
//     trial_artifacts: HashMap<u32, Vec<PathBuf>>,  // trial_id → ファイルパス一覧
//     artifacts_dir: PathBuf,                        // スキャンしたフォルダパス
// },
//
// /// HTML レポート生成完了
// /// 🔵 信頼性: REQ-005-A・NFR-003 より
// HtmlReportDone {
//     html: String,                       // 完成した HTML 文字列
//     suggested_filename: String,         // "{study_name}_{timestamp}.html"
// },

// ============================================================
// REQ-005: HtmlReportBuilder
// egui-app/src/io/html_report.rs (新規)
// ============================================================

/// HTML レポート生成のためのスナップショット（バックグラウンドスレッドに渡す）
///
/// 🔵 信頼性: REQ-005-A〜D・NFR-003 より
#[derive(Debug, Clone)]
pub struct HtmlReportSnapshot {
    pub study_name: String,                    // 🔵 REQ-005-B より
    pub objective_names: Vec<String>,          // 🔵 REQ-005-B より
    pub param_names: Vec<String>,              // 🔵 REQ-005-B より
    pub total_trials: usize,                   // 🔵 REQ-005-B より
    pub pareto_count: usize,                   // 🔵 REQ-005-B より
    pub selected_trials: Vec<HtmlTrialRow>,    // 🔵 REQ-005-D より（フィルタ選択試行のみ）
    pub statistics: TrialStatistics,           // 🔵 REQ-005-B より
}

/// HTML レポート用試行行データ
///
/// 🔵 信頼性: REQ-005-B・TrialRow 実装調査より
#[derive(Debug, Clone)]
pub struct HtmlTrialRow {
    pub trial_id: u32,                         // 🔵 TrialRow より
    pub trial_number: u32,                     // 🔵 TrialRow より
    pub params: HashMap<String, f64>,          // 🔵 TrialRow より
    pub objectives: Vec<f64>,                  // 🔵 TrialRow より
    pub pareto_rank: u32,                      // 🔵 TrialRow より
}

/// HTML レポート用統計情報
///
/// 🔵 信頼性: REQ-005-B より
#[derive(Debug, Clone)]
pub struct TrialStatistics {
    pub objective_means: Vec<f64>,             // 🔵 REQ-005-B より
    pub objective_variances: Vec<f64>,         // 🔵 REQ-005-B より
    pub pareto_count: usize,                   // 🔵 REQ-005-B より
    pub selected_count: usize,                 // 🔵 REQ-005-D より
}

// ============================================================
// REQ-006: ComparisonState — AppState に追加
// egui-app/src/state/app_state.rs に追加
// ============================================================

// 以下のフィールドを AppState struct に追加する:
//
// /// 比較モードフラグ (REQ-006-A)
// /// 🔵 信頼性: REQ-006-A・設計ヒアリング Q3 より
// pub comparison_mode: bool,
//
// /// 比較対象 Study コンテキスト最大4件 (REQ-006-A)
// /// 🔵 信頼性: REQ-006-A より
// pub comparison_studies: Vec<StudyContext>,
//
// /// 比較 Study ごとの代表色 (REQ-006-B / NFR-102)
// /// 🔵 信頼性: REQ-006-B・NFR-102 より
// pub comparison_colors: Vec<egui::Color32>,

/// 比較モード統計サマリー（比較パネルが計算して表示）
///
/// 🔵 信頼性: REQ-006-B 統計サマリー比較テーブルより
#[derive(Debug, Clone)]
pub struct ComparisonSummary {
    pub study_name: String,                    // 🔵 REQ-006-B より
    pub total_trials: usize,                   // 🔵 REQ-006-B より
    pub pareto_count: usize,                   // 🔵 REQ-006-B より
    pub best_values: Vec<f64>,                 // 🔵 REQ-006-B より（目的数分）
    pub mean_values: Vec<f64>,                 // 🔵 REQ-006-B より（目的数分）
    pub color: [u8; 4],                        // 🟡 [r, g, b, a]（egui::Color32 の serde 代替）
}

// ============================================================
// REQ-007: ArtifactsScanner
// egui-app/src/io/artifacts.rs (新規)
// ============================================================

/// Artifacts フォルダスキャン結果
///
/// 🔵 信頼性: REQ-007-A〜H・NFR-201 より
#[derive(Debug, Clone)]
pub struct ArtifactsScanResult {
    pub base_dir: PathBuf,                     // 🔵 REQ-007-A より
    pub trial_artifacts: HashMap<u32, Vec<ArtifactFile>>, // 🔵 REQ-007-C より
}

/// 個別アーティファクトファイル情報
///
/// 🔵 信頼性: REQ-007-D〜G より
#[derive(Debug, Clone)]
pub struct ArtifactFile {
    pub path: PathBuf,                         // 🔵 NFR-201 正規化済みパス
    pub filename: String,                      // 🔵 REQ-007-G より
    pub size_bytes: u64,                       // 🔵 REQ-007-G より
    pub file_type: ArtifactFileType,           // 🔵 REQ-007-E〜G より
}

/// アーティファクトファイルタイプ分類
///
/// 🔵 信頼性: REQ-007-E〜G より
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactFileType {
    Image,    // 🔵 REQ-007-E より (.png / .jpg / .jpeg / .webp)
    Csv,      // 🔵 REQ-007-F より (.csv)
    Other,    // 🔵 REQ-007-G より (その他)
}

impl ArtifactFileType {
    /// ファイル拡張子から ArtifactFileType を判定する
    ///
    /// 🔵 信頼性: REQ-007-E/F/G より
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "webp" => ArtifactFileType::Image, // 🔵 REQ-007-E
            "csv" => ArtifactFileType::Csv,                              // 🔵 REQ-007-F
            _ => ArtifactFileType::Other,                                // 🔵 REQ-007-G
        }
    }
}

// ============================================================
// REQ-008: ConvergenceDiagnostics — 単目的専用 UI 用データ
// egui-app/src/state/app_state.rs への追加
// ============================================================

// 以下のフィールドを AppState struct に追加する:
//
// /// 単目的 Best 値推移 (REQ-008-C)
// /// 🔵 信頼性: REQ-008-B・REQ-008-C より
// pub best_trial_history: Option<Vec<BestTrialEntry>>,

/// 単目的最適化の Best 値推移エントリ
///
/// 🔵 信頼性: REQ-008-B・REQ-008-E/F より
#[derive(Debug, Clone)]
pub struct BestTrialEntry {
    pub trial_id: u32,                         // 🔵 REQ-008-E より
    pub trial_number: u32,                     // 🔵 REQ-008-E より
    pub objective_value: f64,                  // 🔵 REQ-008-E/F より
    pub delta_value: f64,                      // 🔵 REQ-008-F より（前 Best との差分）
    pub top5_params: Vec<(String, f64)>,       // 🔵 REQ-008-F より（重要度上位5変数）
}

/// 収束診断カード表示用データ
///
/// 🔵 信頼性: REQ-008-B より
#[derive(Debug, Clone)]
pub struct ConvergenceDiagnostics {
    pub best_value: f64,                       // 🔵 REQ-008-B より
    pub best_trial_id: u32,                    // 🔵 REQ-008-B より
    pub recent_improvement_rate: f64,          // 🔵 REQ-008-B より（直近100試行の改善率）
}

impl ConvergenceDiagnostics {
    /// TrialRow の Vec から収束診断データを計算する
    ///
    /// 🔵 信頼性: REQ-008-B より
    pub fn compute(trial_rows: &[crate::state::types::TrialRow], is_minimize: bool) -> Option<Self> {
        if trial_rows.is_empty() {
            return None;
        }
        // best_value, best_trial_id, recent_improvement_rate を計算
        // (実装詳細は tdd-red フェーズで確定)
        todo!()
    }
}

// ============================================================
// REQ-009: ParallelCoordsChart 拡張フィールド
// egui-app/src/ui/widgets/parallel_coords.rs の ParallelCoordsChart struct に追加
// ============================================================

// 以下のフィールドを ParallelCoordsChart struct に追加する:
//
// /// 各軸の表示/非表示状態 (REQ-009-A)
// /// 🔵 信頼性: REQ-009-A より
// pub axis_visibility: HashMap<String, bool>,
//
// (既存の axis_order: Vec<String> は REQ-009-B のドラッグ並び替えに使用)

// ============================================================
// REQ-001: AppState に追加するフィールド
// egui-app/src/state/app_state.rs に追加
// ============================================================

// 以下のフィールドを AppState struct に追加する:
//
// /// Trade-off Navigator の重み (REQ-001-B)
// /// 🔵 信頼性: REQ-001-B・REQ-001-G より
// pub tradeoff_weights: Vec<f64>,
//
// /// チェビシェフスコア昇順ソート済みインデックス (REQ-001-D/E)
// /// 🔵 信頼性: REQ-001-D・tradeoff.rs 実装調査より
// pub tradeoff_sorted_indices: Option<Vec<u32>>,

// ============================================================
// REQ-007: AppState に追加するフィールド
// egui-app/src/state/app_state.rs に追加
// ============================================================

// 以下のフィールドを AppState struct に追加する:
//
// /// Artifacts フォルダパス（自動検出または手動選択）(REQ-007-A/B)
// /// 🔵 信頼性: REQ-007-A・REQ-007-B より
// pub artifacts_dir: Option<PathBuf>,
//
// /// trial_id → ArtifactFile 一覧のマップ (REQ-007-C)
// /// 🔵 信頼性: REQ-007-C・ArtifactFile 型定義より
// pub artifact_map: HashMap<u32, Vec<ArtifactFile>>,

// ============================================================
// LayoutMode 追加バリアント
// egui-app/src/state/layout_state.rs の LayoutMode enum に追加
// ============================================================

// 以下のバリアントを LayoutMode enum に追加する:
//
// /// 複数 Study 比較専用レイアウト (REQ-006)
// /// 🔵 信頼性: REQ-006-A・設計ヒアリング Q3 より
// Comparison,

// ============================================================
// WidgetStates に追加するフィールド
// egui-app/src/ui/widget_states.rs の WidgetStates struct、
// および各ウィジェット struct に追加
// ============================================================

// OptimizationHistoryChart struct に追加 (REQ-008):
//
// /// Best 値追跡ライン表示 (REQ-008-C)
// /// 🔵 信頼性: REQ-008-C より
// pub show_best_line: bool,
//
// /// Y 軸半対数スケール (REQ-008-D)
// /// 🔵 信頼性: REQ-008-D より
// pub log_scale: bool,

// ============================================================
// 信頼性レベルサマリー
//
// 🔵 青信号: 51件 (89%)
// 🟡 黄信号: 6件 (11%)
// 🔴 赤信号: 0件 (0%)
//
// 品質評価: ✅ 高品質
// ============================================================
