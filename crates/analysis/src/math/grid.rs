/// Generate `n` equally-spaced values in `[min, max]`.
pub(crate) fn linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linspace_normal() {
        let v = linspace(0.0, 1.0, 5);
        assert_eq!(v.len(), 5);
        assert!((v[0] - 0.0).abs() < 1e-10);
        assert!((v[4] - 1.0).abs() < 1e-10);
        assert!((v[2] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn linspace_zero_returns_empty() {
        assert!(linspace(0.0, 1.0, 0).is_empty());
    }

    #[test]
    fn linspace_one_returns_midpoint() {
        let v = linspace(0.0, 10.0, 1);
        assert_eq!(v.len(), 1);
        assert!((v[0] - 5.0).abs() < 1e-10);
    }
}
