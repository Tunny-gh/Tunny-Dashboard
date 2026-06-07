use super::helpers::normalize_objectives;

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn chebyshev_sort(objectives: &[Vec<f64>], weights: &[f64], is_minimize: &[bool]) -> Vec<u32> {
    let n = objectives.len();
    if n == 0 {
        return vec![];
    }
    let m = objectives[0].len();
    if m == 0 || weights.iter().all(|&w| w == 0.0) {
        return (0..n as u32).collect();
    }

    let norm_objs = normalize_objectives(objectives, is_minimize);

    let ideal: Vec<f64> = (0..m)
        .map(|j| {
            norm_objs
                .iter()
                .map(|obj| obj[j])
                .filter(|v| !v.is_nan())
                .fold(f64::INFINITY, f64::min)
        })
        .collect();

    let mut scores: Vec<(usize, f64)> = norm_objs
        .iter()
        .enumerate()
        .map(|(i, obj)| {
            let score = obj
                .iter()
                .enumerate()
                .map(|(j, &v)| {
                    let w = weights.get(j).copied().unwrap_or(0.0);
                    w * (v - ideal[j]).abs()
                })
                .fold(0.0f64, f64::max);
            (i, if score.is_nan() { f64::INFINITY } else { score })
        })
        .collect();

    scores.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.into_iter().map(|(i, _)| i as u32).collect()
}
