/// Linear congruential generator: returns next 64-bit value and updates state.
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Reusable LCG RNG struct shared with kriging and sensitivity modules.
pub(crate) struct Lcg {
    state: u64,
}

impl Lcg {
    pub(crate) fn new(seed: u64) -> Self {
        Lcg {
            state: seed ^ 0xcafef00dd15ea5e5,
        }
    }

    /// Returns a random `usize` in `[0, n)`.
    pub(crate) fn next_usize(&mut self, n: usize) -> usize {
        lcg_next(&mut self.state) as usize % n
    }
}
