/// Pareto dominance check assuming minimization.
///
/// True when `a` dominates `b` (a <= b on every objective, and a < b on at
/// least one). A dimension containing NaN has both-direction comparisons
/// evaluate to false, so it is skipped as "equal" and the determination is
/// made using only the remaining dimensions (callers are expected to have
/// already filtered to finite values as a rule).
pub(crate) fn dominates_minimized(a: &[f64], b: &[f64]) -> bool {
    let mut strictly_better = false;
    for (&ai, &bi) in a.iter().zip(b.iter()) {
        if ai > bi {
            return false;
        }
        if ai < bi {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Unifies objective values to the minimization direction (sign-flips maximize objectives).
///
/// If `is_minimize` is shorter than the number of objectives, the missing entries are treated as minimize.
pub(crate) fn normalize_objectives(objectives: &[Vec<f64>], is_minimize: &[bool]) -> Vec<Vec<f64>> {
    objectives
        .iter()
        .map(|obj| {
            obj.iter()
                .enumerate()
                .map(|(j, &v)| {
                    if is_minimize.get(j).copied().unwrap_or(true) {
                        v
                    } else {
                        -v
                    }
                })
                .collect()
        })
        .collect()
}

/// Adds a point to the Pareto front. Does nothing if the point is dominated.
pub(crate) fn add_to_pareto_front(front: &mut Vec<Vec<f64>>, point: Vec<f64>) {
    if front.iter().any(|f| dominates_minimized(f, &point)) {
        return;
    }
    front.retain(|f| !dominates_minimized(&point, f));
    front.push(point);
}

/// Automatic computation of the HV reference point: nadir + 0.1·(nadir − ideal).
///
/// Scaling the margin proportionally to the observed range makes it invariant
/// to the objective values' scale (the old implementation's constant +1.0
/// distorted HV for studies with a small scale). For a dimension whose range
/// is degenerate, falls back to a margin proportional to the value's own
/// magnitude (|nadir|·0.1, or 1.0 if that's also 0).
pub(crate) fn compute_ref_point(pareto_objs: &[Vec<f64>], m: usize) -> Vec<f64> {
    let mut nadir = vec![f64::NEG_INFINITY; m];
    let mut ideal = vec![f64::INFINITY; m];
    for obj in pareto_objs {
        for (j, &v) in obj.iter().enumerate() {
            if v > nadir[j] {
                nadir[j] = v;
            }
            if v < ideal[j] {
                ideal[j] = v;
            }
        }
    }
    (0..m)
        .map(|j| {
            let range = nadir[j] - ideal[j];
            let offset = if range > 1e-12 {
                0.1 * range
            } else if nadir[j].abs() > 1e-12 {
                0.1 * nadir[j].abs()
            } else {
                1.0
            };
            nadir[j] + offset
        })
        .collect()
}
