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
}

#[derive(Debug, Clone)]
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

#[derive(Debug, Clone)]
pub struct RfAnovaResult {
    pub importances: Vec<Vec<f64>>,
}

#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
}

#[derive(Debug, Clone)]
pub struct ClusterResult {
    pub labels: Vec<i32>,
    pub n_clusters: usize,
}

#[derive(Debug, Clone)]
pub struct TopsisResult {
    pub scores: Vec<f64>,
    pub ranking: Vec<usize>,
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
}
