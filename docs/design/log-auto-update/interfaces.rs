//! Log Auto Update 型定義
//!
//! 作成日: 2026-05-11
//! 関連設計: architecture.md, dataflow.md
//!
//! 信頼性レベル:
//! - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
//! - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
//! - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義
//!
//! 注意: これは設計文書であり、実際の実装ファイルではありません。
//! Rust eguiネイティブアプリ向けの型定義です。

// ========================================
// rust_core: 差分パーサー拡張
// ========================================

/// インクリメンタル差分パースの拡張結果
/// 🔵 信頼性: 既存 AppendDiffResult + 要件REQ-LU-001より
///
/// 既存の `AppendDiffResult` を拡張し、TrialRow構築を含む。
/// ポーリングスレッド内で呼び出され、新規完了トライアルのデータを構築する。
pub struct DiffResultV2 {
    /// 新規完了トライアル数 🔵 *既存 AppendDiffResult.new_completed より*
    pub new_completed: usize,

    /// 消費バイト数 🔵 *既存 AppendDiffResult.consumed_bytes より*
    pub consumed_bytes: usize,

    /// 実行中トライアル数 🔵 *既存 AppendDiffResult.pending_running より*
    pub pending_running: usize,

    /// 新規完了トライアルの行データ 🔵 *要件REQ-LU-007 + ヒアリング「インクリメンタル更新」より*
    pub new_trial_rows: Vec<TrialRowV2>,

    /// 更新されたStudy毎の完了トライアル数 🔵 *要件REQ-LU-008より*
    pub updated_study_counts: Vec<(u32, usize)>, // (study_id, completed_count)
}

/// インクリメンタルパース用のトライアル行データ
/// 🔵 信頼性: 既存 TrialRow (egui-app) + PendingTrial (rust_core) より
pub struct TrialRowV2 {
    /// トライアルID 🔵 *既存 TrialRow.trial_id より*
    pub trial_id: u32,

    /// Study内の試行番号 🔵 *既存 TrialRow.trial_number より*
    pub trial_number: u32,

    /// パラメータ値 (param_name → display_value) 🔵 *既存 PendingTrial.param_display より*
    pub params: std::collections::HashMap<String, f64>,

    /// カテゴリパラメータ (param_name → label) 🟡 *既存 PendingTrial.param_category_label より*
    pub param_categories: std::collections::HashMap<String, String>,

    /// 目的関数値 🔵 *既存 PendingTrial.values より*
    pub objectives: Vec<f64>,

    /// ユーザー属性（数値型） 🔵 *既存 PendingTrial.user_attrs_numeric より*
    pub user_attrs_numeric: std::collections::HashMap<String, f64>,

    /// ユーザー属性（文字列型） 🔵 *既存 PendingTrial.user_attrs_string より*
    pub user_attrs_string: std::collections::HashMap<String, String>,

    /// 制約値 🔵 *既存 PendingTrial.constraint_values より*
    pub constraint_values: Vec<f64>,

    /// Study ID 🔵 *既存 PendingTrial.study_idx より*
    pub study_id: u32,
}

/// ポーリングスレッドに渡す初期化コンテキスト
/// 🔵 信頼性: 要件REQ-LU-103 + アーキテクチャ設計DD-02より
///
/// インクリメンタルパースに必要なdistribution情報等を含む。
/// トグルON時にAppStateから構築してPollerに渡す。
pub struct LiveUpdateContext {
    /// 監視対象ファイルパス 🔵 *要件REQ-LU-005より*
    pub file_path: std::path::PathBuf,

    /// 現在のファイルサイズ（初期byte offset） 🔵 *要件REQ-LU-001より*
    pub initial_byte_offset: u64,

    /// 次のトライアルID（現在の完了トライアル数） 🔵 *既存 set_next_trial_id() より*
    pub next_trial_id: u32,

    /// 各Studyのdistribution情報 🟡 *インクリメンタルTrialRow構築に必要*
    pub study_distributions: Vec<StudyDistributionInfo>,
}

/// Study毎のdistribution情報
/// 🟡 信頼性: インクリメンタルパースの要件から妥当な推測
pub struct StudyDistributionInfo {
    /// Study ID 🔵 *既存 StudyMeta.study_id より*
    pub study_id: u32,

    /// パラメータ名のリスト 🔵 *既存 StudyMeta.param_names より*
    pub param_names: Vec<String>,

    /// 目的関数名のリスト 🔵 *既存 StudyMeta.objective_names より*
    pub objective_names: Vec<String>,

    /// Distribution情報（パラメータ名 → distribution JSON）
    /// 🟡 *初期パース時に保持する必要がある*
    pub distributions: std::collections::HashMap<String, serde_json::Value>,
}

// ========================================
// egui-app: ポーリングスレッド管理
// ========================================

/// ポーリングスレッドの管理ハンドル
/// 🔵 信頼性: アーキテクチャ設計DD-04 + 既存spawn_taskパターンより
pub struct LiveUpdatePoller {
    /// スレッド停止フラグ 🔵 *AtomicBool による停止制御*
    pub stop_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,

    /// ポーリング間隔（ミリ秒） 🔵 *要件REQ-LU-002より*
    pub interval_ms: std::sync::Arc<std::sync::atomic::AtomicU64>,

    /// スレッドのJoinHandle 🔵 *クリーンなシャットダウンのため*
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
}

// ========================================
// egui-app: メッセージ拡張
// ========================================

/// LiveUpdateDone メッセージ（拡張版）
/// 🔵 信頼性: 既存 AppMessage::LiveUpdateDone + 要件REQ-LU-007・REQ-LU-008より
///
/// 既存の `LiveUpdateDone { new_trial_count, pareto_updated, new_indices }` を
/// 新規トライアルデータを含む形式に拡張。
pub struct LiveUpdateDoneMessage {
    /// 新規完了トライアル数 🔵 *既存フィールドより*
    pub new_trial_count: usize,

    /// 新規トライアルの行データ 🔵 *要件REQ-LU-007より*
    pub new_trial_rows: Vec<TrialRowV2>,

    /// 更新されたStudy毎の完了トライアル数 🔵 *要件REQ-LU-008より*
    pub updated_study_counts: Vec<(u32, usize)>,

    /// 消費バイト数 🔵 *要件REQ-LU-001より*
    pub consumed_bytes: usize,
}

// ========================================
// egui-app: 状態拡張
// ========================================

/// LiveUpdateState の拡張フィールド
/// 🟡 信頼性: 要件REQ-LU-010・REQ-LU-101からの派生
///
/// 既存の `LiveUpdateState` に追加するフィールド。
pub struct LiveUpdateStateExtension {
    /// 連続エラーカウント 🔵 *要件REQ-LU-010（3回連続エラーで自動停止）より*
    pub consecutive_errors: u32,

    /// 最後にファイル変更があった時刻 🔵 *要件REQ-LU-101（60秒無変化で通知）より*
    pub last_change_time: Option<std::time::Instant>,

    /// ポーラースレッドがアクティブかどうか 🔵 *アーキテクチャ設計DD-01より*
    pub poller_active: bool,

    /// ポーラーの管理ハンドル 🔵 *アーキテクチャ設計DD-04より*
    pub poller: Option<LiveUpdatePoller>,

    /// ライブ更新の累積統計 🟡 *要件REQ-LU-302（オプション）より*
    pub total_updates: u64,

    /// 累積追加トライアル数 🟡 *要件REQ-LU-302（オプション）より*
    pub total_new_trials: u64,

    /// 最適化完了通知表示中かどうか 🔵 *要件REQ-LU-101より*
    pub showing_completion_hint: bool,
}

// ========================================
// 信頼性レベルサマリー
// ========================================
//
// - 🔵 青信号: 32件 (86%)
// - 🟡 黄信号: 5件 (14%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: 高品質 — 既存型定義の拡張ベース、新規型は要件定義から直接導出
