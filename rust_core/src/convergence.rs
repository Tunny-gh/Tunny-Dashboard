pub fn compute_improvement_rate(history: &[(u32, f64)], last_n: usize, is_minimize: bool) -> f64 {
    let window: Vec<_> = history.iter().rev().take(last_n).collect();
    if window.len() < 2 {
        return 0.0;
    }
    let mut best_so_far = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    let mut improved_count = 0usize;
    for &&(_, val) in window.iter().rev() {
        let improved = if is_minimize {
            val < best_so_far
        } else {
            val > best_so_far
        };
        if improved {
            best_so_far = val;
            improved_count += 1;
        }
    }
    (improved_count as f64) / (window.len() as f64)
}

pub fn build_best_trial_history(
    trial_ids: &[u32],
    objective_values: &[f64],
    is_minimize: bool,
) -> Vec<(u32, f64)> {
    let mut history = Vec::with_capacity(trial_ids.len());
    let mut best = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for (&id, &val) in trial_ids.iter().zip(objective_values.iter()) {
        let improved = if is_minimize { val < best } else { val > best };
        if improved {
            best = val;
        }
        history.push((id, best));
    }
    history
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn improvement_rate_all_improving() {
        let history = vec![(0u32, 1.0_f64), (1, 0.8), (2, 0.5)];
        let rate = compute_improvement_rate(&history, 100, true);
        assert!(rate > 0.0);
    }

    #[test]
    fn improvement_rate_empty_returns_zero() {
        let rate = compute_improvement_rate(&[], 100, true);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn improvement_rate_single_returns_zero() {
        let history = vec![(0u32, 1.0_f64)];
        let rate = compute_improvement_rate(&history, 100, true);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn improvement_rate_maximize_counts_non_decreasing_steps() {
        // build_best_trial_history(..., false) が生成するような非減少列で
        // maximize 方向の改善が正しくカウントされることを確認する回帰テスト。
        let ids = vec![0u32, 1, 2, 3];
        let vals = vec![1.0f64, 0.5, 2.0, 2.0];
        let history = build_best_trial_history(&ids, &vals, false);
        assert_eq!(history, vec![(0, 1.0), (1, 1.0), (2, 2.0), (3, 2.0)]);

        let rate = compute_improvement_rate(&history, 100, false);
        // 改善は id=0 (初回) と id=2 (1.0 -> 2.0) の2回、window長は4。
        assert_eq!(rate, 2.0 / 4.0);
    }

    #[test]
    fn improvement_rate_minimize_direction_mismatch_detects_no_improvement() {
        // maximize 方向で改善している履歴を誤って minimize として解釈すると
        // 改善が検出されなくなることを確認する（A4 バグの再発防止）。
        let history = vec![(0u32, 1.0_f64), (1, 1.0), (2, 2.0), (3, 2.0)];
        let rate_as_minimize = compute_improvement_rate(&history, 100, true);
        let rate_as_maximize = compute_improvement_rate(&history, 100, false);
        assert_eq!(rate_as_minimize, 1.0 / 4.0);
        assert_eq!(rate_as_maximize, 2.0 / 4.0);
    }

    #[test]
    fn build_best_trial_history_minimize() {
        let ids = vec![0u32, 1, 2];
        let vals = vec![1.0f64, 0.5, 0.8];
        let result = build_best_trial_history(&ids, &vals, true);
        assert_eq!(result, vec![(0, 1.0), (1, 0.5), (2, 0.5)]);
    }

    #[test]
    fn build_best_trial_history_maximize() {
        let ids = vec![0u32, 1, 2];
        let vals = vec![1.0f64, 0.5, 2.0];
        let result = build_best_trial_history(&ids, &vals, false);
        assert_eq!(result, vec![(0, 1.0), (1, 1.0), (2, 2.0)]);
    }

    #[test]
    fn build_best_trial_history_empty() {
        let result = build_best_trial_history(&[], &[], true);
        assert!(result.is_empty());
    }
}
