use crate::state::app_state::{
    ClusterResult, GpuBufferData, SensitivityResult, SobolResult, StudyMeta, TopsisResult, TrialRow,
};

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
    SensitivityDone(SensitivityResult),
    SobolDone(SobolResult),
    ClusteringDone(ClusterResult),
    TopsisDone(TopsisResult),
    PdpDone {
        param: String,
        objective: String,
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
    },
    Error(String),
    SensitivityError(String),
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
