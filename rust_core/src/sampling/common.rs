/// Result of a downsampling operation.
pub struct DownsampleResult {
    /// Row indices (into the active DataFrame) selected for rendering.
    pub indices: Vec<u32>,
    /// Number of Pareto Rank 1 points included.
    pub pareto_count: usize,
    /// Total row count in the active DataFrame.
    pub total_count: usize,
    /// Wall-clock duration of the sampling computation (ms), excluding Pareto
    /// pre-computation which is done once via `init_sampling`.
    pub duration_ms: f64,
}

pub(crate) fn full_result(total_count: usize, duration_ms: f64) -> DownsampleResult {
    DownsampleResult {
        indices: (0..total_count as u32).collect(),
        pareto_count: 0,
        total_count,
        duration_ms,
    }
}

/// Randomly sample `n` elements from `pool` using a fixed seed (42).
///
/// Uses a 64-bit LCG — no external crate required and cross-platform
/// reproducible.
pub(crate) fn random_sample_fixed_seed(pool: &[u32], n: usize) -> Vec<u32> {
    if n >= pool.len() {
        return pool.to_vec();
    }
    let mut buf: Vec<u32> = pool.to_vec();
    let len = buf.len();
    // Knuth's LCG constants
    let mut state: u64 = 42;
    for i in (1..len).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (state >> 33) as usize % (i + 1);
        buf.swap(i, j);
    }
    buf[..n].to_vec()
}
