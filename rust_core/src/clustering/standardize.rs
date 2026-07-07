//! clustering 内共通の列 z-score 標準化ヘルパ。
//!
//! hierarchical / som / pca で重複していた標準化処理の一本化。分散の
//! 自由度補正が異なる（hierarchical・som は母分散 n、pca は不偏分散 n-1）
//! ため `ddof` でパラメータ化する。

/// 各列を平均 0・分散 1 に in-place で標準化する。
///
/// - `ddof`: 分散の自由度補正。0 なら母分散（分母 n）、1 なら不偏分散（分母 n-1）。
/// - 標準偏差が 1e-12 以下の列（実質的に分散ゼロ）は全要素を 0 に写像する。
/// - 戻り値は `(列平均, 列標準偏差)`。標準偏差は補正後の生の値を返す
///   （分散ゼロ列でも 0 に丸めない。逆変換側で同じ閾値判定を行うこと）。
///
/// 前提: 全行が同じ長さであること（呼び出し側で検証する）。
pub(super) fn standardize_columns(x: &mut [Vec<f64>], ddof: usize) -> (Vec<f64>, Vec<f64>) {
    let n = x.len();
    if n == 0 || x[0].is_empty() {
        return (Vec::new(), Vec::new());
    }
    let p = x[0].len();
    let denom = n.saturating_sub(ddof).max(1) as f64;
    let mut means = vec![0.0f64; p];
    let mut stds = vec![0.0f64; p];
    for j in 0..p {
        let mean = x.iter().map(|r| r[j]).sum::<f64>() / n as f64;
        let var = x.iter().map(|r| (r[j] - mean).powi(2)).sum::<f64>() / denom;
        let std = var.sqrt();
        means[j] = mean;
        stds[j] = std;
        for row in x.iter_mut() {
            row[j] = if std > 1e-12 {
                (row[j] - mean) / std
            } else {
                0.0
            };
        }
    }
    (means, stds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn population_variance_standardizes_to_unit() {
        let mut x = vec![vec![1.0], vec![2.0], vec![3.0]];
        let (means, stds) = standardize_columns(&mut x, 0);
        assert!((means[0] - 2.0).abs() < 1e-12);
        // 母分散 = 2/3 → std = sqrt(2/3)
        assert!((stds[0] - (2.0f64 / 3.0).sqrt()).abs() < 1e-12);
        let mean_after: f64 = x.iter().map(|r| r[0]).sum::<f64>() / 3.0;
        let var_after: f64 = x.iter().map(|r| (r[0] - mean_after).powi(2)).sum::<f64>() / 3.0;
        assert!(mean_after.abs() < 1e-12);
        assert!((var_after - 1.0).abs() < 1e-12);
    }

    #[test]
    fn ddof_one_uses_sample_variance() {
        let mut x = vec![vec![1.0], vec![2.0], vec![3.0]];
        let (_, stds) = standardize_columns(&mut x, 1);
        // 不偏分散 = 1.0 → std = 1.0
        assert!((stds[0] - 1.0).abs() < 1e-12);
    }

    #[test]
    fn zero_variance_column_maps_to_zero() {
        let mut x = vec![vec![5.0, 1.0], vec![5.0, 2.0]];
        standardize_columns(&mut x, 0);
        assert_eq!(x[0][0], 0.0);
        assert_eq!(x[1][0], 0.0);
        assert!(x[0][1] != 0.0);
    }

    #[test]
    fn empty_input_is_noop() {
        let mut x: Vec<Vec<f64>> = vec![];
        let (means, stds) = standardize_columns(&mut x, 0);
        assert!(means.is_empty());
        assert!(stds.is_empty());
    }
}
