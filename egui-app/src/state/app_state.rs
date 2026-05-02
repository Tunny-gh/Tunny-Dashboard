pub use super::filter::*;
pub use super::results::AhpResult;
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
    pub importance_cache: HashMap<(u8, usize), SensitivityResult>,
    pub sobol_cache: HashMap<usize, SobolResult>,
    pub cluster_result: Option<ClusterResult>,
    pub downsample_cache: DownsampleCache,
    pub live_update: LiveUpdateState,
    pub topsis_result: Option<TopsisResult>,
    pub mcdm_result: Option<McdmResult>,
    pub ahp_result: Option<AhpResult>,
    pub hv_history: Option<HvHistory>,
    pub selected_colormap: ColormapName,
    pub chart_colors: Vec<egui::Color32>,

    // ── REQ-001: Trade-off Navigator ──────────────────────────
    /// 目的関数ごとの重みベクトル（スライダー値）
    pub tradeoff_weights: Vec<f64>,
    /// score_tradeoff_navigator() の結果（ソート済みインデックス）
    pub tradeoff_sorted_indices: Option<Vec<u32>>,

    // ── REQ-006: Multi-study 比較 ──────────────────────────────
    /// 比較モードが有効か
    pub comparison_mode: bool,
    /// 比較対象の StudyContext リスト（最大 4 件）
    pub comparison_studies: Vec<StudyContext>,
    /// 比較スタディの色リスト
    pub comparison_colors: Vec<egui::Color32>,

    // ── REQ-007: Artifacts ────────────────────────────────────
    /// スキャン済みの artifacts ベースディレクトリ
    pub artifacts_dir: Option<std::path::PathBuf>,
    /// trial_id → ファイルパスリストのマップ
    pub artifact_map: HashMap<u32, Vec<std::path::PathBuf>>,

    // ── REQ-008: 収束診断 ──────────────────────────────────────
    /// (trial_id, cumulative_best_value) の履歴（単目的 Study のみ）
    pub best_trial_history: Option<Vec<(u32, f64)>>,
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
            importance_cache: HashMap::new(),
            sobol_cache: HashMap::new(),
            cluster_result: None,
            downsample_cache: DownsampleCache::default(),
            live_update: LiveUpdateState::default(),
            topsis_result: None,
            mcdm_result: None,
            ahp_result: None,
            hv_history: None,
            selected_colormap: ColormapName::Viridis,
            chart_colors: Vec::new(),
            tradeoff_weights: Vec::new(),
            tradeoff_sorted_indices: None,
            comparison_mode: false,
            comparison_studies: Vec::new(),
            comparison_colors: Vec::new(),
            artifacts_dir: None,
            artifact_map: HashMap::new(),
            best_trial_history: None,
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
        self.importance_cache.clear();
        self.sobol_cache.clear();
        self.cluster_result = None;
        self.topsis_result = None;
        self.mcdm_result = None;
        self.ahp_result = None;
        self.hv_history = None;
        self.downsample_cache.clear();
        self.chart_colors.clear();
        // selected_colormap はユーザー設定を維持

        // REQ-001: Trade-off 結果はリセット（Study 切り替え時に再計算が必要）
        self.tradeoff_weights.clear();
        self.tradeoff_sorted_indices = None;

        // REQ-006: comparison_mode/studies/colors は Study 切り替えでも維持
        // （ユーザーが明示的にリセットするまで比較セッションを保持）

        // REQ-007: Artifacts は Study 切り替え時にリセット
        self.artifacts_dir = None;
        self.artifact_map.clear();

        // REQ-008: 収束履歴は Study 切り替え時にリセット
        self.best_trial_history = None;
    }

    /// ColorMode と ColormapName に基づいて chart_colors を即時再計算する
    pub fn update_chart_colors(&mut self) {
        if let Some(ctx) = &self.current_study {
            let color_mode = self.color_mode.clone();
            let colormap_name = self.selected_colormap.clone();
            let trial_rows = &ctx.trial_rows;
            let objective_names = &ctx.meta.objective_names;
            let mcdm_scores = self.mcdm_result.as_ref().map(|r| r.primary_scores());
            self.chart_colors = crate::render::colormap::compute_chart_colors(
                &color_mode,
                &colormap_name,
                trial_rows,
                objective_names,
                mcdm_scores,
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
        assert!(state.importance_cache.is_empty());
        assert!(state.cluster_result.is_none());
    }

    #[test]
    fn app_state_clear_resets_selection() {
        let mut state = AppState::new();
        state.selected_indices = vec![0, 1, 2];
        state.filter_ranges.insert("x".to_string(), (0.0, 1.0));
        state.highlighted_trial = Some(5);
        state.importance_cache.insert(
            (0u8, 0),
            SensitivityResult {
                param_names: vec!["x".to_string()],
                objective_names: vec!["y".to_string()],
                spearman: vec![vec![0.9]],
                ridge: vec![],
                rf_anova: None,
                mdi: None,
                shap: None,
                permutation: None,
            },
        );

        state.clear();

        assert!(state.selected_indices.is_empty());
        assert!(state.filter_ranges.is_empty());
        assert!(state.highlighted_trial.is_none());
        assert!(state.importance_cache.is_empty());
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

    // ============================================================
    // TASK-2110: 8 新規フィールドのテスト
    // ============================================================

    #[test]
    fn task2110_tradeoff_fields_default() {
        // TC-001, TC-002
        let state = AppState::new();
        assert!(state.tradeoff_weights.is_empty());
        assert!(state.tradeoff_sorted_indices.is_none());
    }

    #[test]
    fn task2110_comparison_fields_default() {
        // TC-003, TC-004, TC-005
        let state = AppState::new();
        assert!(!state.comparison_mode);
        assert!(state.comparison_studies.is_empty());
        assert!(state.comparison_colors.is_empty());
    }

    #[test]
    fn task2110_artifacts_fields_default() {
        // TC-006, TC-007
        let state = AppState::new();
        assert!(state.artifacts_dir.is_none());
        assert!(state.artifact_map.is_empty());
    }

    #[test]
    fn task2110_best_trial_history_default() {
        // TC-008
        let state = AppState::new();
        assert!(state.best_trial_history.is_none());
    }

    #[test]
    fn task2110_clear_resets_artifact_and_history_fields() {
        // TC-009, TC-010
        let mut state = AppState::new();
        state
            .artifact_map
            .insert(0, vec![std::path::PathBuf::from("/tmp/a.png")]);
        state.artifacts_dir = Some(std::path::PathBuf::from("/tmp"));
        state.best_trial_history = Some(vec![(0, 1.0)]);
        state.tradeoff_weights = vec![0.5, 0.5];
        state.tradeoff_sorted_indices = Some(vec![0, 1]);

        state.clear();

        assert!(state.artifact_map.is_empty());
        assert!(state.artifacts_dir.is_none());
        assert!(state.best_trial_history.is_none());
        assert!(state.tradeoff_weights.is_empty());
        assert!(state.tradeoff_sorted_indices.is_none());
    }

    #[test]
    fn task2110_clear_preserves_comparison_fields() {
        // comparison_mode/studies/colors は clear() でリセットしない
        let mut state = AppState::new();
        state.comparison_mode = true;
        state.comparison_colors = vec![egui::Color32::RED];

        state.clear();

        // comparison_mode と comparison_colors は維持される
        assert!(state.comparison_mode);
        assert_eq!(state.comparison_colors.len(), 1);
    }

    #[test]
    fn task2110_tradeoff_weights_writable() {
        // TC-011
        let mut state = AppState::new();
        state.tradeoff_weights = vec![0.5, 0.5];
        assert_eq!(state.tradeoff_weights, vec![0.5, 0.5]);
    }
}
