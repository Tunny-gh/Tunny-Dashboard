use super::data::get_param_numeric_values;
use super::metric_trait::SensitivityMetric;
use super::types::SensitivityResult;
use crate::dataframe::DataFrame;
use crate::math::stats::spearman_correlation;

pub struct SpearmanMetric;

impl SensitivityMetric for SpearmanMetric {
    fn compute(&self, df: &DataFrame, obj_idx: usize) -> Option<SensitivityResult> {
        let param_names = df.param_col_names().to_vec();
        let objective_names = df.objective_col_names().to_vec();
        let n = df.row_count();

        let objective_name = objective_names.get(obj_idx)?.clone();
        if n < 2 || param_names.is_empty() {
            return None;
        }

        let y: Vec<f64> = df
            .get_numeric_column(&objective_name)
            .map(|col| col[..n].to_vec())
            .unwrap_or_else(|| vec![0.0; n]);

        let spearman: Vec<Vec<f64>> = param_names
            .iter()
            .map(|name| {
                let x = get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]);
                vec![compute_spearman(&x, &y)]
            })
            .collect();

        Some(SensitivityResult {
            param_names,
            objective_names: vec![objective_name],
            spearman,
            ridge: vec![],
            rf_anova: None,
            mdi: None,
            shap: None,
            permutation: None,
        })
    }

    fn name(&self) -> &'static str {
        "Spearman"
    }
}

pub fn compute_spearman(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    // Pairwise deletion: drop any (x_i, y_i) pair where either side is
    // non-finite (NaN/Inf) before ranking, matching scipy's nan_policy='omit'.
    // Without this, rank() treats NaN as a tied trailing rank, silently
    // contaminating the correlation with values derived from missing data.
    let (fx, fy): (Vec<f64>, Vec<f64>) = x[..n]
        .iter()
        .zip(&y[..n])
        .filter(|&(&xi, &yi)| xi.is_finite() && yi.is_finite())
        .map(|(&xi, &yi)| (xi, yi))
        .unzip();

    if fx.len() < 2 {
        return f64::NAN;
    }

    spearman_correlation(&fx, &fy)
}
