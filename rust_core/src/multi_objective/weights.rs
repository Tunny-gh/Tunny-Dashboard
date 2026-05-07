pub fn normalize_weights(weights: &mut [f64]) {
    let sum: f64 = weights.iter().sum();
    if sum == 0.0 {
        let n = weights.len();
        if n > 0 {
            let uniform = 1.0 / n as f64;
            for w in weights.iter_mut() {
                *w = uniform;
            }
        }
        return;
    }
    for w in weights.iter_mut() {
        *w /= sum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_weights_sum_to_one() {
        let mut weights = vec![1.0, 2.0, 3.0];
        normalize_weights(&mut weights);
        let sum: f64 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn normalize_weights_already_normalized() {
        let mut weights = vec![0.5, 0.5];
        normalize_weights(&mut weights);
        assert!((weights[0] - 0.5).abs() < 1e-10);
        assert!((weights[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn normalize_weights_zero_sum_becomes_uniform() {
        let mut weights = vec![0.0, 0.0, 0.0];
        normalize_weights(&mut weights);
        let expected = 1.0 / 3.0;
        for w in &weights {
            assert!((w - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn normalize_weights_empty_no_panic() {
        let mut weights: Vec<f64> = vec![];
        normalize_weights(&mut weights);
    }
}
