/// Documentation.
///
/// Documentation.
/// Documentation.
pub(super) fn col_mean_std(data: &[f64]) -> (f64, f64) {
    let n = data.len();
    if n == 0 {
        return (0.0, 1.0);
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let var = data.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = if var.sqrt() < f64::EPSILON {
        1.0
    } else {
        var.sqrt()
    };
    (mean, std_dev)
}

/// Documentation.
///
/// Documentation.
pub(super) fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }
    (0..n)
        .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
        .collect()
}
