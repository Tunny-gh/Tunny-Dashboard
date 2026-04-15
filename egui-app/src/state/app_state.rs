use std::collections::HashMap;

// ============================================================
// 基本型定義
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    Minimize,
    Maximize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrialState {
    Complete,
    Running,
    Pruned,
    Fail,
    Waiting,
}

#[derive(Debug, Clone)]
pub struct StudyMeta {
    pub study_id: u32,
    pub name: String,
    pub directions: Vec<Direction>,
    pub completed_trials: usize,
    pub total_trials: usize,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_names: Vec<String>,
    pub has_constraints: bool,
}

#[derive(Debug, Clone)]
pub struct TrialRow {
    pub trial_id: u32,
    pub params: HashMap<String, f64>,
    pub objectives: Vec<f64>,
    pub pareto_rank: u32,
    pub cluster_id: Option<i32>,
    pub state: TrialState,
    pub user_attrs: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct GpuBufferData {
    pub positions: Vec<f32>,
    pub positions3d: Vec<f32>,
    pub colors: Vec<f32>,
    pub sizes: Vec<f32>,
    pub trial_count: u32,
}

#[derive(Debug, Clone)]
pub struct StudyContext {
    pub meta: StudyMeta,
    pub trial_rows: Vec<TrialRow>,
    pub gpu_data: GpuBufferData,
    pub pareto_indices: Vec<u32>,
}

impl StudyContext {
    /// パラメータのデータ範囲 [min, max] を返す（データがない場合は [0.0, 1.0]）
    pub fn param_range(&self, param_name: &str) -> (f64, f64) {
        let values: Vec<f64> = self
            .trial_rows
            .iter()
            .filter_map(|r| r.params.get(param_name).copied())
            .collect();
        if values.is_empty() {
            return (0.0, 1.0);
        }
        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorMode {
    ParetoRank,
    ObjectiveValue(String),
    TrialNumber,
    ClusterId,
}

impl ColorMode {
    pub fn label(&self) -> &str {
        match self {
            ColorMode::ParetoRank => "Pareto Rank",
            ColorMode::ObjectiveValue(_) => "Objective",
            ColorMode::TrialNumber => "Trial Number",
            ColorMode::ClusterId => "Cluster ID",
        }
    }
}

// ============================================================
// 分析結果型（ placeholder - 詳細は後続タスクで充実させる）
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

// ============================================================
// ダウンサンプリングキャッシュ
// ============================================================

#[derive(Debug, Clone, Default)]
pub struct DownsampleCache {
    pub scatter: Option<Vec<u32>>,
    pub pcp: Option<Vec<u32>>,
    pub thumbnail: Option<Vec<u32>>,
    pub hover: Option<Vec<u32>>,
}

impl DownsampleCache {
    pub fn clear(&mut self) {
        self.scatter = None;
        self.pcp = None;
        self.thumbnail = None;
        self.hover = None;
    }
}

/// 選択率の変化が再サンプリングをトリガーすべきか判定する
pub fn should_resample(current_rate: f64, last_rate: f64) -> bool {
    (current_rate - last_rate).abs() > 0.20
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
// AppState
// ============================================================

/// Hypervolume 推移データ
#[derive(Debug, Clone)]
pub struct HvHistory {
    pub trial_ids: Vec<u32>,
    pub hv_values: Vec<f64>,
}

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

    /// パラメータのフィルター範囲を設定し、selected_indices を更新する
    pub fn set_filter(&mut self, param: &str, min: f64, max: f64) {
        self.filter_ranges.insert(param.to_string(), (min, max));
        self.apply_filters();
    }

    /// パラメータのフィルターを除去し、selected_indices を更新する
    pub fn remove_filter(&mut self, param: &str) {
        self.filter_ranges.remove(param);
        self.apply_filters();
    }

    /// 全フィルターをクリアして全 Trial を選択状態にする
    pub fn clear_filters(&mut self) {
        self.filter_ranges.clear();
        if let Some(ctx) = &self.current_study {
            self.selected_indices = ctx.trial_rows.iter().map(|r| r.trial_id).collect();
        }
    }

    /// グラフ上のドラッグ選択で selected_indices を直接上書きする（フィルターには影響しない）
    pub fn brush_select(&mut self, indices: Vec<u32>) {
        self.selected_indices = indices;
    }

    /// filter_ranges に基づいて selected_indices を再計算する
    fn apply_filters(&mut self) {
        if let Some(ctx) = &self.current_study {
            if self.filter_ranges.is_empty() {
                self.selected_indices = ctx.trial_rows.iter().map(|r| r.trial_id).collect();
                return;
            }
            // filter_ranges のクローンを使ってボローを回避
            let ranges = self.filter_ranges.clone();
            let trial_rows = &ctx.trial_rows;
            self.selected_indices = trial_rows
                .iter()
                .filter(|row| {
                    ranges.iter().all(|(param, (min, max))| {
                        if let Some(&val) = row.params.get(param) {
                            val >= *min && val <= *max
                        } else {
                            true // パラメータが存在しない Trial は除外しない
                        }
                    })
                })
                .map(|r| r.trial_id)
                .collect();
        }
    }

    /// ライブ更新で新規 Trial を追記する（フィルター・選択状態を変更しない）
    pub fn apply_live_update(&mut self, new_trials: Vec<TrialRow>) {
        if let Some(ctx) = &mut self.current_study {
            ctx.trial_rows.extend(new_trials);
            // filter_ranges, selected_indices は変更しない (REQ-134)
        }
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

    #[test]
    fn downsample_cache_clear() {
        let mut cache = DownsampleCache {
            scatter: Some(vec![0, 1, 2]),
            pcp: Some(vec![3, 4]),
            thumbnail: Some(vec![5]),
            hover: Some(vec![6]),
        };
        cache.clear();
        assert!(cache.scatter.is_none());
        assert!(cache.pcp.is_none());
        assert!(cache.thumbnail.is_none());
        assert!(cache.hover.is_none());
    }

    // TASK-2032 performance tests

    #[test]
    fn filter_performance_5k_trials_under_5ms() {
        // Generate 5000 trials
        let trial_rows: Vec<TrialRow> = (0u32..5000)
            .map(|i| {
                let mut params = HashMap::new();
                params.insert("x".to_string(), (i as f64) / 5000.0);
                params.insert("y".to_string(), (i as f64 % 100.0) / 100.0);
                TrialRow {
                    trial_id: i,
                    params,
                    objectives: vec![i as f64],
                    pareto_rank: 0,
                    cluster_id: None,
                    state: TrialState::Complete,
                    user_attrs: HashMap::new(),
                }
            })
            .collect();

        let mut filter_ranges = HashMap::new();
        filter_ranges.insert("x".to_string(), (0.2, 0.8));
        filter_ranges.insert("y".to_string(), (0.1, 0.9));

        let start = std::time::Instant::now();
        let _selected: Vec<u32> = trial_rows
            .iter()
            .filter(|row| {
                filter_ranges.iter().all(|(param, (min, max))| {
                    if let Some(&val) = row.params.get(param) {
                        val >= *min && val <= *max
                    } else {
                        true
                    }
                })
            })
            .map(|r| r.trial_id)
            .collect();
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 5,
            "filter took {}ms (expected < 5ms)",
            elapsed.as_millis()
        );
    }

    // TASK-2027 tests

    #[test]
    fn apply_live_update_increases_trial_count() {
        let mut state = AppState::new();
        state.current_study = Some(make_study_ctx_with_params());
        let initial_count = state.current_study.as_ref().unwrap().trial_rows.len();
        let new_trial = TrialRow {
            trial_id: 99,
            params: HashMap::new(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        };
        state.apply_live_update(vec![new_trial]);
        assert_eq!(
            state.current_study.as_ref().unwrap().trial_rows.len(),
            initial_count + 1
        );
    }

    #[test]
    fn apply_live_update_does_not_change_filter_ranges() {
        let mut state = AppState::new();
        state.current_study = Some(make_study_ctx_with_params());
        state.filter_ranges.insert("x".to_string(), (0.1, 0.9));
        let new_trial = TrialRow {
            trial_id: 99,
            params: HashMap::new(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        };
        state.apply_live_update(vec![new_trial]);
        // filter_ranges must be unchanged
        assert_eq!(state.filter_ranges.get("x"), Some(&(0.1, 0.9)));
    }

    // TASK-2026 tests

    #[test]
    fn should_resample_small_change_no_trigger() {
        // 10% change → no trigger
        assert!(!should_resample(0.6, 0.7));
    }

    #[test]
    fn should_resample_large_change_triggers() {
        // 25% change → triggers
        assert!(should_resample(0.5, 0.75));
    }

    #[test]
    fn should_resample_exactly_20_percent_no_trigger() {
        // exactly 20% is not > 0.20
        assert!(!should_resample(0.5, 0.7));
    }

    #[test]
    fn should_resample_negative_direction_triggers() {
        // selection drops by 25%
        assert!(should_resample(0.75, 0.5));
    }

    #[test]
    fn color_mode_variants() {
        let mode = ColorMode::ObjectiveValue("obj0".to_string());
        assert_ne!(mode, ColorMode::ParetoRank);
        assert_ne!(mode, ColorMode::TrialNumber);
    }

    #[test]
    fn live_update_state_defaults() {
        let state = LiveUpdateState::default();
        assert!(!state.enabled);
        assert!(state.file_path.is_none());
        assert_eq!(state.last_byte_offset, 0);
        assert_eq!(state.interval_ms, 2000);
    }

    fn make_study_ctx_with_params() -> StudyContext {
        let mut params0 = HashMap::new();
        params0.insert("x".to_string(), 0.2);
        let mut params1 = HashMap::new();
        params1.insert("x".to_string(), 0.6);
        let mut params2 = HashMap::new();
        params2.insert("x".to_string(), 0.9);
        let trial_rows = vec![
            TrialRow {
                trial_id: 0,
                params: params0,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 1,
                params: params1,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 2,
                params: params2,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
        ];
        StudyContext {
            meta: StudyMeta {
                study_id: 0,
                name: "test".to_string(),
                directions: vec![Direction::Minimize],
                completed_trials: 3,
                total_trials: 3,
                param_names: vec!["x".to_string()],
                objective_names: vec![],
                user_attr_names: vec![],
                has_constraints: false,
            },
            trial_rows,
            gpu_data: GpuBufferData {
                positions: vec![],
                positions3d: vec![],
                colors: vec![],
                sizes: vec![],
                trial_count: 3,
            },
            pareto_indices: vec![],
        }
    }

    #[test]
    fn set_filter_excludes_out_of_range_trials() {
        let mut state = AppState::new();
        state.current_study = Some(make_study_ctx_with_params());
        state.set_filter("x", 0.0, 0.5);
        // x=0.2 → in range (trial_id=0), x=0.6 → out, x=0.9 → out
        assert!(state.selected_indices.contains(&0));
        assert!(!state.selected_indices.contains(&1));
        assert!(!state.selected_indices.contains(&2));
    }

    #[test]
    fn remove_filter_restores_all_trials() {
        let mut state = AppState::new();
        state.current_study = Some(make_study_ctx_with_params());
        state.set_filter("x", 0.0, 0.5);
        assert_eq!(state.selected_indices.len(), 1);
        state.remove_filter("x");
        assert_eq!(state.selected_indices.len(), 3);
    }

    #[test]
    fn clear_filters_selects_all_trials() {
        let mut state = AppState::new();
        state.current_study = Some(make_study_ctx_with_params());
        state.set_filter("x", 0.0, 0.1);
        assert!(state.selected_indices.is_empty());
        state.clear_filters();
        assert_eq!(state.selected_indices.len(), 3);
    }

    #[test]
    fn brush_select_updates_selected_indices() {
        let mut state = AppState::new();
        state.brush_select(vec![1, 3, 5]);
        assert_eq!(state.selected_indices, vec![1, 3, 5]);
    }

    #[test]
    fn set_filter_then_remove_restores_all() {
        let mut state = AppState::new();
        state.current_study = Some(make_study_ctx_with_params());
        state.set_filter("x", 0.5, 1.0);
        state.remove_filter("x");
        // 全件選択に戻る
        assert_eq!(state.selected_indices.len(), 3);
    }
}
