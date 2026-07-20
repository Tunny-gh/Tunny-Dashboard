//! Pure statistics helpers for the scatter matrix: column ranges, histograms,
//! correlation coefficients, feasibility splitting, and index downsampling.
//! None of these touch egui — they operate purely on numeric slices/indices
//! so they're cheap to unit test and to cache in `MatrixStatsCache`.

use crate::ui::widgets::common::range_math::value_range;

/// Pure function that resolves the objective name used for coloring.
/// - If `selected` is present in `obj_names`, returns that name.
/// - If `None` or the name doesn't exist, returns the first element (`obj_names[0]`).
/// - If `obj_names` is empty, returns `None`.
pub(super) fn resolve_color_objective<'a>(
    selected: &Option<String>,
    obj_names: &'a [String],
) -> Option<&'a str> {
    if obj_names.is_empty() {
        return None;
    }
    if let Some(name) = selected {
        if let Some(found) = obj_names.iter().find(|n| *n == name) {
            return Some(found.as_str());
        }
    }
    Some(obj_names[0].as_str())
}

/// Builds feasible / infeasible index lists from feasibility.
/// For a Study without constraints (feas.has_constraints() == false), all entries
/// are treated as feasible.
pub(super) fn split_feasibility_indices(
    n: usize,
    feas: tunny_core::dataframe::Feasibility<'_>,
) -> (Vec<u32>, Vec<u32>) {
    let (f_idx, inf_idx) = feas.partition_indices(n);
    let feasible: Vec<u32> = f_idx.into_iter().map(|i| i as u32).collect();
    let infeasible: Vec<u32> = inf_idx.into_iter().map(|i| i as u32).collect();
    (feasible, infeasible)
}

/// Evenly downsamples an index list to at most `cap` entries.
/// If already at or below `cap`, returns a plain copy; otherwise downsamples with
/// an evenly spaced stride, reducing the point count while preserving the overall
/// distribution shape.
pub fn downsample_indices_to_cap(indices: &[u32], cap: usize) -> Vec<u32> {
    if cap == 0 {
        return Vec::new();
    }
    if indices.len() <= cap {
        return indices.to_vec();
    }
    // Round the stride up so the result never exceeds cap.
    let step = indices.len().div_ceil(cap);
    indices.iter().step_by(step).copied().collect()
}

/// Computes histogram bin counts.
pub(super) fn compute_histogram(data: &[f64], n_bins: usize) -> Vec<usize> {
    if data.is_empty() || n_bins == 0 {
        return vec![0; n_bins];
    }
    // `data` is guaranteed non-empty by the emptiness check above.
    let (v_min, v_max) = value_range(data.iter().cloned()).unwrap();
    if (v_max - v_min).abs() < f64::EPSILON {
        let mut bins = vec![0usize; n_bins];
        bins[n_bins / 2] = data.len();
        return bins;
    }
    let mut bins = vec![0usize; n_bins];
    for &v in data {
        let idx = ((v - v_min) / (v_max - v_min) * n_bins as f64) as usize;
        let idx = idx.min(n_bins - 1);
        bins[idx] += 1;
    }
    bins
}

/// Computes the Pearson correlation coefficient.
///
/// Delegates the actual computation to `tunny_core::math::stats::pearson_correlation`.
/// For cell display purposes, however, degenerate cases (fewer than 2 elements, or
/// near-zero variance) return 0.0 instead of NaN, and the result is clamped to
/// [-1, 1] to account for floating-point error.
pub(super) fn compute_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    let r = tunny_core::math::stats::pearson_correlation(&x[..n], &y[..n]);
    if r.is_nan() {
        0.0
    } else {
        r.clamp(-1.0, 1.0)
    }
}

/// Returns the min/max of a column (used for coordinate transforms in scatter cells).
/// The `f64::min`/`f64::max` reduction ignores NaN and propagates Inf (preserving
/// the previous behavior).
pub(super) fn col_min_max(data: &[f64]) -> (f64, f64) {
    value_range(data.iter().cloned()).unwrap_or((f64::INFINITY, f64::NEG_INFINITY))
}
