use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// Deterministic ChaCha8-based RNG wrapper.
///
/// Replaces the custom LCG/xorshift PRNGs with a cryptographically
/// secure generator while preserving seed-based reproducibility.
pub(crate) struct SeededRng {
    rng: ChaCha8Rng,
}

impl SeededRng {
    pub(crate) fn from_seed(seed: u64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Returns a random `usize` in `[0, n)`.
    pub(crate) fn next_usize(&mut self, n: usize) -> usize {
        self.rng.random_range(0..n)
    }

    /// Returns a random `f64` in `[0, 1)`.
    pub(crate) fn next_f64(&mut self) -> f64 {
        self.rng.random()
    }

    pub(crate) fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.next_usize(i + 1);
            slice.swap(i, j);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SeededRng;

    #[test]
    fn tc_301_01_seeded_rng_reproducibility() {
        // 【テスト目的】: 同一シードで同一乱数列が再現されることを確認
        let mut rng1 = SeededRng::from_seed(42);
        let mut rng2 = SeededRng::from_seed(42);
        for _ in 0..10 {
            let v1 = rng1.next_f64();
            let v2 = rng2.next_f64();
            assert_eq!(v1, v2, "同一シードで同一乱数列が再現されること");
            assert!(v1 >= 0.0 && v1 < 1.0, "next_f64 は [0,1) の範囲内");
        }
    }

    #[test]
    fn tc_301_02_different_seeds_produce_different_sequences() {
        let mut rng1 = SeededRng::from_seed(1);
        let mut rng2 = SeededRng::from_seed(2);
        let v1 = rng1.next_f64();
        let v2 = rng2.next_f64();
        assert_ne!(v1, v2, "異なるシードで異なる乱数が生成されること");
    }

    #[test]
    fn tc_301_03_next_f64_range() {
        // 【テスト目的】: next_f64() が常に [0, 1) を返すことを確認
        let mut rng = SeededRng::from_seed(0);
        for _ in 0..1000 {
            let v = rng.next_f64();
            assert!(v >= 0.0 && v < 1.0, "next_f64 out of [0,1): {}", v);
        }
    }

    #[test]
    fn tc_301_04_next_usize_range() {
        // 【テスト目的】: next_usize(n) が [0, n) を返すことを確認
        let mut rng = SeededRng::from_seed(0);
        for _ in 0..1000 {
            let v = rng.next_usize(100);
            assert!(v < 100, "next_usize out of [0, 100): {}", v);
        }
    }

    #[test]
    fn tc_b01_seed_zero() {
        let mut rng = SeededRng::from_seed(0);
        let v = rng.next_f64();
        assert!(v >= 0.0 && v < 1.0, "seed=0 で next_f64 が [0,1): {}", v);
    }

    #[test]
    fn tc_b02_next_usize_bound_one() {
        let mut rng = SeededRng::from_seed(42);
        for _ in 0..100 {
            let v = rng.next_usize(1);
            assert_eq!(v, 0, "next_usize(1) は常に 0");
        }
    }
}
