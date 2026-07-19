pub fn compute_improvement_rate(history: &[(u32, f64)], last_n: usize, is_minimize: bool) -> f64 {
    let window: Vec<_> = history.iter().rev().take(last_n).collect();
    if window.len() < 2 {
        return 0.0;
    }
    // Seed the running best from the best value observed *before* the window so
    // the window's first element counts as an improvement only if it actually
    // beats the pre-window best. When the window covers the whole history the
    // prefix is empty and the seed stays at ±INF, so the first trial still
    // counts as the initial improvement (preserving the full-history semantics).
    let start = history.len() - window.len();
    let mut best_so_far = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for &(_, val) in &history[..start] {
        let improved = if is_minimize {
            val < best_so_far
        } else {
            val > best_so_far
        };
        if improved {
            best_so_far = val;
        }
    }
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
        // Regression test verifying that improvements in the maximize direction are counted
        // correctly for a non-decreasing sequence like the one build_best_trial_history(..., false)
        // produces.
        let ids = vec![0u32, 1, 2, 3];
        let vals = vec![1.0f64, 0.5, 2.0, 2.0];
        let history = build_best_trial_history(&ids, &vals, false);
        assert_eq!(history, vec![(0, 1.0), (1, 1.0), (2, 2.0), (3, 2.0)]);

        let rate = compute_improvement_rate(&history, 100, false);
        // Two improvements: id=0 (initial) and id=2 (1.0 -> 2.0); window length is 4.
        assert_eq!(rate, 2.0 / 4.0);
    }

    #[test]
    fn improvement_rate_minimize_direction_mismatch_detects_no_improvement() {
        // Verify that misinterpreting a history that improves in the maximize direction as
        // minimize causes improvements to go undetected (regression guard for the A4 bug).
        let history = vec![(0u32, 1.0_f64), (1, 1.0), (2, 2.0), (3, 2.0)];
        let rate_as_minimize = compute_improvement_rate(&history, 100, true);
        let rate_as_maximize = compute_improvement_rate(&history, 100, false);
        assert_eq!(rate_as_minimize, 1.0 / 4.0);
        assert_eq!(rate_as_maximize, 2.0 / 4.0);
    }

    #[test]
    fn improvement_rate_windowed_does_not_overcount_flat_tail() {
        // 150-trial best-so-far history that improves early then stays flat.
        // The last 100 values are all identical (no real improvement in the
        // window), so the rate over the last 100 must be exactly 0.0. The window
        // starts mid-history, which used to spuriously count its first element.
        let mut history: Vec<(u32, f64)> = Vec::new();
        for i in 0..50u32 {
            history.push((i, 100.0 - i as f64)); // improving: 100, 99, ... 51
        }
        for i in 50..150u32 {
            history.push((i, 51.0)); // flat tail, no improvement
        }
        let rate = compute_improvement_rate(&history, 100, true);
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn improvement_rate_windowed_counts_only_in_window_improvements() {
        // Flat prefix, then a single improvement inside the last-100 window.
        let mut history: Vec<(u32, f64)> = Vec::new();
        for i in 0..60u32 {
            history.push((i, 10.0)); // flat prefix
        }
        for i in 60..160u32 {
            // one improvement at the very end of the window
            let v = if i == 159 { 9.0 } else { 10.0 };
            history.push((i, v));
        }
        // Window = last 100 (trials 60..160). Only trial 159 improves on the
        // pre-window best of 10.0 → exactly one improvement out of 100.
        let rate = compute_improvement_rate(&history, 100, true);
        assert_eq!(rate, 1.0 / 100.0);
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
