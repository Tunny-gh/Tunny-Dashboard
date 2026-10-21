//! All-objective sensitivity preserves its displayable zero fallback separately
//! from single-metric readiness/errors.
use super::*;
use crate::dataframe::DataFrame;

#[derive(Clone, Copy, Debug)]
pub enum HeatmapMethod {
    Spearman,
    Ridge,
    RfAnova,
    Mdi,
    Shap,
    Permutation,
    SobolFirst,
    SobolTotal,
    Ard,
}

pub struct HeatmapData {
    pub param_names: Vec<String>,
    pub unsupported_categorical: Vec<String>,
    pub objective_names: Vec<String>,
    pub values: Vec<Vec<f64>>,
    pub signed: bool,
}

impl HeatmapMethod {
    fn metric(self) -> Option<Box<dyn SensitivityMetric>> {
        match self {
            Self::Spearman => Some(Box::new(SpearmanMetric)),
            Self::Ridge => Some(Box::new(RidgeMetric)),
            Self::RfAnova => Some(Box::new(RfAnovaMetric)),
            Self::Mdi => Some(Box::new(MdiMetric)),
            Self::Shap => Some(Box::new(ShapMetric)),
            Self::Permutation => Some(Box::new(PermutationMetric)),
            _ => None,
        }
    }

    pub fn score(self, r: &SensitivityResult, param: usize) -> f64 {
        let tree = match self {
            Self::RfAnova => r.rf_anova.as_ref().map(|x| &x.0),
            Self::Mdi => r.mdi.as_ref().map(|x| &x.0),
            Self::Shap => r.shap.as_ref().map(|x| &x.0),
            Self::Permutation => r.permutation.as_ref().map(|x| &x.0),
            _ => None,
        };
        match self {
            Self::Spearman => r
                .spearman
                .get(param)
                .and_then(|v| v.first())
                .copied()
                .unwrap_or(0.0),
            Self::Ridge => r
                .ridge
                .first()
                .and_then(|r| r.beta.get(param))
                .copied()
                .unwrap_or(0.0),
            Self::RfAnova | Self::Mdi | Self::Shap | Self::Permutation => tree
                .and_then(|r| r.importances.get(param))
                .and_then(|v| v.first())
                .copied()
                .unwrap_or(0.0),
            _ => 0.0,
        }
    }
}

pub fn compute_heatmap(df: &DataFrame, method: HeatmapMethod, n_samples: usize) -> HeatmapData {
    let signed = matches!(method, HeatmapMethod::Spearman | HeatmapMethod::Ridge);
    let (param_names, unsupported_categorical): (Vec<_>, Vec<_>) = df
        .param_col_names()
        .iter()
        .cloned()
        .partition(|name| !signed || df.get_numeric_column(name).is_some());
    let objective_names = df.objective_col_names().to_vec();
    let mut values = vec![vec![0.0; objective_names.len()]; param_names.len()];
    if matches!(
        method,
        HeatmapMethod::SobolFirst | HeatmapMethod::SobolTotal
    ) {
        if let Some(sobol) = compute_sobol_from_df(df, n_samples) {
            let data = if matches!(method, HeatmapMethod::SobolFirst) {
                sobol.first_order
            } else {
                sobol.total_effect
            };
            for (destination, source) in values.iter_mut().zip(data) {
                for (destination, value) in destination.iter_mut().zip(source) {
                    *destination = value;
                }
            }
        }
    } else if let Some(metric) = method.metric() {
        for objective in 0..objective_names.len() {
            if let Some(result) = metric.compute(df, objective) {
                for (param, row) in values.iter_mut().enumerate() {
                    row[objective] = method.score(&result, param);
                }
            }
        }
    }
    HeatmapData {
        param_names,
        unsupported_categorical,
        objective_names,
        values,
        signed,
    }
}
