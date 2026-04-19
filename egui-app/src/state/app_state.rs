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
    pub selected_colormap: ColormapName,
    pub chart_colors: Vec<egui::Color32>,
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
            selected_colormap: ColormapName::Viridis,
            chart_colors: Vec::new(),
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
        self.chart_colors.clear();
        // selected_colormap はユーザー設定を維持
    }

    /// ColorMode と ColormapName に基づいて chart_colors を即時再計算する
    pub fn update_chart_colors(&mut self) {
        if let Some(ctx) = &self.current_study {
            let color_mode = self.color_mode.clone();
            let colormap_name = self.selected_colormap.clone();
            let trial_rows = &ctx.trial_rows;
            let objective_names = &ctx.meta.objective_names;
            self.chart_colors = crate::render::colormap::compute_chart_colors(
                &color_mode,
                &colormap_name,
                trial_rows,
                objective_names,
            );
        } else {
            self.chart_colors.clear();
        }
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
            mdi: None,
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

    #[test]
    fn app_state_new_colormap_defaults() {
        let state = AppState::new();
        assert_eq!(state.selected_colormap, ColormapName::Viridis);
        assert!(state.chart_colors.is_empty());
    }

    fn make_simple_study_ctx() -> StudyContext {
        use std::collections::HashMap;
        StudyContext {
            meta: StudyMeta {
                study_id: 0,
                name: "test".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 2,
                total_trials: 2,
                param_names: vec!["x".to_string()],
                objective_names: vec![],
                user_attr_names: vec![],
                has_constraints: false,
            },
            trial_rows: vec![
                TrialRow {
                    trial_id: 0,
                    trial_number: 0,
                    params: HashMap::new(),
                    objectives: vec![],
                    pareto_rank: 0,
                    cluster_id: None,
                    state: TrialState::Complete,
                    user_attrs: HashMap::new(),
                },
                TrialRow {
                    trial_id: 1,
                    trial_number: 1,
                    params: HashMap::new(),
                    objectives: vec![],
                    pareto_rank: 1,
                    cluster_id: None,
                    state: TrialState::Complete,
                    user_attrs: HashMap::new(),
                },
            ],
            gpu_data: GpuBufferData {
                positions: vec![],
                positions3d: vec![],
                colors: vec![],
                sizes: vec![],
                trial_count: 2,
            },
            pareto_indices: vec![],
        }
    }

    #[test]
    fn update_chart_colors_with_study() {
        let mut state = AppState::new();
        state.current_study = Some(make_simple_study_ctx());
        state.update_chart_colors();
        assert!(!state.chart_colors.is_empty());
        assert_eq!(state.chart_colors.len(), 2);
    }

    #[test]
    fn update_chart_colors_without_study_clears() {
        let mut state = AppState::new();
        state.chart_colors = vec![egui::Color32::RED];
        state.update_chart_colors();
        assert!(state.chart_colors.is_empty());
    }

    #[test]
    fn clear_clears_chart_colors() {
        let mut state = AppState::new();
        state.current_study = Some(make_simple_study_ctx());
        state.update_chart_colors();
        assert!(!state.chart_colors.is_empty());
        state.clear();
        assert!(state.chart_colors.is_empty());
    }

    #[test]
    fn different_colormap_produces_different_colors() {
        let mut state = AppState::new();
        state.current_study = Some(make_simple_study_ctx());
        state.update_chart_colors();
        let viridis_colors = state.chart_colors.clone();
        state.selected_colormap = ColormapName::Jet;
        state.update_chart_colors();
        let jet_colors = state.chart_colors.clone();
        assert_ne!(viridis_colors, jet_colors);
    }
}
