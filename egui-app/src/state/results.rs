// ============================================================
// 分析結果型
// ============================================================

#[derive(Debug, Clone)]
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
    pub mdi: Option<MdiResult>,
    pub shap: Option<ShapResult>,
}

#[derive(Debug, Clone)]
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

#[derive(Debug, Clone)]
pub struct RfAnovaResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct MdiResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct ShapResult {
    pub importances: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub labels: Vec<i32>,
    pub n_clusters: usize,
}

#[derive(Debug, Clone)]
pub struct TopsisResult {
    pub scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub positive_ideal: Vec<f64>,
    pub negative_ideal: Vec<f64>,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McdmMethod {
    Topsis,
}

impl McdmMethod {
    pub fn label(&self) -> &'static str {
        match self {
            McdmMethod::Topsis => "TOPSIS",
        }
    }

    pub fn all() -> &'static [McdmMethod] {
        &[McdmMethod::Topsis]
    }
}

#[derive(Debug, Clone)]
pub enum McdmResult {
    Topsis(TopsisResult),
}

impl McdmResult {
    pub fn primary_scores(&self) -> &[f64] {
        match self {
            McdmResult::Topsis(r) => &r.scores,
        }
    }

    pub fn ranked_indices(&self) -> &[u32] {
        match self {
            McdmResult::Topsis(r) => &r.ranked_indices,
        }
    }

    pub fn duration_ms(&self) -> f64 {
        match self {
            McdmResult::Topsis(r) => r.duration_ms,
        }
    }

    pub fn method(&self) -> McdmMethod {
        match self {
            McdmResult::Topsis(_) => McdmMethod::Topsis,
        }
    }

    pub fn method_label(&self) -> &'static str {
        self.method().label()
    }
}

/// Hypervolume 推移データ
#[derive(Debug, Clone)]
pub struct HvHistory {
    pub trial_ids: Vec<u32>,
    pub hv_values: Vec<f64>,
    /// ダウンサンプリングのステップ幅（1 = 全点）
    pub sample_step: usize,
}

// ============================================================
// ライブ更新状態
// ============================================================

#[derive(Debug, Clone)]
pub struct LiveUpdateState {
    pub enabled: bool,
    pub file_path: Option<String>,
    pub last_byte_offset: u64,
    pub interval_ms: u64,
}

impl Default for LiveUpdateState {
    fn default() -> Self {
        Self {
            enabled: false,
            file_path: None,
            last_byte_offset: 0,
            interval_ms: 2000,
        }
    }
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_update_state_defaults() {
        let state = LiveUpdateState::default();
        assert!(!state.enabled);
        assert!(state.file_path.is_none());
        assert_eq!(state.last_byte_offset, 0);
        assert_eq!(state.interval_ms, 2000);
    }

    // McdmMethod tests
    #[test]
    fn mcdm_method_label() {
        assert_eq!(McdmMethod::Topsis.label(), "TOPSIS");
    }

    #[test]
    fn mcdm_method_all() {
        assert_eq!(McdmMethod::all(), &[McdmMethod::Topsis]);
    }

    // McdmResult tests
    #[test]
    fn mcdm_result_topsis_primary_scores() {
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.8, 0.6, 0.9],
            ranked_indices: vec![2, 0, 1],
            positive_ideal: vec![1.0],
            negative_ideal: vec![0.0],
            duration_ms: 12.5,
        });
        assert_eq!(result.primary_scores(), &[0.8, 0.6, 0.9]);
    }

    #[test]
    fn mcdm_result_topsis_ranked_indices() {
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.8, 0.6, 0.9],
            ranked_indices: vec![2, 0, 1],
            positive_ideal: vec![1.0],
            negative_ideal: vec![0.0],
            duration_ms: 12.5,
        });
        assert_eq!(result.ranked_indices(), &[2, 0, 1]);
    }

    #[test]
    fn mcdm_result_topsis_method_label() {
        let result = McdmResult::Topsis(TopsisResult {
            scores: vec![0.5],
            ranked_indices: vec![0],
            positive_ideal: vec![1.0],
            negative_ideal: vec![0.0],
            duration_ms: 1.0,
        });
        assert_eq!(result.method_label(), "TOPSIS");
    }

    #[test]
    fn topsis_result_all_fields() {
        let r = TopsisResult {
            scores: vec![0.9, 0.1],
            ranked_indices: vec![0, 1],
            positive_ideal: vec![0.5, 0.5],
            negative_ideal: vec![0.1, 0.1],
            duration_ms: 42.0,
        };
        assert_eq!(r.scores.len(), 2);
        assert_eq!(r.ranked_indices.len(), 2);
        assert_eq!(r.positive_ideal.len(), 2);
        assert_eq!(r.negative_ideal.len(), 2);
        assert!((r.duration_ms - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_result_wrap_topsis_integration() {
        let topsis = TopsisResult {
            scores: vec![0.8, 0.6, 0.9],
            ranked_indices: vec![2, 0, 1],
            positive_ideal: vec![1.0],
            negative_ideal: vec![0.0],
            duration_ms: 12.5,
        };
        let mcdm = McdmResult::Topsis(topsis);
        assert_eq!(mcdm.primary_scores(), &[0.8, 0.6, 0.9]);
        assert_eq!(mcdm.ranked_indices(), &[2, 0, 1]);
        assert!((mcdm.duration_ms() - 12.5).abs() < f64::EPSILON);
        assert_eq!(mcdm.method_label(), "TOPSIS");
    }
}
