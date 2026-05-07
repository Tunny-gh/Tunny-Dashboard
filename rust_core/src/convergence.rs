pub fn compute_improvement_rate(history: &[(u32, f64)], last_n: usize) -> f64 {
    let window: Vec<_> = history.iter().rev().take(last_n).collect();
    if window.len() < 2 {
        return 0.0;
    }
    let mut best_so_far = f64::INFINITY;
    let mut improved_count = 0usize;
    for &&(_, val) in window.iter().rev() {
        if val < best_so_far {
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
        let rate = compute_improvement_rate(&history, 100);
        assert!(rate > 0.0);
    }

    #[test]
    fn improvement_rate_empty_returns_zero() {
        let rate = compute_improvement_rate(&[], 100);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn improvement_rate_single_returns_zero() {
        let history = vec![(0u32, 1.0_f64)];
        let rate = compute_improvement_rate(&history, 100);
        assert_eq!(rate, 0.0);
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
