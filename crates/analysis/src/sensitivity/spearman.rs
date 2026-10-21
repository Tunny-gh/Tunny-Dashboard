use super::metric_trait::SensitivityMetric;
use super::types::SensitivityResult;
use crate::data::finite_rows::finite_rows;
use crate::dataframe::DataFrame;
use crate::math::stats::spearman_correlation;

pub struct SpearmanMetric;

impl SensitivityMetric for SpearmanMetric {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult> {
        let (param_names, unsupported_categorical): (Vec<_>, Vec<_>) = df
            .param_col_names()
            .iter()
            .cloned()
            .partition(|name| df.get_numeric_column(name).is_some());
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        let objective_name = objective_names.get(obj_idx)?.clone();
        if n < 2 || df.param_col_names().is_empty() {
            return None;
        }

        let y: Vec<f64> = df
            .get_numeric_column(&objective_name)
            .map(|col| col.iter().take(n).copied().collect())
            .unwrap_or_else(|| vec![0.0; n]);

        let spearman: Vec<Vec<f64>> = param_names
            .iter()
            .map(|name| vec![compute_spearman(df.get_numeric_column(name).unwrap(), &y)])
            .collect();

        Some(SensitivityResult {
            param_names,
            unsupported_categorical,
            objective_names: vec![objective_name],
            spearman,
            ..Default::default()
        })
    }

    fn name(&self) -> &'static str {
        "Spearman"
    }
}

/// Pairwise finite selection before ranking; short raw inputs still return zero.
pub fn compute_spearman(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    // Pairwise deletion: drop any (x_i, y_i) pair where either side is
    // non-finite (NaN/Inf) before ranking, matching scipy's nan_policy='omit'.
    // Without this, rank() treats NaN as a tied trailing rank, silently
    // contaminating the correlation with values derived from missing data.
    let selected = finite_rows(&[x, y], n);
    let (fx, fy): (Vec<f64>, Vec<f64>) = selected.values.iter().map(|row| (row[0], row[1])).unzip();

    if fx.len() < 2 {
        return f64::NAN;
    }

    spearman_correlation(&fx, &fy)
}
