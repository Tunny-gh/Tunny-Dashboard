use crate::state::app_state::{
    ClusterResult, HeatmapMatrix, McdmResult, SensitivityResult, SobolResult, StudyContext,
    StudyMeta,
};
use crate::state::results::{EntropyResult, HvHistory};
use crate::ui::widgets::cluster_scatter::ClusterCacheKey;
use crate::ui::widgets::mcdm_chart::McdmCacheKey;

/// クラスタリングを開始したチャート。結果は設定キーで共有されるが、
/// 実行状態（spinner / エラー）は開始元のウィジェットに反映する必要があるため、
/// 完了・失敗メッセージにどのチャート発の計算かを持たせる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterChartSource {
    Scatter2D,
    Scatter3D,
    Table,
    ArtifactGallery,
}

/// MCDM 計算を開始したチャート。クラスタと同じく実行状態を開始元へ反映するために持たせる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McdmChartSource {
    Rank,
    Scatter2D,
    Scatter3D,
    Table,
    ArtifactGallery,
}

// ============================================================
// PDP Result types (placeholder for TASK-2025)
// ============================================================

#[derive(Debug, Clone)]
pub struct PdpResult1d {
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
    pub y_upper: Option<Vec<f64>>,
    pub y_lower: Option<Vec<f64>>,
    pub ice_lines: Vec<Vec<f64>>,
    pub r2: Option<f64>,
    pub param_name: String,
    pub objective_name: String,
}

#[derive(Debug, Clone)]
pub struct PdpResult2d {
    pub x_values: Vec<f64>,
    pub y_values: Vec<f64>,
    pub z_values: Vec<Vec<f64>>,
    pub param1_name: String,
    pub param2_name: String,
    pub objective_name: String,
    /// Posterior variance grid (GP methods only).
    pub uncertainties: Option<Vec<Vec<f64>>>,
}

#[derive(Debug, Clone)]
pub enum PdpResult {
    OneDim(PdpResult1d),
    TwoDim(PdpResult2d),
}

// ============================================================
// DownsampleKey
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownsampleKey {
    Scatter,
    Pcp,
    Thumbnail,
    Hover,
}

#[derive(Debug, Clone)]
pub struct ClusterUiError {
    pub user_message: String,
    pub detail_for_dev: Option<String>,
    pub retryable: bool,
}

pub fn cluster_ui_error(
    user_message: impl Into<String>,
    detail: Option<String>,
    retryable: bool,
) -> ClusterUiError {
    ClusterUiError {
        user_message: user_message.into(),
        detail_for_dev: if cfg!(debug_assertions) { detail } else { None },
        retryable,
    }
}

// ============================================================
// Response Surface Plot 関連型
// ============================================================

/// ResponseSurfacePlot の描画結果。
///
/// 応答曲面は Optimizer と同じサロゲートエンジンが生成する `SurfaceSlice`
/// （`目的関数 = f(全パラメータ)` の 2 パラメータスライス、他次元はベスト観測点に固定）。
/// 軸名・モデルラベル・検証 R² は表示用に付随させる。
#[derive(Debug, Clone)]
pub struct ResponseSurfaceResult {
    pub slice: tunny_core::surrogate_opt::SurfaceSlice,
    pub param_x_name: String,
    pub param_y_name: String,
    pub objective_name: String,
    /// 表示用モデル名（例: "GP-FITC"）。
    pub model_label: String,
    /// 学習済みサロゲートの検証 CV R²（平均）。表示用。
    pub cv_r2_mean: f64,
}

// ============================================================
// Surrogate Optimizer 関連型
// ============================================================

/// 獲得関数提案の UI 表示用結果。
#[derive(Debug, Clone)]
pub struct SurrogateSuggestUiResult {
    /// 提案候補（獲得関数の最適化結果）。
    pub candidates: Vec<tunny_core::surrogate_opt::SuggestedCandidate>,
    /// パラメータ名（`candidates[*].params` と同順）。
    pub param_names: Vec<String>,
    /// 目的名（表示用）。
    pub objective_name: String,
}

/// EHVI による多目的次候補提案の UI 表示用結果。
#[derive(Debug, Clone)]
pub struct SurrogateMultiSuggestUiResult {
    /// 提案候補（EHVI の最適化結果）。
    pub candidates: Vec<tunny_core::surrogate_opt::MultiSuggestedCandidate>,
    /// パラメータ名（`candidates[*].params` と同順）。
    pub param_names: Vec<String>,
    /// 目的名（`candidates[*].predicted_values` と同順）。
    pub objective_names: Vec<String>,
}

/// 多目的サロゲート最適化の UI 表示用結果。
/// 計算は `tunny_core::surrogate_opt` がバックグラウンドで行い、
/// パラメータ名・目的名・方向を付与してここへ詰め替える。
#[derive(Debug, Clone)]
pub struct SurrogateMultiOptUiResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    /// 目的ごとに true = 最小化。
    pub minimize: Vec<bool>,
    /// 予測パレートフロント（第 1 目的昇順ソート済み）。
    pub front: Vec<tunny_core::surrogate_opt::ParetoFrontPoint>,
    /// 目的ごとの訓練データ決定係数。
    pub r_squared: Vec<f64>,
    /// 目的ごとの応答曲面スライス（slice_params 無効時は空）。
    pub slices: Vec<tunny_core::surrogate_opt::SurfaceSlice>,
}

/// サロゲート最適化の UI 表示用結果。
/// 計算は `tunny_core::surrogate_opt` がバックグラウンドで行い、
/// パラメータ名と値の対応・方向（minimize/maximize）を付与してここへ詰め替える。
#[derive(Debug, Clone)]
pub struct SurrogateOptUiResult {
    /// 推定最適点（パラメータ名, 値）。
    pub best_params: Vec<(String, f64)>,
    /// 推定最適点でのサロゲート予測値（元の単位）。
    pub best_value: f64,
    /// 予測標準偏差（ガウス過程系のみ）。
    pub predicted_std: Option<f64>,
    /// 訓練データに対するサロゲートの決定係数。
    pub r_squared: f64,
    pub objective_name: String,
    /// true = 最小化問題として最適化した。
    pub minimize: bool,
    /// 最適点を通る応答曲面の 2D スライス（ヒートマップ表示用）。
    pub slice: Option<tunny_core::surrogate_opt::SurfaceSlice>,
    /// 観測データ中のベスト値（元の単位）。最小化なら最小値、最大化なら最大値。
    pub best_observed_value: f64,
    /// 推定最適点での制約サロゲート予測値（元の単位、制約名と同順）。制約なしなら空。
    pub predicted_constraints: Vec<(String, f64)>,
    /// 実行可能性確率（0.0〜1.0）。制約なしなら None。
    pub feasibility_probability: Option<f64>,
}

// ============================================================
// AppMessage
// ============================================================

pub enum AppMessage {
    JournalParsed {
        studies: Vec<StudyMeta>,
        path: std::path::PathBuf,
    },
    StudySelected {
        meta: StudyMeta,
        /// 共有ストア参照キー。UI 側が snapshot(study_id) で Arc<DataFrame> を取得する。
        study_id: u32,
        /// Pareto ランク（行 index 順、アプリ層算出）。StudyView の並行配列へ。
        pareto_rank: Vec<u32>,
        pareto_indices: Vec<u32>,
    },
    /// Study 選択時の逐次（ストリーミング）ロード。完了 Trial を 1000 件ごとに送り、
    /// UI 側はバッチごとに DataFrame を追記再構築して描画を更新する（読み込み中フリーズ回避）。
    /// Pareto ランクは `is_final` のバッチで一度だけ確定計算する。
    StudyChunkLoaded {
        study_id: u32,
        /// その時点までの累積 StudyMeta。
        meta: StudyMeta,
        /// 今回のバッチで新たに完了した Trial 行（core 表現）。
        new_rows: Vec<tunny_core::dataframe::TrialRow>,
        /// 累積パラメータ列名（ソート済み）。
        param_names: Vec<String>,
        /// 目的列名。
        objective_names: Vec<String>,
        /// 累積 user_attr 数値列名。
        user_attr_numeric_names: Vec<String>,
        /// 累積 user_attr 文字列列名。
        user_attr_string_names: Vec<String>,
        /// 観測した制約数の最大値。
        max_constraints: usize,
        /// 最初のバッチか（StudyContext を新規生成する）。
        is_first: bool,
        /// 最終バッチか（Pareto 確定・ローディング終了）。
        is_final: bool,
    },
    SensitivityDone {
        /// (metric cache_id, objective idx, feasible_only)
        key: (u8, usize, bool),
        result: SensitivityResult,
    },
    /// Sensitivity Heatmap 用：選択手法の全パラメータ × 全目的の感度行列。
    SensitivityHeatmapDone {
        metric: crate::ui::widgets::importance_chart::ImportanceMetric,
        feasible_only: bool,
        result: HeatmapMatrix,
    },
    SobolDone {
        /// (objective idx, feasible_only)
        key: (usize, bool),
        result: SobolResult,
    },
    ClusteringDone {
        source: ClusterChartSource,
        key: ClusterCacheKey,
        result: ClusterResult,
    },
    ClusterFailed {
        source: ClusterChartSource,
        err: ClusterUiError,
    },
    McdmDone {
        source: McdmChartSource,
        key: McdmCacheKey,
        result: McdmResult,
    },
    McdmFailed {
        source: McdmChartSource,
        message: String,
    },
    EntropyDone {
        source: McdmChartSource,
        result: EntropyResult,
    },
    PdpDone {
        param: String,
        objective: String,
        model_type: String,
        feasible_only: bool,
        result: PdpResult,
    },
    Pdp2dDone(PdpResult2d),
    DownsampleDone {
        key: DownsampleKey,
        indices: Vec<u32>,
    },
    LiveUpdateDone {
        new_trial_rows: Vec<tunny_core::io::journal::live_update::TrialRow>,
        updated_study_counts: Vec<(u32, usize)>,
    },
    /// 連続エラー（ファイルアクセス失敗など）をポーラーが検出した
    LiveUpdateError(String),
    /// 60秒間ファイル変化がなく最適化完了の可能性を検出した
    LiveUpdateMaybeComplete,
    /// 収束指標（HV / IGD+ / ε / R2）の推移計算が完了した。
    /// 基準 Study と比較 Study の全系列を一括計算し、共通参照セットで正規化する。
    IndicatorHistoryDone {
        indicator: tunny_core::indicators::MoIndicator,
        /// 基準 Study の指標推移。
        base: HvHistory,
        /// 比較 Study の指標推移（comparison_studies と同じ順序）。
        comparisons: Vec<HvHistory>,
    },
    Error(String),
    SensitivityError(String),

    // ── TASK-2112: 新規バリアント ────────────────────────────────────
    /// REQ-006: 比較 Study のロード完了
    ComparisonStudyLoaded {
        study_idx: usize,
        context: Box<StudyContext>,
    },
    /// REQ-007: Artifacts ディレクトリスキャン完了
    ArtifactsDirScanned {
        trial_artifacts: std::collections::HashMap<u32, Vec<crate::io::artifacts::ArtifactEntry>>,
        artifacts_dir: std::path::PathBuf,
    },
    /// REQ-005: HTML レポート生成完了
    HtmlReportDone {
        html: String,
        suggested_filename: String,
    },
    ComparisonStudyLoadFailed(String),
    /// ResponseSurfacePlot のスライス生成が完了した（学習済みサロゲートから生成）。
    ResponseSurfaceDone(ResponseSurfaceResult),
    ResponseSurfaceFailed(String),
    /// サロゲートのフィット＋検証が完了した（最適化段階は別メッセージ）。
    SurrogateFitDone(std::sync::Arc<tunny_core::surrogate_opt::TrainedSurrogate>),
    SurrogateFitFailed(String),
    /// サロゲートのフィットがユーザー操作でキャンセルされた。
    SurrogateFitCancelled,
    SurrogateOptDone(SurrogateOptUiResult),
    SurrogateOptFailed(String),
    /// 多目的サロゲートのフィット＋検証が完了した（全目的分の学習結果を保持）。
    SurrogateMultiFitDone(std::sync::Arc<Vec<tunny_core::surrogate_opt::TrainedSurrogate>>),
    SurrogateMultiFitFailed(String),
    /// 多目的サロゲートのフィットがユーザー操作でキャンセルされた。
    SurrogateMultiFitCancelled,
    SurrogateMultiOptDone(SurrogateMultiOptUiResult),
    SurrogateMultiOptFailed(String),
    ChartCaptureFailed(String),
    /// 獲得関数による候補提案が完了した。
    SurrogateSuggestDone(SurrogateSuggestUiResult),
    /// 獲得関数による候補提案が失敗した。
    SurrogateSuggestFailed(String),
    /// EHVI による多目的候補提案が完了した。
    SurrogateMultiSuggestDone(SurrogateMultiSuggestUiResult),
    /// EHVI による多目的候補提案が失敗した。
    SurrogateMultiSuggestFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_message_error_variant() {
        let msg = AppMessage::Error("test error".to_string());
        match msg {
            AppMessage::Error(e) => assert_eq!(e, "test error"),
            _ => panic!("Expected Error variant"),
        }
    }

    #[test]
    fn downsample_key_equality() {
        assert_eq!(DownsampleKey::Scatter, DownsampleKey::Scatter);
        assert_ne!(DownsampleKey::Scatter, DownsampleKey::Pcp);
    }

    #[test]
    fn pdp_result_one_dim() {
        let result = PdpResult::OneDim(PdpResult1d {
            x_values: vec![0.0, 0.5, 1.0],
            y_values: vec![1.0, 0.5, 0.0],
            y_upper: None,
            y_lower: None,
            ice_lines: vec![],
            r2: None,
            param_name: "x".to_string(),
            objective_name: "y".to_string(),
        });
        match result {
            PdpResult::OneDim(r) => assert_eq!(r.x_values.len(), 3),
            _ => panic!("Expected OneDim"),
        }
    }

    // ── ResponseSurface 新規バリアントのテスト ────────

    #[test]
    fn message_handler_accepts_new_message_family() {
        let slice = tunny_core::surrogate_opt::SurfaceSlice {
            param_x_idx: 0,
            param_y_idx: 1,
            x_values: vec![0.0, 1.0],
            y_values: vec![0.0, 1.0],
            z_values: vec![vec![0.0, 1.0], vec![1.0, 2.0]],
            z_std: None,
        };
        let msgs: Vec<AppMessage> = vec![
            AppMessage::ComparisonStudyLoadFailed("err".to_string()),
            AppMessage::ResponseSurfaceDone(ResponseSurfaceResult {
                slice,
                param_x_name: "x".to_string(),
                param_y_name: "y".to_string(),
                objective_name: "f".to_string(),
                model_label: "Ridge".to_string(),
                cv_r2_mean: 0.5,
            }),
            AppMessage::ResponseSurfaceFailed("compute error".to_string()),
            AppMessage::ChartCaptureFailed("capture error".to_string()),
        ];
        // all variants should be matchable without panic
        for msg in msgs {
            match msg {
                AppMessage::ComparisonStudyLoadFailed(e) => assert!(!e.is_empty()),
                AppMessage::ResponseSurfaceDone(r) => assert_eq!(r.slice.x_values.len(), 2),
                AppMessage::ResponseSurfaceFailed(e) => assert!(!e.is_empty()),
                AppMessage::ChartCaptureFailed(e) => assert!(!e.is_empty()),
                _ => {}
            }
        }
    }
}
