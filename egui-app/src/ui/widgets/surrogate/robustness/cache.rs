//! Fit-stage compute request and cache-key logic for the robustness widget.
//!
//! The scalar/center split and the generation-ID scheme are explained on
//! `RobustnessScalarKey` and `RobustnessCacheKey` below.

use tunny_core::surrogate_opt::SurrogateModelKind;

use super::NoiseDistKind;

/// Fit-stage compute request. Consumed by poll_chart.
pub struct RobustnessFitRequest {
    pub objective_index: usize,
    pub model: SurrogateModelKind,
}

/// Scalar part of the cache key (excluding center): (fit generation ID,
/// bit representation of noise %, sample count, whether epistemic uncertainty
/// is included, noise distribution kind, bit representation of the Weibull
/// shape parameter, bit representation of LSL (None if unset), bit
/// representation of USL (None if unset)). The seed is fixed at 42, so it is
/// not included in the key.
///
/// The first element used to be `Arc::as_ptr`, but if the same address is
/// reused after deallocation, results from a different model could be shown
/// incorrectly (ABA problem). This is avoided by replacing it with a
/// monotonically increasing generation ID (`RobustnessChart::fit_generation`)
/// that advances whenever a fit is adopted.
///
/// Kept in a separate field from center (Vec<u64>). The scalar part is Copy
/// and requires no heap allocation, so recomputing/comparing it every frame
/// is cheap. center is only converted to Vec<u64> when the cache actually
/// needs to be rebuilt (on a miss); `cache_matches` compares against the
/// cached center element-by-element with zero copies.
type RobustnessScalarKey = (u64, u64, usize, bool, u8, u64, Option<u64>, Option<u64>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RobustnessCacheKey {
    scalar: RobustnessScalarKey,
    center_bits: Vec<u64>,
}

#[allow(clippy::too_many_arguments)]
fn scalar_key(
    fit_generation: u64,
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
    noise_dist: NoiseDistKind,
    weibull_shape: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> RobustnessScalarKey {
    (
        fit_generation,
        noise_pct.to_bits(),
        n_samples,
        include_epistemic,
        noise_dist as u8,
        weibull_shape.to_bits(),
        lower_spec.map(f64::to_bits),
        upper_spec.map(f64::to_bits),
    )
}

/// Builds the cache key. Because this converts center to Vec<u64>, call it
/// only on a cache miss (i.e. when actually recomputing and storing). Use
/// `cache_matches` for per-frame comparison.
#[allow(clippy::too_many_arguments)]
pub(super) fn cache_key(
    fit_generation: u64,
    center: &[f64],
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
    noise_dist: NoiseDistKind,
    weibull_shape: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> RobustnessCacheKey {
    RobustnessCacheKey {
        scalar: scalar_key(
            fit_generation,
            noise_pct,
            n_samples,
            include_epistemic,
            noise_dist,
            weibull_shape,
            lower_spec,
            upper_spec,
        ),
        center_bits: center.iter().map(|v| v.to_bits()).collect(),
    }
}

/// Determines whether the current inputs match the cached key, without
/// allocating a Vec. First compares the heap-allocation-free scalar part, and
/// only if that matches, compares center element-by-element with zero copies
/// (no new Vec<u64> is created).
#[allow(clippy::too_many_arguments)]
pub(super) fn cache_matches(
    cached: &RobustnessCacheKey,
    fit_generation: u64,
    center: &[f64],
    noise_pct: f64,
    n_samples: usize,
    include_epistemic: bool,
    noise_dist: NoiseDistKind,
    weibull_shape: f64,
    lower_spec: Option<f64>,
    upper_spec: Option<f64>,
) -> bool {
    let scalar = scalar_key(
        fit_generation,
        noise_pct,
        n_samples,
        include_epistemic,
        noise_dist,
        weibull_shape,
        lower_spec,
        upper_spec,
    );
    if cached.scalar != scalar {
        return false;
    }
    cached.center_bits.len() == center.len()
        && cached
            .center_bits
            .iter()
            .zip(center.iter())
            .all(|(&bits, &v)| bits == v.to_bits())
}
