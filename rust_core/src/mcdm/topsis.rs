/// TOPSIS (Technique for Order Preference by Similarity to Ideal Solution)
/// ranking computation for multi-objective optimization results.
///
/// TASK-1615: mode-frontier-features
/// Normalizes and weights each candidate (trial), computes its Euclidean distance
/// from the positive ideal solution (the best value per objective) and the negative
/// ideal solution (the worst value per objective), and adopts the relative closeness
/// to the negative ideal solution as the score.
use std::time::Instant;

/// TOPSIS computation result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopsisResult {
    /// TOPSIS score per trial (0 to 1; larger means closer to the ideal solution).
    pub scores: Vec<f64>,
    /// Trial indices sorted in descending order of score.
    pub ranked_indices: Vec<u32>,
    /// Positive ideal solution per objective (best value after weighted normalization).
    pub positive_ideal: Vec<f64>,
    /// Negative ideal solution per objective (worst value after weighted normalization).
    pub negative_ideal: Vec<f64>,
    /// Time taken for the computation (milliseconds).
    pub duration_ms: f64,
}

/// Computes the score and ranking of each trial using the TOPSIS method.
///
/// Steps:
/// 1. Validate the input.
/// 2. Normalize the weights.
/// 3. Exclude rows containing NaN/Inf, and perform vector normalization and weighting
///    only on the valid rows.
/// 4. Determine the positive and negative ideal solutions.
/// 5. Compute each trial's score from its distances to the positive and negative
///    ideal solutions.
///
/// When no valid trial exists, returns a uniform score of 0.5 for all trials.
/// A trial containing NaN/Inf gets a score of 0.0 and is placed at the bottom of the ranking.
pub fn compute_topsis(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
) -> Result<TopsisResult, String> {
    let start = Instant::now();

    // Validate the input.
    super::validate_inputs(values, n_trials, n_objectives, weights, is_minimize)?;

    // Weights are expected to sum to 1, but defend against callers that pass
    // unnormalized weights (or a degenerate sum) — mirrors VIKOR. TOPSIS scores
    // are invariant to a positive weight scale, so this only guards edge cases
    // (all-zero / NaN weights) and keeps the API symmetric across MCDM methods.
    let weights = super::normalize_weights(weights);
    let weights = weights.as_slice();

    let valid_indices = super::filter_valid_indices(values, n_trials, n_objectives);

    // If there are no valid trials at all, return a uniform score of 0.5.
    if valid_indices.is_empty() {
        return Ok(uniform_score_result(n_trials, n_objectives, 0.5, &start));
    }

    // Flat row-major matrix: avoids 50K separate heap allocations and improves cache locality.
    let n_valid = valid_indices.len();
    let weighted_matrix =
        build_weighted_matrix(values, n_objectives, weights, &valid_indices, n_valid);

    // Determine the positive and negative ideal solutions.
    let (positive_ideal, negative_ideal) =
        find_ideal_solutions(&weighted_matrix, n_valid, n_objectives, is_minimize);

    // Compute each trial's score.
    let valid_scores = compute_scores(
        &weighted_matrix,
        n_valid,
        n_objectives,
        &positive_ideal,
        &negative_ideal,
    );

    // Write the valid trials' scores back into the original trial indices
    // (invalid trials remain 0.0).
    let mut scores = vec![0.0_f64; n_trials];
    for (vi, &ti) in valid_indices.iter().enumerate() {
        scores[ti] = valid_scores[vi];
    }

    // Sort trial indices in descending order of score.
    let mut ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
    ranked_indices.sort_unstable_by(|&a, &b| {
        scores[b as usize]
            .partial_cmp(&scores[a as usize])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(TopsisResult {
        scores,
        ranked_indices,
        positive_ideal,
        negative_ideal,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

// =============================================================================
// Helper functions
// =============================================================================

/// Builds a result assigning a uniform score to all trials when no valid
/// trial exists. Both ideal solutions are the zero vector.
fn uniform_score_result(
    n_trials: usize,
    n_objectives: usize,
    score: f64,
    start: &Instant,
) -> TopsisResult {
    let scores = vec![score; n_trials];
    let mut ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
    ranked_indices.sort_by(|&a, &b| {
        scores[b as usize]
            .partial_cmp(&scores[a as usize])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    TopsisResult {
        scores,
        ranked_indices,
        positive_ideal: vec![0.0; n_objectives],
        negative_ideal: vec![0.0; n_objectives],
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    }
}

/// Build a flat (row-major) weighted normalized matrix.
///
/// r_ij = v_ij / sqrt(sum_i(v_ij^2))  → w_ij = weights[j] * r_ij
///
/// Using a flat Vec<f64> instead of Vec<Vec<f64>> avoids n_valid separate heap
/// allocations and gives contiguous memory access (better cache locality).
fn build_weighted_matrix(
    values: &[f64],
    n_objectives: usize,
    weights: &[f64],
    valid_indices: &[usize],
    n_valid: usize,
) -> Vec<f64> {
    // Compute the Euclidean norm for each objective (column).
    let mut col_norms = vec![0.0_f64; n_objectives];
    for &i in valid_indices {
        for j in 0..n_objectives {
            let v = values[i * n_objectives + j];
            col_norms[j] += v * v;
        }
    }
    for norm in col_norms.iter_mut() {
        *norm = norm.sqrt();
    }

    let mut matrix = vec![0.0_f64; n_valid * n_objectives];
    for (idx, &i) in valid_indices.iter().enumerate() {
        for j in 0..n_objectives {
            let v = values[i * n_objectives + j];
            let r = if col_norms[j].abs() < f64::EPSILON {
                0.0
            } else {
                v / col_norms[j]
            };
            matrix[idx * n_objectives + j] = r * weights[j];
        }
    }
    matrix
}

/// Determines the positive and negative ideal solutions for each objective
/// from the weighted normalized matrix.
///
/// Single row-major pass: cache-friendly, avoids multiple column scans.
fn find_ideal_solutions(
    weighted_matrix: &[f64],
    n_valid: usize,
    n_objectives: usize,
    is_minimize: &[bool],
) -> (Vec<f64>, Vec<f64>) {
    let mut col_min = vec![f64::INFINITY; n_objectives];
    let mut col_max = vec![f64::NEG_INFINITY; n_objectives];

    // Single pass over all rows (row-major, contiguous access).
    for i in 0..n_valid {
        let base = i * n_objectives;
        for j in 0..n_objectives {
            let v = weighted_matrix[base + j];
            if v < col_min[j] {
                col_min[j] = v;
            }
            if v > col_max[j] {
                col_max[j] = v;
            }
        }
    }

    let mut positive = vec![0.0_f64; n_objectives];
    let mut negative = vec![0.0_f64; n_objectives];
    for j in 0..n_objectives {
        (positive[j], negative[j]) = if is_minimize[j] {
            (col_min[j], col_max[j]) // minimize: positive ideal = min, negative ideal = max
        } else {
            (col_max[j], col_min[j]) // maximize: positive ideal = max, negative ideal = min
        };
    }
    (positive, negative)
}

/// Computes the TOPSIS score for each trial based on its Euclidean
/// distance from the positive and negative ideal solutions.
///
/// D+_i = sqrt(sum_j(w_ij - A+_j)^2)
/// D-_i = sqrt(sum_j(w_ij - A-_j)^2)
/// score_i = D-_i / (D+_i + D-_i) (0.5 when D+ + D- = 0)
fn compute_scores(
    weighted_matrix: &[f64],
    n_valid: usize,
    n_objectives: usize,
    positive_ideal: &[f64],
    negative_ideal: &[f64],
) -> Vec<f64> {
    (0..n_valid)
        .map(|i| {
            let base = i * n_objectives;
            let d_plus: f64 = (0..n_objectives)
                .map(|j| (weighted_matrix[base + j] - positive_ideal[j]).powi(2))
                .sum::<f64>()
                .sqrt();
            let d_minus: f64 = (0..n_objectives)
                .map(|j| (weighted_matrix[base + j] - negative_ideal[j]).powi(2))
                .sum::<f64>()
                .sqrt();
            let denom = d_plus + d_minus;
            if denom.abs() < f64::EPSILON {
                0.5
            } else {
                d_minus / denom
            }
        })
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Normal cases
    // -------------------------------------------------------------------------

    #[test]
    fn tc_1615_01_basic_two_obj_minimize() {
        // trial0: (1, 4)
        // trial1: (4, 1)
        // trial2: (2, 2)
        // Both objectives are minimize.

        let values = [1.0_f64, 4.0, 4.0, 1.0, 2.0, 2.0];
        let weights = [0.5_f64, 0.5];
        let is_minimize = [true, true];

        let result = compute_topsis(&values, 3, 2, &weights, &is_minimize);

        assert!(result.is_ok(), "計算は成功するはず");
        let r = result.unwrap();

        assert_eq!(r.ranked_indices.len(), 3);
        assert_eq!(r.scores.len(), 3);
        for &s in &r.scores {
            assert!(
                (0.0..=1.0).contains(&s),
                "スコアは 0〜1 の範囲であるべき: {}",
                s
            );
        }
        // The ranking should be in descending order of score.
        for i in 0..r.ranked_indices.len() - 1 {
            let idx_curr = r.ranked_indices[i] as usize;
            let idx_next = r.ranked_indices[i + 1] as usize;
            assert!(
                r.scores[idx_curr] >= r.scores[idx_next],
                "ランキングはスコア降順であるべき: scores[{}]={} >= scores[{}]={}",
                idx_curr,
                r.scores[idx_curr],
                idx_next,
                r.scores[idx_next]
            );
        }
    }

    #[test]
    fn tc_1615_02_maximize_direction() {
        // trial0: (1, 1)
        // trial1: (5, 1)
        // trial2: (5, 5)

        let values = [1.0_f64, 1.0, 5.0, 1.0, 5.0, 5.0];
        let weights = [0.7_f64, 0.3]; // weight obj0 more heavily
        let is_minimize = [false, true]; // obj0 is maximize

        let result = compute_topsis(&values, 3, 2, &weights, &is_minimize);

        assert!(result.is_ok());
        let r = result.unwrap();

        assert_eq!(
            r.ranked_indices[0], 1,
            "trial1 は obj0・obj1 とも最良なので 1 位のはず。ranked[0]={}",
            r.ranked_indices[0]
        );
        assert!(
            r.scores[0] < r.scores[1],
            "trial1 のスコアは trial0 より大きいはず: scores[0]={}, scores[1]={}",
            r.scores[0],
            r.scores[1]
        );
    }

    #[test]
    fn unnormalized_weights_match_normalized() {
        // Weights are normalized internally, so any positive scaling of the
        // same weight vector must produce identical scores and ranking.
        let values = [1.0_f64, 5.0, 3.0, 3.0, 5.0, 1.0];
        let a = compute_topsis(&values, 3, 2, &[0.3, 0.7], &[true, true]).unwrap();
        let b = compute_topsis(&values, 3, 2, &[3.0, 7.0], &[true, true]).unwrap();
        for i in 0..3 {
            assert!(
                (a.scores[i] - b.scores[i]).abs() < 1e-12,
                "scores must be invariant to weight scaling"
            );
        }
        assert_eq!(a.ranked_indices, b.ranked_indices);
    }

    #[test]
    fn tc_1615_03_weights_affect_ranking() {
        // Confirms that changing the weights changes the ranking.

        let values = [1.0_f64, 5.0, 5.0, 1.0]; // trial0:(1,5) trial1:(5,1)
        let is_minimize = [true, true];

        // Case where obj0 is weighted more heavily.
        let result_a = compute_topsis(&values, 2, 2, &[0.9, 0.1], &is_minimize).unwrap();
        // Case where obj1 is weighted more heavily.
        let result_b = compute_topsis(&values, 2, 2, &[0.1, 0.9], &is_minimize).unwrap();

        assert_eq!(
            result_a.ranked_indices[0], 0,
            "obj0 重視なら trial0 が 1 位のはず"
        );
        assert_eq!(
            result_b.ranked_indices[0], 1,
            "obj1 重視なら trial1 が 1 位のはず"
        );
    }

    #[test]
    fn tc_1615_04_single_trial() {
        // Boundary-value test for the case of a single trial.

        let values = [3.0_f64, 7.0];
        let result = compute_topsis(&values, 1, 2, &[0.5, 0.5], &[true, true]);

        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.scores.len(), 1);
        assert!(
            (r.scores[0] - 0.5).abs() < 1e-9,
            "1 trial のみの場合 score=0.5 のはず: {}",
            r.scores[0]
        );
        assert_eq!(r.ranked_indices, vec![0u32]);
    }

    #[test]
    fn tc_1615_05_single_objective() {
        // Boundary-value test for the case of a single objective.

        let values = [3.0_f64, 1.0, 2.0]; // 3 trials x 1 objective
        let result = compute_topsis(&values, 3, 1, &[1.0], &[true]);

        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.ranked_indices[0], 1, "最小値 1.0 の trial1 が 1 位のはず");
        assert_eq!(
            r.ranked_indices[2], 0,
            "最大値 3.0 の trial0 が最下位のはず"
        );
    }

    // -------------------------------------------------------------------------
    // Error cases
    // -------------------------------------------------------------------------

    #[test]
    fn tc_1615_06_zero_trials_error() {
        // n_trials=0 should be an error.

        let result = compute_topsis(&[], 0, 2, &[0.5, 0.5], &[true, true]);
        assert!(result.is_err(), "n_trials=0 はエラーになるはず");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("n_trials") || msg.contains("trial"),
            "エラーメッセージに n_trials が含まれるはず: {}",
            msg
        );
    }

    #[test]
    fn tc_1615_07_values_length_mismatch_error() {
        // Error when the length of values does not match n_trials * n_objectives.

        let result = compute_topsis(&[1.0, 2.0, 3.0], 2, 2, &[0.5, 0.5], &[true, true]);
        assert!(result.is_err(), "values 長不一致はエラーになるはず");
    }

    #[test]
    fn tc_1615_08_weights_length_mismatch_error() {
        // Error when the length of weights does not match n_objectives.

        let result = compute_topsis(&[1.0, 2.0, 3.0, 4.0], 2, 2, &[1.0], &[true, true]);
        assert!(result.is_err(), "weights 長不一致はエラーになるはず");
    }

    #[test]
    fn tc_1615_09_is_minimize_length_mismatch_error() {
        // Error when the length of is_minimize does not match n_objectives.

        let result = compute_topsis(&[1.0, 2.0, 3.0, 4.0], 2, 2, &[0.5, 0.5], &[true]);
        assert!(result.is_err(), "is_minimize 長不一致はエラーになるはず");
    }

    // -------------------------------------------------------------------------
    // Boundary values / edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn tc_1615_10_all_same_values_no_crash() {
        // Confirms it does not crash even when all trials have the same value.

        let values = [2.0_f64, 3.0, 2.0, 3.0, 2.0, 3.0]; // 3 trials x 2 objectives
        let result = compute_topsis(&values, 3, 2, &[0.5, 0.5], &[true, true]);

        assert!(result.is_ok(), "全 trial 同値でもエラーにならないはず");
        let r = result.unwrap();
        for &s in &r.scores {
            assert!(
                (s - 0.5).abs() < 1e-9,
                "全 trial 同値なら score=0.5 のはず: {}",
                s
            );
        }
    }

    #[test]
    fn tc_1615_11_nan_trial_ranked_last() {
        // Confirms that when trial1 contains NaN, it is excluded from the
        // computation and ranked last.

        let values = [1.0_f64, 1.0, f64::NAN, 1.0];
        let result = compute_topsis(&values, 2, 2, &[0.5, 0.5], &[true, true]);

        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.scores[1], 0.0, "NaN trial のスコアは 0.0 のはず");
        assert_eq!(
            *r.ranked_indices.last().unwrap(),
            1u32,
            "NaN trial は最下位にランクされるはず"
        );
    }

    #[test]
    fn tc_1615_12_performance_50k_trials() {
        // Debug builds omit optimizations; use a smaller dataset to keep the
        // assertion feasible without sacrificing meaningful coverage.
        #[cfg(debug_assertions)]
        let n_trials: usize = 500;
        #[cfg(not(debug_assertions))]
        let n_trials: usize = 50_000;

        let n_objectives: usize = 4;
        let values: Vec<f64> = (0..n_trials * n_objectives)
            .map(|i| (i % 100) as f64)
            .collect();
        let weights = [0.25_f64; 4];
        let is_minimize = [true; 4];

        let result = compute_topsis(&values, n_trials, n_objectives, &weights, &is_minimize);

        let result = result.expect("TOPSIS must succeed at scale");
        assert_eq!(result.ranked_indices.len(), n_trials);
    }

    #[test]
    fn tc_1615_13_ranked_indices_length() {
        // Confirms that the lengths of ranked_indices and scores match n_trials.

        let values: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 3 trials x 2 objectives
        let result = compute_topsis(&values, 3, 2, &[0.5, 0.5], &[true, true]).unwrap();

        assert_eq!(result.ranked_indices.len(), 3);
        assert_eq!(result.scores.len(), 3);
        // ranked_indices should be a permutation of each index.
        let mut sorted = result.ranked_indices.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec![0u32, 1, 2],
            "インデックス 0,1,2 が各 1 回ずつ含まれるはず"
        );
    }

    #[test]
    fn tc_1615_14_ideal_solutions_dimension() {
        // Confirms that the dimensions of positive_ideal / negative_ideal match n_objectives.

        let values: Vec<f64> = (0..9).map(|i| i as f64).collect(); // 3 trials x 3 objectives
        let result = compute_topsis(&values, 3, 3, &[1.0 / 3.0; 3], &[true; 3]).unwrap();

        assert_eq!(result.positive_ideal.len(), 3);
        assert_eq!(result.negative_ideal.len(), 3);
    }

    #[test]
    fn tc_1615_15_inf_trial_excluded_and_ranked_last() {
        // trial1 has +Inf in an objective -> must be excluded from computation
        // (same treatment as NaN) and ranked last without contaminating other scores.
        let values = [1.0_f64, 1.0, f64::INFINITY, 1.0];
        let result = compute_topsis(&values, 2, 2, &[0.5, 0.5], &[true, true]);

        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.scores[1], 0.0, "Inf trial score must be 0.0");
        assert_eq!(
            *r.ranked_indices.last().unwrap(),
            1u32,
            "Inf trial must be ranked last"
        );
    }

    #[test]
    fn tc_1616_01_two_trials_ranking() {
        // trial0: (1, 2) (both objectives are lowest)
        // trial1: (3, 4)
        // trial0 is favored since is_minimize.
        let values = [1.0_f64, 2.0, 3.0, 4.0];
        let result = compute_topsis(&values, 2, 2, &[0.5, 0.5], &[true, true]).unwrap();

        assert_eq!(result.ranked_indices[0], 0, "trial0 が 1 位のはず");
        assert!(
            result.scores[0] > result.scores[1],
            "trial0 のスコアは trial1 より大きいはず"
        );
    }

    // ---- TASK-2268: make build_weighted_matrix use a single allocation ----

    #[test]
    fn tc_2268_01_topsis_scores_match_after_single_alloc_refactor() {
        let values = [1.0_f64, 4.0, 4.0, 1.0, 2.0, 2.0];
        let weights = [0.5_f64, 0.5];
        let is_minimize = [true, true];
        let r = compute_topsis(&values, 3, 2, &weights, &is_minimize).unwrap();
        for (i, &s) in r.scores.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(&s),
                "score[{}]={} must be in [0,1]",
                i,
                s
            );
        }
        assert_eq!(r.ranked_indices[0], 2, "trial2 should rank best");
    }

    #[test]
    fn tc_2268_02_all_nan_returns_empty_valid_scores() {
        let values = [f64::NAN, f64::NAN, f64::NAN, f64::NAN];
        let r = compute_topsis(&values, 2, 2, &[0.5, 0.5], &[true, true]).unwrap();
        for &s in &r.scores {
            assert!(
                (s - 0.5).abs() < 1e-9,
                "all-NaN gives uniform 0.5, got {}",
                s
            );
        }
    }

    #[test]
    fn tc_2268_03_single_objective_single_alloc() {
        let values = [3.0_f64, 1.0, 2.0];
        let r = compute_topsis(&values, 3, 1, &[1.0], &[true]).unwrap();
        assert_eq!(r.ranked_indices[0], 1, "smallest value should rank first");
        assert_eq!(r.ranked_indices[2], 0, "largest value should rank last");
    }

    #[test]
    fn tc_1616_02_weights_scale_invariant() {
        // Since weights are normalized internally, the same ranking should
        // result as long as the ratio is unchanged, regardless of scale.
        let values = [1.0_f64, 5.0, 5.0, 1.0];
        let r1 = compute_topsis(&values, 2, 2, &[0.7, 0.3], &[true, true]).unwrap();
        // Weights scaled by the same ratio as above.
        let r2 = compute_topsis(&values, 2, 2, &[7.0, 3.0], &[true, true]).unwrap();

        assert_eq!(
            r1.ranked_indices[0], 0,
            "weights=[0.7,0.3] なら trial0 が 1 位のはず"
        );
        assert_eq!(
            r2.ranked_indices[0], 0,
            "weights=[7.0,3.0] でも同じ順位のはず"
        );
    }
}
