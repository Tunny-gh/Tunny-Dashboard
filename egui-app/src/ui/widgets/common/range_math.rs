//! 値域正規化・min/max 集計の共有ヘルパー。
//!
//! 散布図・PDP・ヒートマップなど複数のウィジェットで重複していた
//! 「[0,1] 正規化」「イテレータからの値域集計」「退化範囲（min==max）の拡張」
//! ロジックをここに集約する。各呼び出し元固有のフォールバック挙動（空/非有限値の
//! 扱い）は呼び出し側に残し、このモジュールでは共通の集計・正規化のみを提供する。

/// 値を [0, 1] に正規化する（クランプ済み）。`v_max == v_min`（退化範囲）の場合は 0.5 を返す。
pub fn normalize01(v: f64, min: f64, max: f64) -> f32 {
    if (max - min).abs() < f64::EPSILON {
        return 0.5;
    }
    ((v - min) / (max - min)).clamp(0.0, 1.0) as f32
}

/// 数値イテレータから [min, max] を計算する（NaN は無視、Inf は集計に含む）。
/// イテレータが空の場合のみ `None` を返す。非有限値（Inf）のフィルタや、
/// 退化範囲の扱いは呼び出し側の判断に委ねる（`expand_degenerate` 参照）。
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

/// 数値イテレータから非有限値（NaN・Inf）を除いた [min, max] を計算する。
/// 有限値が 1 件も無い（空を含む）場合は `None` を返す。
/// 退化範囲の扱いは呼び出し側の判断に委ねる（`expand_degenerate` 参照）。
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

/// 退化範囲（`max - min` がほぼ 0）を ±1 に広げる。それ以外はそのまま返す。
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
        // Inf のフィルタは呼び出し側の責務。ここでは集計に含まれる。
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
