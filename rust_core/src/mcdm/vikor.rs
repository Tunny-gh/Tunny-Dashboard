use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct VikorResult {
    pub s_values: Vec<f64>,
    pub r_values: Vec<f64>,
    pub q_values: Vec<f64>,
    pub display_scores: Vec<f64>,
    pub ranked_indices: Vec<u32>,
    pub best_values: Vec<f64>,
    pub worst_values: Vec<f64>,
    /// Compromise solution set per Opricovic & Tzeng (2004) conditions C1/C2,
    /// indices into the original alternatives, sorted by Q ascending.
    /// Empty when there are fewer than 1 valid (finite-row) alternative.
    pub compromise_indices: Vec<usize>,
    pub duration_ms: f64,
}

pub fn compute_vikor(
    values: &[f64],
    n_trials: usize,
    n_objectives: usize,
    weights: &[f64],
    is_minimize: &[bool],
    v: f64,
) -> Result<VikorResult, String> {
    let start = Instant::now();

    super::validate_inputs(values, n_trials, n_objectives, weights, is_minimize)?;

    // Weights are expected to sum to 1, but defend against callers that pass
    // unnormalized weights (or a degenerate sum) instead of erroring out.
    let weights = super::normalize_weights(weights);
    let weights = weights.as_slice();

    let valid_indices = super::filter_valid_indices(values, n_trials, n_objectives);

    if valid_indices.is_empty() {
        return Ok(uniform_vikor_result(n_trials, n_objectives, &start));
    }

    // 1. Find best/worst for each objective in a single pass
    let mut best_values = vec![f64::INFINITY; n_objectives];
    let mut worst_values = vec![f64::NEG_INFINITY; n_objectives];
    for &i in &valid_indices {
        let base = i * n_objectives;
        for j in 0..n_objectives {
            let val = values[base + j];
            let (b, w) = if is_minimize[j] {
                (
                    f64::min(best_values[j], val),
                    f64::max(worst_values[j], val),
                )
            } else {
                (
                    f64::max(best_values[j], val),
                    f64::min(worst_values[j], val),
                )
            };
            best_values[j] = b;
            worst_values[j] = w;
        }
    }

    // 2. Compute S and R for each valid trial
    let n_valid = valid_indices.len();
    let mut s_values = vec![0.0_f64; n_valid];
    let mut r_values = vec![0.0_f64; n_valid];

    for (vi, &ti) in valid_indices.iter().enumerate() {
        let base = ti * n_objectives;
        let mut s_i = 0.0_f64;
        let mut r_i = 0.0_f64;
        for j in 0..n_objectives {
            let range_j = (best_values[j] - worst_values[j]).abs();
            let contrib = if range_j < f64::EPSILON {
                0.0
            } else {
                weights[j] * (best_values[j] - values[base + j]).abs() / range_j
            };
            s_i += contrib;
            if contrib > r_i {
                r_i = contrib;
            }
        }
        s_values[vi] = s_i;
        r_values[vi] = r_i;
    }

    // 3. Find S*, S-, R*, R-
    let s_star = s_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let s_neg = s_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let r_star = r_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let r_neg = r_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    // 4. Compute Q values
    let s_range = s_neg - s_star;
    let r_range = r_neg - r_star;

    let mut q_valid = vec![0.0_f64; n_valid];
    for (vi, _) in valid_indices.iter().enumerate() {
        let term1 = if s_range < f64::EPSILON {
            0.0
        } else {
            (s_values[vi] - s_star) / s_range
        };
        let term2 = if r_range < f64::EPSILON {
            0.0
        } else {
            (r_values[vi] - r_star) / r_range
        };
        q_valid[vi] = v * term1 + (1.0 - v) * term2;
    }

    // 5. Build full result arrays (NaN trials get q=1.0)
    let mut s_full = vec![0.0_f64; n_trials];
    let mut r_full = vec![0.0_f64; n_trials];
    let mut q_full = vec![1.0_f64; n_trials];

    for (vi, &ti) in valid_indices.iter().enumerate() {
        s_full[ti] = s_values[vi];
        r_full[ti] = r_values[vi];
        q_full[ti] = q_valid[vi];
    }

    // 6. Rank by Q ascending (lower Q = better)
    let mut ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
    ranked_indices.sort_unstable_by(|&a, &b| {
        q_full[a as usize]
            .partial_cmp(&q_full[b as usize])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 7. display_scores = 1.0 - Q
    let display_scores: Vec<f64> = q_full.iter().map(|&q| 1.0 - q).collect();

    // 8. Compromise solution set (C1/C2, Opricovic & Tzeng 2004), computed on
    // valid alternatives only.
    let compromise_indices = compute_compromise_set(
        &valid_indices,
        &q_valid,
        &s_values,
        &r_values,
        s_star,
        r_star,
    );

    Ok(VikorResult {
        s_values: s_full,
        r_values: r_full,
        q_values: q_full,
        display_scores,
        ranked_indices,
        best_values,
        worst_values,
        compromise_indices,
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

/// Determine the VIKOR compromise solution set from the standard C1/C2
/// conditions (Opricovic & Tzeng, 2004).
///
/// `valid_indices[vi]` maps a local (valid-only) index `vi` to the original
/// alternative index. `q_valid`/`s_values`/`r_values` are aligned with
/// `valid_indices`. `s_star`/`r_star` are the precomputed min(S)/min(R).
///
/// - J < 1: empty set.
/// - J == 1: the single valid alternative.
/// - J >= 2: let A1, A2 be the two best (Q-ascending) alternatives and
///   DQ = 1 / (J - 1).
///   - C1 (acceptable advantage): Q(A2) - Q(A1) >= DQ
///   - C2 (acceptable stability): A1 also ties for best S or best R
///   - C1 & C2 hold -> {A1}
///   - only C2 fails -> {A1, A2}
///   - C1 fails -> all Ai with Q(Ai) - Q(A1) < DQ
fn compute_compromise_set(
    valid_indices: &[usize],
    q_valid: &[f64],
    s_values: &[f64],
    r_values: &[f64],
    s_star: f64,
    r_star: f64,
) -> Vec<usize> {
    let n_valid = valid_indices.len();
    if n_valid == 0 {
        return Vec::new();
    }

    // Local (valid-only) indices ordered by Q ascending.
    let mut order: Vec<usize> = (0..n_valid).collect();
    order.sort_unstable_by(|&a, &b| {
        q_valid[a]
            .partial_cmp(&q_valid[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if n_valid == 1 {
        return vec![valid_indices[order[0]]];
    }

    let a1 = order[0];
    let a2 = order[1];
    let dq = 1.0 / (n_valid as f64 - 1.0);

    let c1 = q_valid[a2] - q_valid[a1] >= dq;
    let c2 = s_values[a1] == s_star || r_values[a1] == r_star;

    if !c1 {
        order
            .iter()
            .take_while(|&&vi| q_valid[vi] - q_valid[a1] < dq)
            .map(|&vi| valid_indices[vi])
            .collect()
    } else if !c2 {
        vec![valid_indices[a1], valid_indices[a2]]
    } else {
        vec![valid_indices[a1]]
    }
}

fn uniform_vikor_result(n_trials: usize, n_objectives: usize, start: &Instant) -> VikorResult {
    let q_values = vec![1.0; n_trials];
    let display_scores = vec![0.0; n_trials];
    let ranked_indices: Vec<u32> = (0..n_trials as u32).collect();
    VikorResult {
        s_values: vec![0.0; n_trials],
        r_values: vec![0.0; n_trials],
        q_values,
        display_scores,
        ranked_indices,
        best_values: vec![0.0; n_objectives],
        worst_values: vec![0.0; n_objectives],
        compromise_indices: Vec::new(),
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
    }
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
    fn tc_vikor_001_basic_two_obj_minimize() {
        // 3 trials x 2 objectives, both minimize, v=0.5
        // trial0=[1,2], trial1=[3,1], trial2=[2,2]
        // best=[1,1], worst=[3,2], range=[2,1]
        // trial0: contrib=[0,0.5] S=0.5 R=0.5
        // trial1: contrib=[0.5,0] S=0.5 R=0.5
        // trial2: contrib=[0.25,0.5] S=0.75 R=0.5
        // S*=0.5, S-=0.75, R*=R-=0.5 (tie)
        // Q0=Q1=0.0, Q2=0.5
        let values = [1.0_f64, 2.0, 3.0, 1.0, 2.0, 2.0];
        let weights = [0.5_f64, 0.5];
        let is_minimize = [true, true];

        let result = compute_vikor(&values, 3, 2, &weights, &is_minimize, 0.5);

        assert!(result.is_ok(), "basic compute should succeed");
        let r = result.unwrap();

        assert_eq!(r.q_values.len(), 3);
        assert_eq!(r.s_values.len(), 3);
        assert_eq!(r.r_values.len(), 3);
        assert_eq!(r.ranked_indices.len(), 3);
        assert_eq!(r.display_scores.len(), 3);

        // trial2 has highest Q -> worst rank
        assert_eq!(
            *r.ranked_indices.last().unwrap(),
            2u32,
            "trial2 should be last: ranked={:?}",
            r.ranked_indices
        );

        // display_scores = 1.0 - q_values
        for i in 0..3 {
            assert!(
                (r.display_scores[i] - (1.0 - r.q_values[i])).abs() < 1e-9,
                "display_scores[{}] should be 1-q[{}]",
                i,
                i
            );
        }

        // Q values in [0,1]
        for &q in &r.q_values {
            assert!((0.0..=1.0 + 1e-9).contains(&q), "Q out of range: {}", q);
        }
    }

    #[test]
    fn tc_vikor_002_maximize_direction() {
        // 2 trials x 2 objectives
        // trial0=[5,1], trial1=[1,5], weights=[0.7,0.3], is_minimize=[false,true]
        // obj0 maximize: best=5, worst=1, range=4
        // obj1 minimize: best=1, worst=5, range=4
        // trial0: contrib=[0.7*(5-5)/4, 0.3*(1-1)/4]=[0,0] S=0 R=0
        // trial1: contrib=[0.7*(5-1)/4, 0.3*(1-5)/4|abs]=[0.7,0.3] S=1 R=0.7
        // -> trial0 is best (Q=0)
        let values = [5.0_f64, 1.0, 1.0, 5.0];
        let weights = [0.7_f64, 0.3];
        let is_minimize = [false, true];

        let result = compute_vikor(&values, 2, 2, &weights, &is_minimize, 0.5);
        assert!(result.is_ok());
        let r = result.unwrap();

        assert_eq!(
            r.ranked_indices[0], 0u32,
            "trial0 should be best: ranked={:?}",
            r.ranked_indices
        );
        assert!(
            r.q_values[0] < r.q_values[1],
            "trial0 Q < trial1 Q: {:?}",
            r.q_values
        );
    }

    #[test]
    fn tc_vikor_003_v_zero_r_only() {
        // v=0.0 => Q = (R-R*)/(R--R*)
        let values = [1.0_f64, 3.0, 2.0]; // 3 trials x 1 obj minimize
        let weights = [1.0_f64];
        let is_minimize = [true];

        let result = compute_vikor(&values, 3, 1, &weights, &is_minimize, 0.0);
        assert!(result.is_ok());
        let r = result.unwrap();

        // With v=0, Q should equal R normalized
        // S term has zero weight; Q = R normalized
        // R values = S values (single objective)
        // R* = min R, R- = max R
        let r_star = r.r_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let r_neg = r.r_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let denom = r_neg - r_star;
        if denom > 1e-9 {
            for i in 0..3 {
                let expected_q = (r.r_values[i] - r_star) / denom;
                assert!(
                    (r.q_values[i] - expected_q).abs() < 1e-9,
                    "v=0: Q[{}]={} expected {}",
                    i,
                    r.q_values[i],
                    expected_q
                );
            }
        }
    }

    #[test]
    fn tc_vikor_004_v_one_s_only() {
        // v=1.0 => Q = (S-S*)/(S--S*)
        let values = [1.0_f64, 3.0, 2.0];
        let weights = [1.0_f64];
        let is_minimize = [true];

        let result = compute_vikor(&values, 3, 1, &weights, &is_minimize, 1.0);
        assert!(result.is_ok());
        let r = result.unwrap();

        let s_star = r.s_values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let s_neg = r.s_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let denom = s_neg - s_star;
        if denom > 1e-9 {
            for i in 0..3 {
                let expected_q = (r.s_values[i] - s_star) / denom;
                assert!(
                    (r.q_values[i] - expected_q).abs() < 1e-9,
                    "v=1: Q[{}]={} expected {}",
                    i,
                    r.q_values[i],
                    expected_q
                );
            }
        }
    }

    #[test]
    fn tc_vikor_005_weights_affect_ranking() {
        // 2 trials x 2 objectives (both minimize)
        // trial0=[1,5], trial1=[5,1]
        let values = [1.0_f64, 5.0, 5.0, 1.0];
        let is_minimize = [true, true];

        // weights_a: obj0 dominant -> trial0 first
        let ra = compute_vikor(&values, 2, 2, &[0.9, 0.1], &is_minimize, 0.5).unwrap();
        assert_eq!(
            ra.ranked_indices[0], 0u32,
            "weights=[0.9,0.1]: trial0 should be best, got {:?}",
            ra.ranked_indices
        );

        // weights_b: obj1 dominant -> trial1 first
        let rb = compute_vikor(&values, 2, 2, &[0.1, 0.9], &is_minimize, 0.5).unwrap();
        assert_eq!(
            rb.ranked_indices[0], 1u32,
            "weights=[0.1,0.9]: trial1 should be best, got {:?}",
            rb.ranked_indices
        );
    }

    #[test]
    fn tc_vikor_006_ranked_indices_q_ascending() {
        // ranked_indices must be in Q ascending order
        let values = [1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = compute_vikor(&values, 3, 2, &[0.5, 0.5], &[true, true], 0.5).unwrap();

        for i in 0..result.ranked_indices.len() - 1 {
            let ia = result.ranked_indices[i] as usize;
            let ib = result.ranked_indices[i + 1] as usize;
            assert!(
                result.q_values[ia] <= result.q_values[ib] + 1e-12,
                "ranked_indices not Q-ascending at [{}]: q[{}]={} > q[{}]={}",
                i,
                ia,
                result.q_values[ia],
                ib,
                result.q_values[ib]
            );
        }
    }

    // -------------------------------------------------------------------------
    // Compromise set (C1/C2, Opricovic & Tzeng 2004)
    // -------------------------------------------------------------------------

    #[test]
    fn tc_vikor_007_compromise_c1_c2_hold_single_solution() {
        // 3 trials x 2 objectives, both minimize, weights=[0.5,0.5], v=0.5.
        // trial0=[1,1] (ideal in both), trial1=[8,8], trial2=[10,10]
        // best=[1,1], worst=[10,10], range=9 each
        // trial0: S=0, R=0
        // trial1: contrib=7/9*0.5=0.3889 each -> S=0.7778, R=0.3889
        // trial2: contrib=0.5 each -> S=1.0, R=0.5
        // S*=0, S-=1.0, R*=0, R-=0.5
        // Q0=0, Q1=0.5*0.7778+0.5*0.7778=0.7778, Q2=1.0
        // J=3, DQ=1/2=0.5; C1: Q1-Q0=0.7778>=0.5 -> holds
        // C2: trial0's S=0=S* -> holds
        // -> compromise = {trial0}
        let values = [1.0_f64, 1.0, 8.0, 8.0, 10.0, 10.0];
        let weights = [0.5_f64, 0.5];
        let is_minimize = [true, true];

        let r = compute_vikor(&values, 3, 2, &weights, &is_minimize, 0.5).unwrap();

        assert_eq!(
            r.compromise_indices,
            vec![0usize],
            "C1 & C2 both hold: compromise set should be {{trial0}}, got {:?}",
            r.compromise_indices
        );
    }

    #[test]
    fn tc_vikor_008_compromise_c1_fails_multi_solution() {
        // Same as above but trial1=[2,2] (much closer to trial0) so C1 fails.
        // trial0=[1,1], trial1=[2,2], trial2=[10,10]
        // trial0: S=0, R=0
        // trial1: contrib=1/9*0.5=0.05556 each -> S=0.1111, R=0.05556
        // trial2: S=1.0, R=0.5
        // Q0=0, Q1=0.5*0.1111+0.5*0.1111=0.1111, Q2=1.0
        // J=3, DQ=0.5; C1: Q1-Q0=0.1111>=0.5 -> fails
        // compromise = {Ai : Q(Ai)-Q(A1) < DQ} = {trial0 (0<0.5), trial1 (0.1111<0.5)}
        // trial2 excluded (1.0 !< 0.5)
        let values = [1.0_f64, 1.0, 2.0, 2.0, 10.0, 10.0];
        let weights = [0.5_f64, 0.5];
        let is_minimize = [true, true];

        let r = compute_vikor(&values, 3, 2, &weights, &is_minimize, 0.5).unwrap();

        assert_eq!(
            r.compromise_indices,
            vec![0usize, 1usize],
            "C1 fails: compromise set should be {{trial0, trial1}}, got {:?}",
            r.compromise_indices
        );
    }

    #[test]
    fn tc_vikor_009_compromise_c2_fails_two_solutions() {
        // Direct unit test of the compromise-set logic with hand-picked S/R/Q
        // values (independent of a full compute_vikor pipeline, since C1/C2
        // only depend on the relative ordering, not how S/R/Q were derived).
        //
        // 4 valid alternatives at original indices [100, 200, 300, 400]:
        //   idx100: Q=0.05 (best), S=0.5, R=0.3
        //   idx200: Q=0.9,         S=0.1 (=S*), R=0.4
        //   idx300: Q=0.8,         S=0.6, R=0.05 (=R*)
        //   idx400: Q=0.7,         S=0.7, R=0.6
        // A1=idx100 (Q=0.05), A2=idx400 (Q=0.7, next-smallest Q)
        // J=4, DQ=1/3=0.3333; C1: 0.7-0.05=0.65>=0.3333 -> holds
        // C2: idx100's S=0.5 != S*=0.1, R=0.3 != R*=0.05 -> fails
        // -> compromise = {A1, A2} = {idx100, idx400}, sorted by Q ascending
        let valid_indices = [100usize, 200, 300, 400];
        let q_valid = [0.05_f64, 0.9, 0.8, 0.7];
        let s_values = [0.5_f64, 0.1, 0.6, 0.7];
        let r_values = [0.3_f64, 0.4, 0.05, 0.6];
        let s_star = 0.1_f64;
        let r_star = 0.05_f64;

        let compromise = compute_compromise_set(
            &valid_indices,
            &q_valid,
            &s_values,
            &r_values,
            s_star,
            r_star,
        );

        assert_eq!(
            compromise,
            vec![100usize, 400usize],
            "C1 holds, C2 fails: compromise set should be {{idx100, idx400}}, got {:?}",
            compromise
        );
    }

    #[test]
    fn tc_vikor_010_compromise_single_valid_alternative() {
        // J=1: compromise set is just the single valid alternative.
        let valid_indices = [42usize];
        let q_valid = [0.3_f64];
        let s_values = [0.2_f64];
        let r_values = [0.2_f64];

        let compromise =
            compute_compromise_set(&valid_indices, &q_valid, &s_values, &r_values, 0.2, 0.2);

        assert_eq!(compromise, vec![42usize]);
    }

    #[test]
    fn tc_vikor_011_compromise_no_valid_alternatives() {
        let compromise = compute_compromise_set(&[], &[], &[], &[], f64::INFINITY, f64::INFINITY);
        assert!(compromise.is_empty());
    }

    #[test]
    fn tc_vikor_012_unnormalized_weights_match_normalized() {
        // Weights not summing to 1 must be defensively normalized to give the
        // same result as pre-normalized weights.
        let values = [1.0_f64, 1.0, 8.0, 8.0, 10.0, 10.0];
        let is_minimize = [true, true];

        let normalized = compute_vikor(&values, 3, 2, &[0.5, 0.5], &is_minimize, 0.5).unwrap();
        let unnormalized = compute_vikor(&values, 3, 2, &[2.0, 2.0], &is_minimize, 0.5).unwrap();

        for i in 0..3 {
            assert!(
                (normalized.q_values[i] - unnormalized.q_values[i]).abs() < 1e-9,
                "q_values[{}] differ: normalized={} unnormalized={}",
                i,
                normalized.q_values[i],
                unnormalized.q_values[i]
            );
        }
        assert_eq!(
            normalized.compromise_indices,
            unnormalized.compromise_indices
        );
    }

    // Weight normalization itself (empty/zero/negative/NaN fallback, division by
    // sum) is tested in `crate::mcdm::tests`; `tc_vikor_012` above covers that
    // compute_vikor's own normalization step produces consistent results.

    // -------------------------------------------------------------------------
    // Error cases
    // -------------------------------------------------------------------------

    #[test]
    fn tc_vikor_e01_zero_trials_error() {
        let result = compute_vikor(&[], 0, 2, &[0.5, 0.5], &[true, true], 0.5);
        assert!(result.is_err(), "n_trials=0 must return Err");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("n_trials"),
            "error msg must contain 'n_trials': {}",
            msg
        );
    }

    #[test]
    fn tc_vikor_e02_values_length_mismatch() {
        // expects 4 values, got 2
        let result = compute_vikor(&[1.0, 2.0], 2, 2, &[0.5, 0.5], &[true, true], 0.5);
        assert!(result.is_err(), "values length mismatch must return Err");
    }

    #[test]
    fn tc_vikor_e03_weights_length_mismatch() {
        let result = compute_vikor(&[1.0, 2.0, 3.0, 4.0], 2, 2, &[1.0], &[true, true], 0.5);
        assert!(result.is_err(), "weights length mismatch must return Err");
    }

    #[test]
    fn tc_vikor_e04_is_minimize_length_mismatch() {
        let result = compute_vikor(&[1.0, 2.0, 3.0, 4.0], 2, 2, &[0.5, 0.5], &[true], 0.5);
        assert!(
            result.is_err(),
            "is_minimize length mismatch must return Err"
        );
    }

    // -------------------------------------------------------------------------
    // Boundary / edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn tc_vikor_b01_single_trial() {
        // 1 trial: range_j=0 for all -> contrib=0 -> S=R=0 -> Q=0
        let result = compute_vikor(&[3.0, 7.0], 1, 2, &[0.5, 0.5], &[true, true], 0.5);
        assert!(result.is_ok(), "single trial must not error");
        let r = result.unwrap();
        assert_eq!(r.q_values.len(), 1);
        assert!(
            !r.q_values[0].is_nan(),
            "Q must not be NaN for single trial"
        );
        assert!(
            r.q_values[0].is_finite(),
            "Q must be finite for single trial"
        );
        assert_eq!(r.ranked_indices, vec![0u32]);
    }

    #[test]
    fn tc_vikor_b02_all_same_values() {
        // All trials identical -> zero division guards fire -> no NaN/crash
        let values = vec![2.0_f64, 3.0, 2.0, 3.0, 2.0, 3.0];
        let result = compute_vikor(&values, 3, 2, &[0.5, 0.5], &[true, true], 0.5);
        assert!(result.is_ok(), "all-same values must not error");
        let r = result.unwrap();
        for (i, &q) in r.q_values.iter().enumerate() {
            assert!(!q.is_nan(), "Q[{}] must not be NaN", i);
            assert!(q.is_finite(), "Q[{}] must be finite", i);
        }
    }

    #[test]
    fn tc_vikor_b03_nan_trial() {
        // trial1 has NaN -> excluded from computation -> q=1.0, ranked last
        let values = vec![1.0_f64, 1.0, f64::NAN, 1.0];
        let result = compute_vikor(&values, 2, 2, &[0.5, 0.5], &[true, true], 0.5);
        assert!(result.is_ok(), "NaN trial must not error");
        let r = result.unwrap();
        assert_eq!(
            r.q_values[1], 1.0,
            "NaN trial q must be 1.0, got {}",
            r.q_values[1]
        );
        assert_eq!(
            r.display_scores[1], 0.0,
            "NaN trial display_score must be 0.0"
        );
        assert_eq!(
            *r.ranked_indices.last().unwrap(),
            1u32,
            "NaN trial must be ranked last"
        );
    }

    #[test]
    fn tc_vikor_b04_single_objective() {
        // 3 trials x 1 objective (minimize)
        let values = vec![3.0_f64, 1.0, 2.0];
        let result = compute_vikor(&values, 3, 1, &[1.0], &[true], 0.5);
        assert!(result.is_ok(), "single objective must not error");
        let r = result.unwrap();
        // trial1 (value=1.0) has best value -> should be ranked first
        assert_eq!(
            r.ranked_indices[0], 1u32,
            "trial1 (min value) should be best: ranked={:?}",
            r.ranked_indices
        );
    }

    #[test]
    fn tc_vikor_b05_neg_inf_trial() {
        // trial1 has -Inf -> excluded from computation (same treatment as NaN) -> q=1.0, ranked last
        let values = vec![1.0_f64, 1.0, f64::NEG_INFINITY, 1.0];
        let result = compute_vikor(&values, 2, 2, &[0.5, 0.5], &[true, true], 0.5);
        assert!(result.is_ok(), "Inf trial must not error");
        let r = result.unwrap();
        assert_eq!(
            r.q_values[1], 1.0,
            "Inf trial q must be 1.0, got {}",
            r.q_values[1]
        );
        assert_eq!(
            r.display_scores[1], 0.0,
            "Inf trial display_score must be 0.0"
        );
        assert_eq!(
            *r.ranked_indices.last().unwrap(),
            1u32,
            "Inf trial must be ranked last"
        );
    }

    // -------------------------------------------------------------------------
    // Performance
    // -------------------------------------------------------------------------

    #[test]
    fn tc_vikor_perf01_50k_trials() {
        let n_trials: usize = 50_000;
        let n_objectives: usize = 4;
        let values: Vec<f64> = (0..n_trials * n_objectives)
            .map(|i| (i % 100) as f64)
            .collect();
        let weights = vec![0.25_f64; n_objectives];
        let is_minimize = vec![true; n_objectives];

        let result = compute_vikor(&values, n_trials, n_objectives, &weights, &is_minimize, 0.5);

        let result = result.expect("50k trials must not error");
        assert_eq!(result.ranked_indices.len(), n_trials);
    }
}
