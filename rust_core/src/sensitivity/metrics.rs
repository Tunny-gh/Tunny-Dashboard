use super::constants::{
    MDI_MAX_ROWS, MDI_SEED, PFI_MAX_ROWS, PFI_SEED_BASE, PFI_SPLIT_SEED, RF_ANOVA_MAX_ROWS,
    RF_ANOVA_SEED, SHAP_MAX_ROWS, SHAP_SEED,
};
use super::tree_common::PreparedData;

/// ツリーベースの感度分析メトリクス共通トレイト。
///
/// `prepare_training_data` で前処理済みの `PreparedData` を受け取り、
/// (feature_importances, r_squared) を返す。
/// importances の合計は 1.0 になるよう正規化すること（またはすべて 0.0）。
/// データ不足や LightGBM の訓練失敗時は `None` を返す。
pub(crate) trait TreeMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)>;
    /// ダウンサンプリング上限行数
    fn max_rows(&self) -> usize;
    /// データサンプリング用シード
    fn data_seed(&self) -> u64;
    /// ホールドアウト分割シャッフル用シード
    fn split_seed(&self) -> u64;
}

pub(crate) struct RfAnovaMetric;
pub(crate) struct MdiMetric;
pub(crate) struct ShapMetric;
pub(crate) struct PermutationMetric;

impl TreeMetric for RfAnovaMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::rf_anova::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        RF_ANOVA_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        RF_ANOVA_SEED
    }
    fn split_seed(&self) -> u64 {
        RF_ANOVA_SEED.wrapping_add(1)
    }
}

impl TreeMetric for MdiMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::mdi::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        MDI_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        MDI_SEED
    }
    fn split_seed(&self) -> u64 {
        MDI_SEED.wrapping_add(1)
    }
}

impl TreeMetric for ShapMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::shap::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        SHAP_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        SHAP_SEED
    }
    fn split_seed(&self) -> u64 {
        SHAP_SEED.wrapping_add(1)
    }
}

impl TreeMetric for PermutationMetric {
    fn compute_importances(&self, data: &PreparedData) -> Option<(Vec<f64>, f64)> {
        super::permutation::compute_from_prepared(data)
    }
    fn max_rows(&self) -> usize {
        PFI_MAX_ROWS
    }
    fn data_seed(&self) -> u64 {
        PFI_SEED_BASE
    }
    fn split_seed(&self) -> u64 {
        PFI_SPLIT_SEED
    }
}
