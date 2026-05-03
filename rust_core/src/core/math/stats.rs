/// Column mean and standard deviation.
/// - Empty slice: returns (0.0, 1.0)
/// - std < EPSILON: std is fixed to 1.0 (zero-division guard)
pub(crate) fn column_mean_std(vals: &[f64]) -> (f64, f64) {
    let n = vals.len();
    if n == 0 {
        return (0.0, 1.0);
    }
    let nf = n as f64;
    let mean = vals.iter().sum::<f64>() / nf;
    let var = vals.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / nf;
    let std_dev = var.sqrt();
    let std_dev = if std_dev < f64::EPSILON { 1.0 } else { std_dev };
    (mean, std_dev)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_data() {
        let (mean, std) = column_mean_std(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        assert!((mean - 3.0).abs() < 1e-10);
        assert!((std - 1.4142).abs() < 0.01);
    }

    #[test]
    fn constant_values() {
        let (mean, std) = column_mean_std(&[5.0, 5.0, 5.0]);
        assert!((mean - 5.0).abs() < 1e-10);
        assert!((std - 1.0).abs() < 1e-10);
    }

    #[test]
    fn empty_slice() {
        let (mean, std) = column_mean_std(&[]);
        assert!((mean - 0.0).abs() < 1e-10);
        assert!((std - 1.0).abs() < 1e-10);
    }

    #[test]
    fn single_element() {
        let (mean, std) = column_mean_std(&[3.0]);
        assert!((mean - 3.0).abs() < 1e-10);
        assert!((std - 1.0).abs() < 1e-10);
    }
}
