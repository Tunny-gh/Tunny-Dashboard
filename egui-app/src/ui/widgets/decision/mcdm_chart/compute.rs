use crate::state::results::{McdmMethod, WeightMode};

/// MCDM compute request payload
pub struct McdmComputeRequest {
    pub method: McdmMethod,
    pub weights: Vec<f64>,
    pub v: f64,
}

/// Cache key for MCDM results.
/// Each chart (Ranking / Scatter2D / Scatter3D / Table) references
/// `app_state.mcdm_cache` with this key so that results computed for the same
/// settings (method, weight mode, weights, v value) can be shared and reused.
///
/// Weights and v are continuous values, so they are quantized (6 decimal places)
/// to make the key Hash/Eq-able.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct McdmCacheKey {
    pub method: McdmMethod,
    pub weight_mode: WeightMode,
    pub weights_q: Vec<i64>,
    pub v_q: i64,
}

impl McdmCacheKey {
    fn quantize(x: f64) -> i64 {
        (x * 1_000_000.0).round() as i64
    }

    /// Builds a key from already-normalized weights.
    fn from_normalized(
        method: McdmMethod,
        weight_mode: WeightMode,
        weights: &[f64],
        v: f64,
    ) -> Self {
        let weights_q = weights.iter().map(|&w| Self::quantize(w)).collect();
        // v is only meaningful for VIKOR, so normalize it to 0 for other methods.
        let v_q = if method == McdmMethod::Vikor {
            Self::quantize(v)
        } else {
            0
        };
        Self {
            method,
            weight_mode,
            weights_q,
            v_q,
        }
    }

    /// Builds a key from the current settings (unnormalized weights).
    pub fn from_settings(
        method: McdmMethod,
        weight_mode: WeightMode,
        weights: &[f64],
        v: f64,
    ) -> Self {
        Self::from_normalized(method, weight_mode, &normalize_weights(weights), v)
    }

    /// Builds a key from a compute request (weights already normalized).
    pub fn from_request(req: &McdmComputeRequest, weight_mode: WeightMode) -> Self {
        Self::from_normalized(req.method, weight_mode, &req.weights, req.v)
    }
}

/// Returns normalized weights (delegates to `tunny_core::mcdm::normalize_weights`).
pub fn normalize_weights(weights: &[f64]) -> Vec<f64> {
    tunny_core::mcdm::normalize_weights(weights)
}
