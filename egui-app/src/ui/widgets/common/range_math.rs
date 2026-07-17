//! Shared helpers for value-range normalization and min/max aggregation.
//!
//! Consolidates the "[0,1] normalization", "value-range aggregation from
//! an iterator", and "expansion of a degenerate range (min==max)" logic
//! that was duplicated across multiple widgets (scatter plots, PDP,
//! heatmaps, etc.). Fallback behavior specific to each caller (handling of
//! empty / non-finite values) remains with the caller; this module
//! provides only the common aggregation and normalization.

/// Normalizes a value to [0, 1] (clamped). Returns 0.5 when `v_max ==
/// v_min` (a degenerate range).
pub fn normalize01(v: f64, min: f64, max: f64) -> f32 {
    if (max - min).abs() < f64::EPSILON {
        return 0.5;
    }
    ((v - min) / (max - min)).clamp(0.0, 1.0) as f32
}

/// Computes [min, max] from a numeric iterator (NaN is ignored, Inf is
/// included in the aggregation). Returns `None` only when the iterator is
/// empty. Filtering of non-finite values (Inf) and handling of a
/// degenerate range are left to the caller's judgment (see
/// `expand_degenerate`).
pub fn value_range<I: IntoIterator<Item = f64>>(vals: I) -> Option<(f64, f64)> {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    let mut any = false;
    for v in vals {
        any = true;
        mn = mn.min(v);
        mx = mx.max(v);
    }
    any.then_some((mn, mx))
}

/// Computes [min, max] from a numeric iterator, excluding non-finite
/// values (NaN, Inf). Returns `None` when there are no finite values at
/// all (including when the iterator is empty). Handling of a degenerate
/// range is left to the caller's judgment (see `expand_degenerate`).
pub fn finite_value_range<I: IntoIterator<Item = f64>>(vals: I) -> Option<(f64, f64)> {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for v in vals {
        if v.is_finite() {
            mn = mn.min(v);
            mx = mx.max(v);
        }
    }
    (mn.is_finite() && mx.is_finite()).then_some((mn, mx))
}

/// Expands a degenerate range (`max - min` nearly 0) by ±1. Otherwise
/// returns it unchanged.
pub fn expand_degenerate(min: f64, max: f64) -> (f64, f64) {
    if (max - min).abs() < f64::EPSILON {
        (min - 1.0, max + 1.0)
    } else {
        (min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize01_min_maps_to_zero() {
        assert!((normalize01(0.0, 0.0, 10.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn normalize01_max_maps_to_one() {
        assert!((normalize01(10.0, 0.0, 10.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize01_degenerate_range_returns_half() {
        assert!((normalize01(5.0, 5.0, 5.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn normalize01_clamps_out_of_range() {
        assert!((normalize01(-1.0, 0.0, 1.0) - 0.0).abs() < 1e-6);
        assert!((normalize01(2.0, 0.0, 1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn value_range_empty_is_none() {
        assert_eq!(value_range(std::iter::empty()), None);
    }

    #[test]
    fn value_range_basic() {
        assert_eq!(value_range([1.0, 3.0, 2.0]), Some((1.0, 3.0)));
    }

    #[test]
    fn value_range_ignores_nan() {
        assert_eq!(value_range([1.0, f64::NAN, 4.0]), Some((1.0, 4.0)));
    }

    #[test]
    fn value_range_includes_infinite() {
        // Filtering Inf is the caller's responsibility; here it is included in the aggregation.
        let (mn, mx) = value_range([1.0, f64::INFINITY]).unwrap();
        assert_eq!(mn, 1.0);
        assert!(mx.is_infinite());
    }

    #[test]
    fn finite_value_range_ignores_non_finite() {
        let vals = [1.0, f64::NAN, 5.0, f64::INFINITY, -2.0];
        assert_eq!(finite_value_range(vals), Some((-2.0, 5.0)));
    }

    #[test]
    fn finite_value_range_empty_is_none() {
        assert_eq!(finite_value_range(std::iter::empty()), None);
    }

    #[test]
    fn finite_value_range_all_non_finite_is_none() {
        assert_eq!(finite_value_range([f64::NAN, f64::INFINITY]), None);
    }

    #[test]
    fn expand_degenerate_equal_expands_by_one() {
        assert_eq!(expand_degenerate(5.0, 5.0), (4.0, 6.0));
    }

    #[test]
    fn expand_degenerate_distinct_unchanged() {
        assert_eq!(expand_degenerate(1.0, 3.0), (1.0, 3.0));
    }
}
