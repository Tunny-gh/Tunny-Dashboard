use crate::state::app_state::{
    ClusterResult, GpuBufferData, McdmResult, SensitivityResult, SobolResult, StudyContext,
    StudyMeta, TopsisResult, TrialRow,
};
use crate::state::results::{AhpResult, EntropyResult};

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
    /// Posterior variance grid (Kriging / Sparse Kriging only).
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
// AppMessage
// ============================================================

pub enum AppMessage {
    JournalParsed {
        studies: Vec<StudyMeta>,
        path: std::path::PathBuf,
    },
    StudySelected {
        meta: StudyMeta,
        trial_rows: Vec<TrialRow>,
        gpu_data: GpuBufferData,
        pareto_indices: Vec<u32>,
    },
    SensitivityDone {
        key: (u8, usize),
        result: SensitivityResult,
    },
    SobolDone {
        obj_idx: usize,
        result: SobolResult,
    },
    ClusteringDone(ClusterResult),
    ClusterFailed(ClusterUiError),
    TopsisDone(TopsisResult),
    McdmDone(McdmResult),
    AhpDone(AhpResult),
    EntropyDone(EntropyResult),
    PdpDone {
        param: String,
        objective: String,
        model_type: String,
        result: PdpResult,
    },
    Pdp2dDone(PdpResult2d),
    DownsampleDone {
        key: DownsampleKey,
        indices: Vec<u32>,
    },
    LiveUpdateDone {
        new_trial_count: usize,
        pareto_updated: bool,
        new_indices: Vec<u32>,
    },
    HvHistoryDone {
        trial_ids: Vec<u32>,
        hv_values: Vec<f64>,
        sample_step: usize,
    },
    Error(String),
    SensitivityError(String),

    // ── TASK-2112: 新規バリアント ────────────────────────────────────
    /// REQ-001: Trade-off Navigator 計算完了
    TradeoffDone {
        sorted_indices: Vec<u32>,
    },
    /// REQ-006: 比較 Study のロード完了
    ComparisonStudyLoaded {
        study_idx: usize,
        context: Box<StudyContext>,
    },
    /// REQ-007: Artifacts ディレクトリスキャン完了
    ArtifactsDirScanned {
        trial_artifacts: std::collections::HashMap<u32, Vec<std::path::PathBuf>>,
        artifacts_dir: std::path::PathBuf,
    },
    /// REQ-005: HTML レポート生成完了
    HtmlReportDone {
        html: String,
        suggested_filename: String,
    },
    /// TASK-1505: MCDM散布図計算完了
    McdmScatterComputed {
        /// 表示ポイント (x_norm, y_norm, r, g, b)
        points: Vec<(f64, f64, u8, u8, u8)>,
        total_trials: usize,
    },
    /// TASK-1505: MCDM散布図計算失敗
    McdmScatterComputeFailed(String),
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
}
