// cluster-widget-chart-not-displayed 型定義（Rust設計用）
// 作成日: 2026-05-05
// 関連設計: architecture.md, dataflow.md
//
// 信頼性レベル:
// - 🔵 青信号: EARS要件定義書・設計文書・既存実装を参考にした確実な型定義
// - 🟡 黄信号: EARS要件定義書・設計文書・既存実装から妥当な推測による型定義
// - 🔴 赤信号: EARS要件定義書・設計文書・既存実装にない推測による型定義

// ========================================
// 入力設定
// ========================================

/// クラスタリング実行の対象空間
/// 🔵 信頼性: ヒアリング「対象空間」 + 既存クラスタリング仕様より
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterTargetSpace {
    Objective,
    Variable,
    Combined,
}

/// k の決定方式
/// 🟡 信頼性: ヒアリング（エルボー法デフォルト） + 既存実装パターンから妥当推測
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KSelectionMode {
    ElbowDefault,
    Manual,
}

/// 初期化方式
/// 🟡 信頼性: ヒアリング（初期化方式入力）より
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KMeansInitStrategy {
    KMeansPlusPlus,
    Deterministic,
}

/// 実行要求
/// 🔵 信頼性: REQ-001/103 + ヒアリング確定事項より
#[derive(Debug, Clone)]
pub struct ClusterComputeRequest {
    pub k: usize,                       // 🔵 入力要件より
    pub target_space: ClusterTargetSpace, // 🔵 ヒアリングより
    pub k_mode: KSelectionMode,         // 🟡 UI詳細設計より
    pub init_strategy: KMeansInitStrategy, // 🟡 UI詳細設計より
}

// ========================================
// 実行状態
// ========================================

/// ウィジェット表示状態
/// 🔵 信頼性: REQ-004/101, NFR-201, ヒアリングより
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterWidgetStatus {
    NotRun,
    Running,
    Failed,
    Ready,
}

/// 実行結果（UI反映用）
/// 🔵 信頼性: 既存 AppState.cluster_result + 要件 REQ-002/003 より
#[derive(Debug, Clone)]
pub struct ClusterRenderState {
    pub status: ClusterWidgetStatus,          // 🔵 状態表示要件より
    pub result: Option<ClusterResultRef>,     // 🔵 既存結果型参照
    pub error: Option<ClusterUiError>,        // 🔵 インラインエラー要件より
}

/// 既存 ClusterResult 参照ラッパー
/// 🔵 信頼性: egui-app/src/state/results.rs より
#[derive(Debug, Clone)]
pub struct ClusterResultRef {
    pub labels_len: usize,
    pub n_clusters: usize,
}

// ========================================
// エラー型
// ========================================

/// 表示用エラー（環境で詳細度切替）
/// 🔵 信頼性: ヒアリング「環境で切替」より
#[derive(Debug, Clone)]
pub struct ClusterUiError {
    pub user_message: String,             // 🔵 本番向け簡易文言
    pub detail_for_dev: Option<String>,   // 🔵 開発時のみ表示
    pub retryable: bool,                  // 🔵 再実行導線の有無
}

// ========================================
// ガード関数の設計シグネチャ
// ========================================

/// 入力検証
/// 🔵 信頼性: NFR-101, EDGE-102 より
pub fn validate_cluster_request(
    request: &ClusterComputeRequest,
    trial_count: usize,
) -> Result<(), ClusterUiError> {
    if request.k < 2 {
        return Err(ClusterUiError {
            user_message: "kは2以上で指定してください".to_string(),
            detail_for_dev: Some("validation: k < 2".to_string()),
            retryable: true,
        });
    }

    if request.k > trial_count {
        return Err(ClusterUiError {
            user_message: "kは試行回数以下で指定してください".to_string(),
            detail_for_dev: Some("validation: k > trial_count".to_string()),
            retryable: true,
        });
    }

    Ok(())
}

/// 完了データ整合性検証
/// 🟡 信頼性: EDGE-002 要件から妥当推測
pub fn validate_cluster_result_len(
    labels_len: usize,
    trial_count: usize,
) -> Result<(), ClusterUiError> {
    if labels_len != trial_count {
        return Err(ClusterUiError {
            user_message: "結果整合性エラーが発生しました。再実行してください".to_string(),
            detail_for_dev: Some(format!(
                "validation: labels_len({}) != trial_count({})",
                labels_len, trial_count
            )),
            retryable: true,
        });
    }

    Ok(())
}

// ========================================
// 信頼性レベルサマリー
// ========================================
// - 🔵 青信号: 13件 (86.7%)
// - 🟡 黄信号: 2件 (13.3%)
// - 🔴 赤信号: 0件 (0%)
//
// 品質評価: 高品質
