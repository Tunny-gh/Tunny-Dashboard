use crate::core::math::stats::column_mean_std;

pub(super) fn col_mean_std(data: &[f64]) -> (f64, f64) {
    column_mean_std(data)
}
