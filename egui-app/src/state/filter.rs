use std::collections::HashMap;

use super::app_state::AppState;

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
    ///
    /// 実際の行フィルタは `tunny_core::filter::filter_rows_permissive` に委譲する
    /// （列が存在しない場合は除外しない「素通し」挙動）。
    fn apply_filters(&mut self) {
        if let Some(ctx) = &self.current_study {
            if self.filter_ranges.is_empty() {
                self.selected_indices = ctx.view.trial_ids.clone();
                return;
            }
            let ranges: HashMap<String, tunny_core::filter::Range> = self
                .filter_ranges
                .iter()
                .map(|(param, &(min, max))| {
                    (
                        param.clone(),
                        tunny_core::filter::Range {
                            min: Some(min),
                            max: Some(max),
                        },
                    )
                })
                .collect();
            self.selected_indices =
                tunny_core::filter::filter_rows_permissive(&ctx.view.df, &ranges)
                    .into_iter()
                    .map(|i| ctx.view.trial_ids.get(i as usize).copied().unwrap_or(i))
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
            param_bounds: Default::default(),
        };
        StudyContext::from_rows_for_test(meta, trial_rows)
    }

    // TASK-2032 performance tests

    #[test]
    fn filter_5k_trials_at_scale() {
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

        let selected: Vec<u32> = trial_rows
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

        // The range filter must keep some rows and exclude others.
        assert!(!selected.is_empty(), "filter excluded everything");
        assert!(
            selected.len() < trial_rows.len(),
            "filter kept all rows ({}), expected some to be excluded",
            selected.len()
        );
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
