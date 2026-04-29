/// AHP (Analytic Hierarchy Process)
///
/// TASK-2144: AHP implementation
///
/// Derives a priority vector (weights) from a pairwise comparison matrix
/// using the eigenvector approximation method, then scores trials via
/// weighted sum with Min-Max normalization.
use std::time::Instant;

const RI_TABLE: [f64; 6] = [0.0, 0.0, 0.58, 0.90, 1.12, 1.24];
// n=1  n=2  n=3   n=4   n=5   n≥6(approx)

#[derive(Debug, Clone, serde::Serialize)]
pub struct AhpResult {
    pub priority_vector: Vec<f64>,
    pub scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub lambda_max: f64,
    pub ci: f64,
    pub ri: f64,
    pub cr: f64,
    pub is_consistent: bool,
    pub duration_ms: f64,
}

pub fn compute_ahp(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    pairwise_matrix: &[f64],
    is_minimize: &[bool],
) -> Result<AhpResult, String> {
    let start = Instant::now();

    // Use uniform weights for validate_inputs (AHP derives its own weights)
    let uniform_weights = vec![1.0 / n_objectives as f64; n_objectives];
    super::validate_inputs(
        values,
        n_trials,
        n_objectives,
        &uniform_weights,
        is_minimize,
    )?;

    let expected_upper = n_objectives * n_objectives.saturating_sub(1) / 2;
    if pairwise_matrix.len() != expected_upper {
        return Err(format!(
            "pairwise_matrix length mismatch: expected {}, got {}",
            expected_upper,
            pairwise_matrix.len()
        ));
    }

    // Check pairwise values are positive (Saaty scale)
    for &v in pairwise_matrix {
        if v <= 0.0 {
            return Err("pairwise_matrix values must be positive (Saaty 1-9 scale)".to_string());
        }
    }

    // n=1 early return
    if n_objectives == 1 {
        let priority_vector = vec![1.0];
        let result = compute_scores(values, n_trials, 1, &priority_vector, is_minimize);
        return Ok(AhpResult {
            priority_vector,
            scores: result.scores,
            ranked_indices: result.ranked_indices,
            lambda_max: 1.0,
            ci: 0.0,
            ri: 0.0,
            cr: 0.0,
            is_consistent: true,
            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
        });
    }

    // Expand upper-triangle to full n×n matrix (row-major flat)
    let mut matrix = vec![0.0f64; n_objectives * n_objectives];
    for i in 0..n_objectives {
        matrix[i * n_objectives + i] = 1.0;
    }
    for i in 0..n_objectives {
        for j in (i + 1)..n_objectives {
            let idx = upper_tri_index(n_objectives, i, j);
            let val = pairwise_matrix[idx];
            matrix[i * n_objectives + j] = val;
            matrix[j * n_objectives + i] = 1.0 / val;
        }
    }

    // Column sums
    let mut col_sums = vec![0.0f64; n_objectives];
    for j in 0..n_objectives {
        for i in 0..n_objectives {
            col_sums[j] += matrix[i * n_objectives + j];
        }
    }
    for &cs in &col_sums {
        if cs == 0.0 {
            return Err("column sum is zero in pairwise comparison matrix".to_string());
        }
    }

    // Priority vector (eigenvector approximation: normalize columns, then row average)
    let mut priority_vector = vec![0.0f64; n_objectives];
    for i in 0..n_objectives {
        let row_sum: f64 = (0..n_objectives)
            .map(|j| matrix[i * n_objectives + j] / col_sums[j])
            .sum();
        priority_vector[i] = row_sum / n_objectives as f64;
    }

    // Consistency check
    let lambda_max: f64 = col_sums
        .iter()
        .zip(priority_vector.iter())
        .map(|(c, w)| c * w)
        .sum();
    let n = n_objectives as f64;
    let ci = if n > 1.0 {
        (lambda_max - n) / (n - 1.0)
    } else {
        0.0
    };
    let ri_idx = (n_objectives - 1).min(5);
    let ri = RI_TABLE[ri_idx];
    let cr = if ri > 0.0 { ci / ri } else { 0.0 };
    let is_consistent = cr <= 0.10;

    let result = compute_scores(
        values,
        n_trials,
        n_objectives,
        &priority_vector,
        is_minimize,
    );

    Ok(AhpResult {
        priority_vector,
        scores: result.scores,
        ranked_indices: result.ranked_indices,
        lambda_max,
        ci,
        ri,
        cr,
        is_consistent,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

struct ScoreResult {
    scores: Vec<f64>,
    ranked_indices: Vec<u32>,
}

fn compute_scores(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    priority_vector: &[f64],
    is_minimize: &[bool],
) -> ScoreResult {
    let valid_indices = super::filter_valid_indices(values, n_trials, n_objectives);

    let mut scores = vec![0.0f64; n_trials];

    if valid_indices.is_empty() {
        let ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
        return ScoreResult {
            scores,
            ranked_indices,
        };
    }

    // min/max per objective (valid trials only)
    let mut min_vals = vec![f64::INFINITY; n_objectives];
    let mut max_vals = vec![f64::NEG_INFINITY; n_objectives];
    for &idx in &valid_indices {
        for j in 0..n_objectives {
            let v = values[idx * n_objectives + j];
            if v < min_vals[j] {
                min_vals[j] = v;
            }
            if v > max_vals[j] {
                max_vals[j] = v;
            }
        }
    }

    // Score valid trials
    for &idx in &valid_indices {
        let mut score = 0.0f64;
        for j in 0..n_objectives {
            let v = values[idx * n_objectives + j];
            let range = max_vals[j] - min_vals[j];
            let normalized = if range > 0.0 {
                if is_minimize[j] {
                    (max_vals[j] - v) / range
                } else {
                    (v - min_vals[j]) / range
                }
            } else {
                0.0
            };
            score += priority_vector[j] * normalized;
        }
        scores[idx] = score;
    }

    // Rank: valid trials by score descending, NaN trials at the end
    let valid_set: std::collections::HashSet<usize> = valid_indices.iter().copied().collect();
    let mut ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
    ranked_indices.sort_by(|&a, &b| {
        let a_valid = valid_set.contains(&(a as usize));
        let b_valid = valid_set.contains(&(b as usize));
        match (a_valid, b_valid) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => scores[b as usize]
                .partial_cmp(&scores[a as usize])
                .unwrap_or(std::cmp::Ordering::Equal),
        }
    });

    ScoreResult {
        scores,
        ranked_indices,
    }
}

pub fn upper_tri_index(n: usize, i: usize, j: usize) -> usize {
    debug_assert!(i < j && j < n);
    i * (2 * n - i - 1) / 2 + (j - i - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tc_ahp_001_01_n2_basic() {
        // n=2, pairwise=[3.0] → A[0][1]=3, A[1][0]=1/3
        // col_sums = [1+1/3, 3+1] = [1.333, 4.0]
        // normalized: [1/1.333, 3/4.0] = [0.75, 0.75] → row avg = [0.75, 0.75]? No.
        // Row 0: 1/1.333 + 3/4.0 = 0.75 + 0.75 = 1.5 → 1.5/2 = 0.75
        // Row 1: (1/3)/1.333 + 1/4.0 = 0.25 + 0.25 = 0.5 → 0.5/2 = 0.25
        let values = [1.0_f64, 2.0, 3.0, 4.0]; // 2 trials × 2 objectives
        let pairwise = [3.0_f64];
        let is_minimize = [false, false];
        let result = compute_ahp(&values, 2, 2, &pairwise, &is_minimize).unwrap();

        assert!((result.priority_vector[0] - 0.75).abs() < 1e-9);
        assert!((result.priority_vector[1] - 0.25).abs() < 1e-9);
        assert!((result.cr - 0.0).abs() < 1e-9, "n=2: CR should be 0.0");
        assert!(result.is_consistent);
    }

    #[test]
    fn tc_ahp_001_02_n3_saaty_textbook() {
        // Saaty textbook example: price/quality/design
        // A = [[1,3,5],[1/3,1,3],[1/5,1/3,1]]
        // Upper triangle: [3.0, 5.0, 3.0]
        let pairwise = [3.0_f64, 5.0, 3.0];
        let values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]; // 3×3
        let is_minimize = [false, false, false];
        let result = compute_ahp(&values, 3, 3, &pairwise, &is_minimize).unwrap();

        // priority_vector ≈ [0.637, 0.258, 0.105]
        assert!(
            (result.priority_vector[0] - 0.637).abs() < 0.01,
            "w[0] ≈ 0.637, got {}",
            result.priority_vector[0]
        );
        assert!(
            (result.priority_vector[1] - 0.258).abs() < 0.01,
            "w[1] ≈ 0.258, got {}",
            result.priority_vector[1]
        );
        assert!(
            (result.priority_vector[2] - 0.105).abs() < 0.01,
            "w[2] ≈ 0.105, got {}",
            result.priority_vector[2]
        );
        assert!(
            result.cr < 0.10,
            "CR ≈ 0.034, should be < 0.10, got {}",
            result.cr
        );
        assert!(result.is_consistent);
    }

    #[test]
    fn tc_ahp_001_03_n4_inconsistent() {
        // Intentionally inconsistent pairwise matrix for n=4
        // Upper triangle: [9.0, 9.0, 9.0, 1.0/9.0, 1.0/9.0, 9.0]
        // This creates a highly inconsistent matrix
        let pairwise = [9.0_f64, 9.0, 9.0, 1.0 / 9.0, 1.0 / 9.0, 9.0];
        let values = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ]; // 3×4
        let is_minimize = [false, false, false, false];
        let result = compute_ahp(&values, 3, 4, &pairwise, &is_minimize).unwrap();

        assert!(
            !result.is_consistent,
            "CR = {}, should be > 0.10",
            result.cr
        );
        assert!(result.cr > 0.10);
        // Should still complete successfully
        assert_eq!(result.scores.len(), 3);
    }

    #[test]
    fn tc_ahp_004_01_nan_trial_ranked_last() {
        // n=2, 3 trials, trial #1 has NaN
        let values = [1.0_f64, 2.0, f64::NAN, 1.0, 3.0, 4.0];
        let pairwise = [2.0_f64];
        let is_minimize = [false, false];
        let result = compute_ahp(&values, 3, 2, &pairwise, &is_minimize).unwrap();

        assert!(
            (result.scores[1] - 0.0).abs() < f64::EPSILON,
            "NaN trial score = 0.0"
        );
        assert_eq!(
            *result.ranked_indices.last().unwrap(),
            1u32,
            "NaN trial at end"
        );
    }

    #[test]
    fn tc_ahp_006_01_n1_single_objective() {
        let values = [3.0_f64, 1.0, 2.0]; // 3 trials × 1 objective
        let pairwise: [f64; 0] = [];
        let is_minimize = [true];
        let result = compute_ahp(&values, 3, 1, &pairwise, &is_minimize).unwrap();

        assert_eq!(result.priority_vector, vec![1.0]);
        assert!((result.ci - 0.0).abs() < f64::EPSILON);
        assert!((result.cr - 0.0).abs() < f64::EPSILON);
        assert!(result.is_consistent);
        // minimize: smallest value (1.0) should rank first
        assert_eq!(
            result.ranked_indices[0], 1,
            "trial 1 (value=1.0) should rank first"
        );
    }

    #[test]
    fn tc_ahp_007_01_minimize_direction() {
        // n=1, minimize=true: smaller values get higher scores
        let values = [10.0_f64, 2.0, 5.0]; // 3 trials × 1 objective
        let pairwise: [f64; 0] = [];
        let is_minimize = [true];
        let result = compute_ahp(&values, 3, 1, &pairwise, &is_minimize).unwrap();

        // trial 1 (2.0) should have highest score
        assert!(
            result.scores[1] > result.scores[0],
            "minimize: trial1(2.0) > trial0(10.0)"
        );
        assert!(
            result.scores[1] > result.scores[2],
            "minimize: trial1(2.0) > trial2(5.0)"
        );
    }

    #[test]
    fn tc_ahp_007_02_max_equals_min() {
        // All trials have same objective value → normalized = 0.0, no panic
        let values = [5.0_f64, 5.0, 5.0]; // 3 trials × 1 objective, all same
        let pairwise: [f64; 0] = [];
        let is_minimize = [false];
        let result = compute_ahp(&values, 3, 1, &pairwise, &is_minimize).unwrap();

        for &s in &result.scores {
            assert!((s - 0.0).abs() < f64::EPSILON, "same values → score = 0.0");
        }
    }

    #[test]
    fn tc_ahp_008_01_ranking_descending() {
        // n=1, maximize: verify ranked_indices is score-descending
        let values = [1.0_f64, 5.0, 3.0]; // 3 trials × 1 objective
        let pairwise: [f64; 0] = [];
        let is_minimize = [false];
        let result = compute_ahp(&values, 3, 1, &pairwise, &is_minimize).unwrap();

        // ranked_indices should be score-descending
        for i in 0..result.ranked_indices.len() - 1 {
            let idx_curr = result.ranked_indices[i] as usize;
            let idx_next = result.ranked_indices[i + 1] as usize;
            assert!(
                result.scores[idx_curr] >= result.scores[idx_next],
                "ranked_indices not descending: scores[{}]={} < scores[{}]={}",
                idx_curr,
                result.scores[idx_curr],
                idx_next,
                result.scores[idx_next]
            );
        }
        // trial 1 (5.0) should rank first for maximize
        assert_eq!(result.ranked_indices[0], 1);
    }
}
