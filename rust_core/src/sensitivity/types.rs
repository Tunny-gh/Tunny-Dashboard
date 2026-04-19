/// Metric type for sensitivity analysis selection.
#[derive(Debug, Clone, PartialEq)]
pub enum SensitivityMetric {
    Spearman,
    Ridge,
    RfAnova,
}

#[derive(Debug, Clone)]
pub struct SensitivityResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub spearman: Vec<Vec<f64>>,
    pub ridge: Vec<RidgeResult>,
    pub rf_anova: Option<RfAnovaResult>,
}

#[derive(Debug, Clone)]
pub struct RfAnovaResult {
    pub importances: Vec<Vec<f64>>, // [param][objective]
    pub r_squared: Vec<f64>,        // [objective]
}

#[derive(Debug, Clone)]
pub struct RidgeResult {
    pub beta: Vec<f64>,
    pub r_squared: f64,
}

#[derive(Debug, Clone)]
pub struct SobolResult {
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub first_order: Vec<Vec<f64>>,
    pub total_effect: Vec<Vec<f64>>,
    pub r_squared: Vec<f64>, // surrogate fit per objective
    pub n_samples: usize,
}
