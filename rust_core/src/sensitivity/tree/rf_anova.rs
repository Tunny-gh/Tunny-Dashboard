use super::super::constants::{
    RF_ANOVA_MAX_ROWS, RF_ANOVA_RF_MAX_DEPTH, RF_ANOVA_RF_MIN_SAMPLES_LEAF, RF_ANOVA_RF_TREES,
    RF_ANOVA_SEED,
};
use super::common::{run_importances_pipeline, PreparedData};
use super::fanova::{compute_fanova, FanovaConfig};

/// 前処理済みデータから fANOVA 重要度を計算する（`metrics::RfAnovaMetric` からも呼ばれる）。
///
/// Hutter et al. (2014) の functional ANOVA: 独自実装の CART 回帰フォレストを訓練データ
/// (train split) で学習し、各木の葉ノード区間（box）から主効果の分散を厳密に周辺化分解する。
/// R² は評価データ (eval split) に対するフォレスト予測から算出する。
pub(in crate::sensitivity) fn compute_from_prepared(
    data: &PreparedData,
) -> Option<(Vec<f64>, f64)> {
    let (x_train, x_eval, y_train, y_eval) = data.split();
    let config = FanovaConfig {
        n_trees: RF_ANOVA_RF_TREES,
        max_depth: RF_ANOVA_RF_MAX_DEPTH,
        min_samples_leaf: RF_ANOVA_RF_MIN_SAMPLES_LEAF,
        seed: RF_ANOVA_SEED,
    };
    compute_fanova(x_train, y_train, x_eval, y_eval, &config)
}

pub fn compute_rf_anova_importances(x_matrix: &[Vec<f64>], y: &[f64]) -> (Vec<f64>, f64) {
    run_importances_pipeline(
        x_matrix,
        y,
        RF_ANOVA_MAX_ROWS,
        RF_ANOVA_SEED,
        RF_ANOVA_SEED.wrapping_add(1),
        compute_from_prepared,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::rng::SeededRng;

    /// t2: y = 4*x0 + 小さいノイズ。x0 が支配的な特徴量になるはず。
    fn make_dominant_xy(n: usize, n_feats: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let mut rng = SeededRng::from_seed(123);
        let x: Vec<Vec<f64>> = (0..n)
            .map(|_| (0..n_feats).map(|_| rng.next_f64()).collect())
            .collect();
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, row)| 4.0 * row[0] + 0.05 * (i as f64).sin())
            .collect();
        (x, y)
    }

    #[test]
    fn dominant_feature_gets_majority_importance() {
        let (x, y) = make_dominant_xy(200, 3);
        let (importances, _r2) = compute_rf_anova_importances(&x, &y);
        assert_eq!(importances.len(), 3);
        assert!(importances[0] > 0.7, "importances={importances:?}");
        assert!(
            importances[0] > importances[1] && importances[0] > importances[2],
            "importances={importances:?}"
        );
    }

    /// t3: 同一入力での 2 回の呼び出しはビット単位で一致する（rayon 並列化があっても
    /// 集約は木の順序を保った逐次ループで行うため決定的）。
    #[test]
    fn deterministic_across_repeated_calls() {
        let (x, y) = make_dominant_xy(120, 3);
        let (a, r2_a) = compute_rf_anova_importances(&x, &y);
        let (b, r2_b) = compute_rf_anova_importances(&x, &y);
        assert_eq!(a, b, "importances should be bit-identical");
        assert_eq!(r2_a, r2_b, "r_squared should be bit-identical");
    }

    /// t4a: y が定数の場合、全木で分割が発生せず全分散もゼロになる。パニックせず
    /// 重要度はすべて 0.0 になることを確認する。
    #[test]
    fn constant_y_does_not_panic() {
        let x: Vec<Vec<f64>> = (0..30).map(|i| vec![i as f64 / 30.0, 0.5]).collect();
        let y = vec![7.0; 30];
        let (importances, r2) = compute_rf_anova_importances(&x, &y);
        assert_eq!(importances.len(), 2);
        assert!(
            importances.iter().all(|&v| v == 0.0),
            "importances={importances:?}"
        );
        assert_eq!(r2, 0.0);
    }

    /// t4b: 定数の特徴量列があってもパニックせず、可変な特徴量が支配的重要度を持つ。
    #[test]
    fn constant_feature_column_does_not_panic() {
        let mut rng = SeededRng::from_seed(9);
        let x: Vec<Vec<f64>> = (0..40).map(|_| vec![rng.next_f64(), 1.0]).collect();
        let y: Vec<f64> = x.iter().map(|row| 3.0 * row[0]).collect();
        let (importances, _r2) = compute_rf_anova_importances(&x, &y);
        assert_eq!(importances.len(), 2);
        assert!(
            importances[0] > importances[1],
            "importances={importances:?}"
        );
    }
}
