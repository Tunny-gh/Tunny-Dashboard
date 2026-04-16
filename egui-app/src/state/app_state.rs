pub use super::filter::*;
pub use super::results::*;
pub use super::types::*;

use std::collections::HashMap;

// ============================================================
// AppState
// ============================================================

#[derive(Debug)]
pub struct AppState {
    pub all_studies: Vec<StudyMeta>,
    pub journal_path: Option<std::path::PathBuf>,
    pub current_study: Option<StudyContext>,
    pub selected_indices: Vec<u32>,
    pub filter_ranges: HashMap<String, (f64, f64)>,
    pub highlighted_trial: Option<u32>,
    pub color_mode: ColorMode,
    pub sensitivity_result: Option<SensitivityResult>,
    pub sobol_result: Option<SobolResult>,
    pub cluster_result: Option<ClusterResult>,
    pub downsample_cache: DownsampleCache,
    pub live_update: LiveUpdateState,
    pub topsis_result: Option<TopsisResult>,
    pub hv_history: Option<HvHistory>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            all_studies: Vec::new(),
            journal_path: None,
            current_study: None,
            selected_indices: Vec::new(),
            filter_ranges: HashMap::new(),
            highlighted_trial: None,
            color_mode: ColorMode::ParetoRank,
            sensitivity_result: None,
            sobol_result: None,
            cluster_result: None,
            downsample_cache: DownsampleCache::default(),
            live_update: LiveUpdateState::default(),
            topsis_result: None,
            hv_history: None,
        }
    }

    /// ハイライト中の Trial を設定する
    pub fn set_highlight(&mut self, trial_id: u32) {
        self.highlighted_trial = Some(trial_id);
    }

    /// Study切り替え時にBrushing&Linking状態と分析結果をリセット
    pub fn clear(&mut self) {
        self.selected_indices.clear();
        self.filter_ranges.clear();
        self.highlighted_trial = None;
        self.sensitivity_result = None;
        self.sobol_result = None;
        self.cluster_result = None;
        self.topsis_result = None;
        self.hv_history = None;
        self.downsample_cache.clear();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_new_initial_values() {
        let state = AppState::new();
        assert!(state.all_studies.is_empty());
        assert!(state.current_study.is_none());
        assert!(state.selected_indices.is_empty());
        assert!(state.filter_ranges.is_empty());
        assert!(state.highlighted_trial.is_none());
        assert_eq!(state.color_mode, ColorMode::ParetoRank);
        assert!(state.sensitivity_result.is_none());
        assert!(state.cluster_result.is_none());
    }

    #[test]
    fn app_state_clear_resets_selection() {
        let mut state = AppState::new();
        state.selected_indices = vec![0, 1, 2];
        state.filter_ranges.insert("x".to_string(), (0.0, 1.0));
        state.highlighted_trial = Some(5);
        state.sensitivity_result = Some(SensitivityResult {
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            spearman: vec![vec![0.9]],
            ridge: vec![],
            rf_anova: None,
        });

        state.clear();

        assert!(state.selected_indices.is_empty());
        assert!(state.filter_ranges.is_empty());
        assert!(state.highlighted_trial.is_none());
        assert!(state.sensitivity_result.is_none());
    }

    #[test]
    fn app_state_clear_preserves_studies() {
        let mut state = AppState::new();
        state.all_studies.push(StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 10,
            total_trials: 10,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
        });

        state.clear();

        // Studies should NOT be cleared
        assert_eq!(state.all_studies.len(), 1);
    }
}
