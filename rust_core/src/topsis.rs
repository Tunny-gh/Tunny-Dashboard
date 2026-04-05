/// TOPSIS (Technique for Order Preference by Similarity to Ideal Solution)
/// English documentation.
///
/// TASK-1615: mode-frontier-features
/// English documentation.
/// English documentation.
use std::time::Instant;

/// English documentation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TopsisResult {
    /// English documentation.
    pub scores: Vec<f64>,
    /// English documentation.
    pub ranked_indices: Vec<u32>,
    /// English documentation.
    pub positive_ideal: Vec<f64>,
    /// English documentation.
    pub negative_ideal: Vec<f64>,
    /// English documentation.
    pub duration_ms: f64,
}

/// English documentation.
///
/// English documentation.
///
/// English documentation.
/// English documentation.
/// English documentation.
/// English documentation.
/// English documentation.
/// English documentation.
///
/// English documentation.
/// English documentation.
pub fn compute_topsis(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
) -> Result<TopsisResult, String> {
    let start = Instant::now();

    // English comment.
    validate_inputs(values, n_trials, n_objectives, weights, is_minimize)?;

    // English comment.
    let valid_indices: Vec<usize> = (0..n_trials)
        .filter(|&i| !(0..n_objectives).any(|j| values[i * n_objectives + j].is_nan()))
        .collect();

    // English comment.
    if valid_indices.is_empty() {
        return Ok(uniform_score_result(n_trials, n_objectives, 0.5, &start));
    }

    // English comment.
    let weighted_matrix =
        build_weighted_matrix(values, n_trials, n_objectives, weights, &valid_indices);

    // English comment.
    let (positive_ideal, negative_ideal) =
        find_ideal_solutions(&weighted_matrix, n_objectives, is_minimize);

    // English comment.
    let valid_scores = compute_scores(&weighted_matrix, &positive_ideal, &negative_ideal);

    // English comment.
    let mut scores = vec![0.0_f64; n_trials];
    for (vi, &ti) in valid_indices.iter().enumerate() {
        scores[ti] = valid_scores[vi];
    }

    // English comment.
    let mut ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
    ranked_indices.sort_by(|&a, &b| {
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
// English comment.
// =============================================================================

/// English documentation.
/// English documentation.
fn validate_inputs(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
) -> Result<(), String> {
    if n_trials == 0 {
        return Err("n_trials must be >= 1".to_string());
    }
    if n_objectives == 0 {
        return Err("n_objectives must be >= 1".to_string());
    }
    if values.len() != n_trials * n_objectives {
        return Err(format!(
            "values length mismatch: expected {}, got {}",
            n_trials * n_objectives,
            values.len()
        ));
    }
    if weights.len() != n_objectives {
        return Err(format!(
            "weights length mismatch: expected {}, got {}",
            n_objectives,
            weights.len()
        ));
    }
    if is_minimize.len() != n_objectives {
        return Err(format!(
            "is_minimize length mismatch: expected {}, got {}",
            n_objectives,
            is_minimize.len()
        ));
    }
    Ok(())
}

/// English documentation.
/// English documentation.
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

/// English documentation.
///
/// r_ij = v_ij / sqrt(sum_i(v_ij^2))  → w_ij = weights[j] * r_ij
///
/// English documentation.
fn build_weighted_matrix(
    values: &[f64],
    _n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    valid_indices: &[usize],
) -> Vec<Vec<f64>> {
    // English comment.
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

    // English comment.
    valid_indices
        .iter()
        .map(|&i| {
            (0..n_objectives)
                .map(|j| {
                    let v = values[i * n_objectives + j];
                    let r = if col_norms[j].abs() < f64::EPSILON {
                        0.0
                    } else {
                        v / col_norms[j]
                    };
                    r * weights[j]
                })
                .collect()
        })
        .collect()
}

/// English documentation.
///
/// English documentation.
/// English documentation.
fn find_ideal_solutions(
    weighted_matrix: &[Vec<f64>],
    n_objectives: usize,
    is_minimize: &[bool],
) -> (Vec<f64>, Vec<f64>) {
    let mut positive = vec![0.0_f64; n_objectives];
    let mut negative = vec![0.0_f64; n_objectives];

    for j in 0..n_objectives {
        let col_min = weighted_matrix
            .iter()
            .map(|r| r[j])
            .fold(f64::INFINITY, f64::min);
        let col_max = weighted_matrix
            .iter()
            .map(|r| r[j])
            .fold(f64::NEG_INFINITY, f64::max);
        (positive[j], negative[j]) = if is_minimize[j] {
            (col_min, col_max) // English: English
        } else {
            (col_max, col_min) // English: English
        };
    }
    (positive, negative)
}

/// English documentation.
///
/// D+_i = sqrt(sum_j(w_ij - A+_j)^2)
/// D-_i = sqrt(sum_j(w_ij - A-_j)^2)
/// score_i = D-_i / (D+_i + D-_i)  D++D-=0 → 0.5 🔵
fn compute_scores(
    weighted_matrix: &[Vec<f64>],
    positive_ideal: &[f64],
    negative_ideal: &[f64],
) -> Vec<f64> {
    weighted_matrix
        .iter()
        .map(|row| {
            let d_plus: f64 = row
                .iter()
                .enumerate()
                .map(|(j, &w)| (w - positive_ideal[j]).powi(2))
                .sum::<f64>()
                .sqrt();
            let d_minus: f64 = row
                .iter()
                .enumerate()
                .map(|(j, &w)| (w - negative_ideal[j]).powi(2))
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
// English comment.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // English comment.
    // -------------------------------------------------------------------------

    #[test]
    fn tc_1615_01_basic_two_obj_minimize() {
        // English comment.
        // English comment.
        // English comment.
        // English comment.

        // English comment.
        // English comment.
        // English comment.
        // English comment.
        let values = [1.0_f64, 4.0, 4.0, 1.0, 2.0, 2.0];
        let weights = [0.5_f64, 0.5];
        let is_minimize = [true, true];

        // English comment.
        let result = compute_topsis(&values, 3, 2, &weights, &is_minimize);

        // English comment.
        assert!(result.is_ok(), "English");
        let r = result.unwrap();

        // English comment.
        assert_eq!(r.ranked_indices.len(), 3);
        // English comment.
        assert_eq!(r.scores.len(), 3);
        // English comment.
        for &s in &r.scores {
            assert!(s >= 0.0 && s <= 1.0, "English0〜1English: {}", s);
        }
        // English comment.
        for i in 0..r.ranked_indices.len() - 1 {
            let idx_curr = r.ranked_indices[i] as usize;
            let idx_next = r.ranked_indices[i + 1] as usize;
            assert!(
                r.scores[idx_curr] >= r.scores[idx_next],
                "English: scores[{}]={} >= scores[{}]={}",
                idx_curr,
                r.scores[idx_curr],
                idx_next,
                r.scores[idx_next]
            );
        }
    }

    #[test]
    fn tc_1615_02_maximize_direction() {
        // English comment.
        // English comment.
        // English comment.
        // English comment.

        // English comment.
        // English comment.
        // English comment.
        // English comment.
        let values = [1.0_f64, 1.0, 5.0, 1.0, 5.0, 5.0];
        let weights = [0.7_f64, 0.3]; // obj0English
        let is_minimize = [false, true]; // obj0Englishmaximize

        // English comment.
        let result = compute_topsis(&values, 3, 2, &weights, &is_minimize);

        // English comment.
        assert!(result.is_ok());
        let r = result.unwrap();

        // English comment.
        assert_eq!(
            r.ranked_indices[0], 1,
            "trial1Englishobj0English・obj1English。ranked[0]={}",
            r.ranked_indices[0]
        );
        // English comment.
        assert!(
            r.scores[0] < r.scores[1],
            "trial1>trial0English: scores[0]={}, scores[1]={}",
            r.scores[0],
            r.scores[1]
        );
    }

    #[test]
    fn tc_1615_03_weights_affect_ranking() {
        // English comment.
        // English comment.
        // English comment.
        // English comment.

        let values = [1.0_f64, 5.0, 5.0, 1.0]; // trial0:(1,5) trial1:(5,1)
        let is_minimize = [true, true];

        // English comment.
        let result_a = compute_topsis(&values, 2, 2, &[0.9, 0.1], &is_minimize).unwrap();
        // English comment.
        let result_b = compute_topsis(&values, 2, 2, &[0.1, 0.9], &is_minimize).unwrap();

        // English comment.
        assert_eq!(
            result_a.ranked_indices[0], 0,
            "obj0Englishtrial0English1English"
        );
        // English comment.
        assert_eq!(
            result_b.ranked_indices[0], 1,
            "obj1Englishtrial1English1English"
        );
    }

    #[test]
    fn tc_1615_04_single_trial() {
        // English comment.
        // English comment.
        // English comment.

        let values = [3.0_f64, 7.0];
        let result = compute_topsis(&values, 1, 2, &[0.5, 0.5], &[true, true]);

        assert!(result.is_ok());
        let r = result.unwrap();
        // English comment.
        assert_eq!(r.scores.len(), 1);
        // English comment.
        assert!(
            (r.scores[0] - 0.5).abs() < 1e-9,
            "1Englishscore=0.5English: {}",
            r.scores[0]
        );
        // English comment.
        assert_eq!(r.ranked_indices, vec![0u32]);
    }

    #[test]
    fn tc_1615_05_single_objective() {
        // English comment.
        // English comment.

        let values = [3.0_f64, 1.0, 2.0]; // 3English×1English
        let result = compute_topsis(&values, 3, 1, &[1.0], &[true]);

        assert!(result.is_ok());
        let r = result.unwrap();
        // English comment.
        assert_eq!(r.ranked_indices[0], 1, "English1.0Englishtrial1English");
        // English comment.
        assert_eq!(r.ranked_indices[2], 0, "English3.0Englishtrial0English");
    }

    // -------------------------------------------------------------------------
    // English comment.
    // -------------------------------------------------------------------------

    #[test]
    fn tc_1615_06_zero_trials_error() {
        // English comment.
        // English comment.

        let result = compute_topsis(&[], 0, 2, &[0.5, 0.5], &[true, true]);
        assert!(result.is_err(), "n_trials=0EnglishーEnglish");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("n_trials") || msg.contains("trial"),
            "EnglishーEnglishーEnglishn_trialsEnglish: {}",
            msg
        );
    }

    #[test]
    fn tc_1615_07_values_length_mismatch_error() {
        // English comment.
        // English comment.

        // English comment.
        let result = compute_topsis(&[1.0, 2.0, 3.0], 2, 2, &[0.5, 0.5], &[true, true]);
        assert!(result.is_err(), "valuesEnglishーEnglish");
    }

    #[test]
    fn tc_1615_08_weights_length_mismatch_error() {
        // English comment.
        // English comment.

        // English comment.
        let result = compute_topsis(&[1.0, 2.0, 3.0, 4.0], 2, 2, &[1.0], &[true, true]);
        assert!(result.is_err(), "weightsEnglishーEnglish");
    }

    #[test]
    fn tc_1615_09_is_minimize_length_mismatch_error() {
        // English comment.
        // English comment.

        // English comment.
        let result = compute_topsis(&[1.0, 2.0, 3.0, 4.0], 2, 2, &[0.5, 0.5], &[true]);
        assert!(result.is_err(), "is_minimizeEnglishーEnglish");
    }

    // -------------------------------------------------------------------------
    // English comment.
    // -------------------------------------------------------------------------

    #[test]
    fn tc_1615_10_all_same_values_no_crash() {
        // English comment.
        // English comment.
        // English comment.

        let values = [2.0_f64, 3.0, 2.0, 3.0, 2.0, 3.0]; // 3English
        let result = compute_topsis(&values, 3, 2, &[0.5, 0.5], &[true, true]);

        // English comment.
        assert!(result.is_ok(), "EnglishーEnglish");
        let r = result.unwrap();
        // English comment.
        for &s in &r.scores {
            assert!((s - 0.5).abs() < 1e-9, "Englishscore=0.5: {}", s);
        }
    }

    #[test]
    fn tc_1615_11_nan_trial_ranked_last() {
        // English comment.
        // English comment.
        // English comment.

        // English comment.
        let values = [1.0_f64, 1.0, f64::NAN, 1.0];
        let result = compute_topsis(&values, 2, 2, &[0.5, 0.5], &[true, true]);

        assert!(result.is_ok());
        let r = result.unwrap();
        // English comment.
        assert_eq!(r.scores[1], 0.0, "NaNEnglish0.0English");
        // English comment.
        assert_eq!(*r.ranked_indices.last().unwrap(), 1u32, "NaNEnglish");
    }

    #[test]
    fn tc_1615_12_performance_50k_trials() {
        // English comment.
        // English comment.

        let n_trials: usize = 50_000;
        let n_objectives: usize = 4;
        // English comment.
        let values: Vec<f64> = (0..n_trials * n_objectives)
            .map(|i| (i % 100) as f64)
            .collect();
        let weights = [0.25_f64; 4];
        let is_minimize = [true; 4];

        // English comment.
        let start = Instant::now();
        let result = compute_topsis(&values, n_trials, n_objectives, &weights, &is_minimize);
        let elapsed_ms = start.elapsed().as_millis();

        // English comment.
        assert!(result.is_ok());
        // English comment.
        assert!(
            elapsed_ms < 100,
            "50K×4English100msEnglish: {}ms",
            elapsed_ms
        );
    }

    #[test]
    fn tc_1615_13_ranked_indices_length() {
        // English comment.
        // English comment.

        let values: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 3English×2English
        let result = compute_topsis(&values, 3, 2, &[0.5, 0.5], &[true, true]).unwrap();

        // English comment.
        assert_eq!(result.ranked_indices.len(), 3);
        // English comment.
        assert_eq!(result.scores.len(), 3);
        // English comment.
        let mut sorted = result.ranked_indices.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0u32, 1, 2], "0,1,2English1English");
    }

    #[test]
    fn tc_1615_14_ideal_solutions_dimension() {
        // English comment.
        // English comment.

        let values: Vec<f64> = (0..9).map(|i| i as f64).collect(); // 3English×3English
        let result = compute_topsis(&values, 3, 3, &[1.0 / 3.0; 3], &[true; 3]).unwrap();

        // English comment.
        assert_eq!(result.positive_ideal.len(), 3);
        // English comment.
        assert_eq!(result.negative_ideal.len(), 3);
    }

    #[test]
    fn tc_1616_01_two_trials_ranking() {
        // English comment.
        // English comment.
        // English comment.
        // English comment.
        let values = [1.0_f64, 2.0, 3.0, 4.0];
        let result = compute_topsis(&values, 2, 2, &[0.5, 0.5], &[true, true]).unwrap();

        // English comment.
        assert_eq!(result.ranked_indices[0], 0, "trial0English1English");
        // English comment.
        assert!(
            result.scores[0] > result.scores[1],
            "trial0Englishtrial1English"
        );
    }

    #[test]
    fn tc_1616_02_weights_scale_invariant() {
        // English comment.
        // English comment.
        // English comment.
        // English comment.
        let values = [1.0_f64, 5.0, 5.0, 1.0];
        let r1 = compute_topsis(&values, 2, 2, &[0.7, 0.3], &[true, true]).unwrap();
        // English comment.
        let r2 = compute_topsis(&values, 2, 2, &[7.0, 3.0], &[true, true]).unwrap();

        // English comment.
        assert_eq!(
            r1.ranked_indices[0], 0,
            "weights=[0.7,0.3]Englishtrial0English1English"
        );
        assert_eq!(r2.ranked_indices[0], 0, "weights=[7.0,3.0]English");
    }
}
