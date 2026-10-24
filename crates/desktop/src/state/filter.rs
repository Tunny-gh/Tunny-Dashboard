use std::collections::HashMap;

use super::app_state::AppState;

// ============================================================
// AppState filter methods
// ============================================================

impl AppState {
    /// Sets the filter range for a parameter and updates selected_indices.
    pub fn set_filter(&mut self, param: &str, min: f64, max: f64) {
        self.filter_ranges.insert(param.to_string(), (min, max));
        self.apply_filters();
    }

    /// Recomputes selected_indices based on filter_ranges.
    ///
    /// The actual row filtering is delegated to
    /// `tdashboard_analysis::filter::filter_rows_permissive` (a "pass-through" behavior that does
    /// not exclude rows when a column does not exist).
    pub(crate) fn apply_filters(&mut self) {
        if let Some(ctx) = &self.current_study {
            if self.filter_ranges.is_empty() {
                self.selected_indices = ctx.view.trial_ids.clone();
                return;
            }
            let ranges: HashMap<String, tdashboard_analysis::filter::Range> = self
                .filter_ranges
                .iter()
                .map(|(param, &(min, max))| {
                    (
                        param.clone(),
                        tdashboard_analysis::filter::Range {
                            min: Some(min),
                            max: Some(max),
                        },
                    )
                })
                .collect();
            self.selected_indices =
                tdashboard_analysis::filter::filter_rows_permissive(&ctx.view.df, &ranges)
                    .into_iter()
                    .map(|i| ctx.view.trial_ids.get(i as usize).copied().unwrap_or(i))
                    .collect();
        }
    }
}

// ============================================================
// Tests
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
            },
            TrialRow {
                trial_id: 1,
                trial_number: 1,
                params: params1,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
            },
            TrialRow {
                trial_id: 2,
                trial_number: 2,
                params: params2,
                objectives: vec![],
                pareto_rank: 0,
                cluster_id: None,
            },
        ];
        let meta = StudyMeta {
            study_id: 0,
            name: "test".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 3,
            param_names: vec!["x".to_string()],
            objective_names: vec![],
            param_bounds: Default::default(),
        };
        StudyContext::from_rows_for_test(meta, trial_rows)
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
        assert_eq!(state.filter_ranges.get("x"), Some(&(0.0, 0.5)));
        state.set_filter("x", 0.2, 0.8);
        assert_eq!(state.filter_ranges.get("x"), Some(&(0.2, 0.8)));
        assert_eq!(state.selected_indices, [0, 1]);
    }
}
