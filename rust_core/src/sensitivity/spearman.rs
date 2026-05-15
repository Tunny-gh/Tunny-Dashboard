use crate::core::math::stats::pearson_correlation;
use crate::dataframe::DataFrame;
use super::data::get_param_numeric_values;
use super::metric_trait::SensitivityMetric;
use super::types::SensitivityResult;

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

fn rank(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return vec![];
    }

    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| {
        let va = values[a];
        let vb = values[b];
        match (va.is_nan(), vb.is_nan()) {
            (true, _) => std::cmp::Ordering::Greater,
            (_, true) => std::cmp::Ordering::Less,
            _ => va.partial_cmp(&vb).unwrap(),
        }
    });

    let mut ranks = vec![0.0f64; n];
    let mut i = 0;

    while i < n {
        let val = values[indices[i]];
        if val.is_nan() {
            let avg = (i as f64 + 1.0 + n as f64) / 2.0;
            for k in i..n {
                ranks[indices[k]] = avg;
            }
            break;
        }

        let mut j = i + 1;
        while j < n && values[indices[j]] == val {
            j += 1;
        }

        let avg_rank = (i as f64 + 1.0 + j as f64) / 2.0;
        for k in i..j {
            ranks[indices[k]] = avg_rank;
        }
        i = j;
    }

    ranks
}

pub fn compute_spearman(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }

    let rx = rank(&x[..n]);
    let ry = rank(&y[..n]);

    pearson_correlation(&rx, &ry)
}
