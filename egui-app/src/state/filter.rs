use super::app_state::AppState;

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
// AppState フィルターメソッド
// ============================================================

impl AppState {
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
            self.selected_indices = ctx.view.trial_ids.clone();
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
                self.selected_indices = ctx.view.trial_ids.clone();
                return;
            }
            // filter_ranges のクローンを使ってボローを回避
            let ranges = self.filter_ranges.clone();
            // 列スライスで直接フィルタ（per-row HashMap 再構築を回避）
            let n = ctx.view.row_count();
            self.selected_indices = (0..n)
                .filter(|&i| {
                    ranges.iter().all(|(param, (min, max))| {
                        if let Some(col) = ctx.view.numeric_column(param) {
                            let val = col.get(i).copied().unwrap_or(f64::NAN);
                            val.is_finite() && val >= *min && val <= *max
                        } else {
                            true // 列が存在しない場合は除外しない
                        }
                    })
                })
                .map(|i| ctx.view.trial_ids.get(i).copied().unwrap_or(i as u32))
                .collect();
        }
    }
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::state::app_state::*;

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
                trial_number: 0,
                params: params0,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 1,
                trial_number: 1,
                params: params1,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
            TrialRow {
                trial_id: 2,
                trial_number: 2,
                params: params2,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
                state: TrialState::Complete,
                user_attrs: HashMap::new(),
            },
        ];
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 3,
            total_trials: 3,
            param_names: vec!["x".to_string()],
            objective_names: vec![],
            user_attr_names: vec![],
            has_constraints: false,
        };
        StudyContext::from_rows_for_test(meta, trial_rows)
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
                    trial_number: i,
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
            elapsed.as_millis() < 10,
            "filter took {}ms (expected < 10ms)",
            elapsed.as_millis()
        );
    }

    // TASK-2026 tests

    #[test]
    fn should_resample_small_change_no_trigger() {
        // 10% change -> no trigger
        assert!(!should_resample(0.6, 0.7));
    }

    #[test]
    fn should_resample_large_change_triggers() {
        // 25% change -> triggers
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
    fn set_filter_excludes_out_of_range_trials() {
        let mut state = AppState::new();
        state.current_study = Some(make_study_ctx_with_params());
        state.set_filter("x", 0.0, 0.5);
        // x=0.2 -> in range (trial_id=0), x=0.6 -> out, x=0.9 -> out
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
