/// Documentation.
///
/// Documentation.
pub(super) fn dominates_minimized(a: &[f64], b: &[f64]) -> bool {
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

/// Documentation.
///
/// Documentation.
pub(super) fn normalize_objectives(objectives: &[Vec<f64>], is_minimize: &[bool]) -> Vec<Vec<f64>> {
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

/// Documentation.
///
/// Documentation.
pub(super) fn compute_ref_point(pareto_objs: &[Vec<f64>], m: usize) -> Vec<f64> {
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
        .map(|j| nadir[j] + (nadir[j] - ideal[j]).abs() * 0.1 + 1.0)
        .collect()
}
