//! Shared request/result types for surrogate fitting and optimization
//! (single- and multi-objective).

use super::model_selection::ModelSelectionReport;
use super::models;
use super::optimizers::OptimizerKind;
use super::validation::SurrogateValidationReport;
use super::SurrogateModelKind;

/// Input to surrogate optimization.
pub struct SurrogateOptRequest {
    /// Training data (row = trial, column = parameter), in original units.
    pub x_matrix: Vec<Vec<f64>>,
    /// Objective values (original units).
    pub y: Vec<f64>,
    /// Name of each parameter column (same order as `best_params` in the result).
    pub param_names: Vec<String>,
    /// Objective name (for display).
    pub objective_name: String,
    /// true = minimize, false = maximize.
    pub minimize: bool,
    /// Surrogate model to use.
    pub model: SurrogateModelKind,
    /// Optimizer to use.
    pub optimizer: OptimizerKind,
    /// Column indices of the two parameters for the response-surface slice
    /// through the optimum (for display).
    pub slice_params: Option<(usize, usize)>,
    /// Number of points along one side of the slice grid.
    pub n_grid: usize,
    /// Constraint data (empty = unconstrained).
    pub constraints: Vec<ConstraintData>,
}

/// Data for a single constraint passed to surrogate fitting.
///
/// Optuna's constraint convention: value ≤ 0 is feasible.
pub struct ConstraintData {
    /// Constraint name (for display/logging).
    pub name: String,
    /// Constraint value per trial (same row order as `x_matrix`).
    pub values: Vec<f64>,
}

/// Input to surrogate fitting + validation.
pub struct SurrogateFitRequest {
    pub x_matrix: Vec<Vec<f64>>,
    pub y: Vec<f64>,
    pub param_names: Vec<String>,
    pub objective_name: String,
    pub model: SurrogateModelKind,
    /// When true, ignores `model` and cross-validates `AUTO_CANDIDATES` to
    /// automatically select and fit the best model (the outcome is recorded in
    /// `TrainedSurrogate.model_selection`).
    pub auto_select: bool,
    /// Constraint data (empty = unconstrained). Each element is one constraint.
    pub constraints: Vec<ConstraintData>,
    /// Row indices (into `x_matrix`) to prioritize as inducing points. Empty =
    /// uniform (default). Used to concentrate the GP's inducing points on
    /// Pareto-front trials in the multi-objective case. Has no effect when N is
    /// at or below the GP's inducing-point cap (100), since Z = X uses every
    /// point anyway.
    pub priority_rows: Vec<usize>,
    /// Declared range (low, high) per parameter column (derived from the log;
    /// same order as `param_names`). When `Some(vec)`, each column is normalized
    /// using this range instead of the observed min/max, so the optimization
    /// search box (normalized space [0,1]^d) matches the true variable range.
    /// Falls back to the observed range for columns that are `None`, or when the
    /// whole field is `None`.
    pub param_bounds: Option<Vec<Option<(f64, f64)>>>,
}

/// A validated fit result, reused for optimization.
pub struct TrainedSurrogate {
    pub(crate) surrogate: models::FittedSurrogate,
    pub model_kind: SurrogateModelKind,
    pub param_names: Vec<String>,
    pub objective_name: String,
    /// Original data used for fitting (used as the optimization start point).
    pub(crate) x_matrix: Vec<Vec<f64>>,
    pub(crate) y: Vec<f64>,
    pub validation: SurrogateValidationReport,
    /// Relative parameter importance derived from the ARD length scales (same
    /// order as `param_names`, summing to 1.0).
    ///
    /// Some only for GP (single SGP: FITC / VFE). None for MoE / Ridge / LightGBM.
    /// The importance corresponds to the model's input dimensions (= columns of
    /// `x_matrix`), whose column order matches `param_names` (since
    /// `fit_surrogate` never reorders columns).
    pub param_importance: Option<Vec<f64>>,
    /// Constraint names (same order as `constraint_models`; empty = unconstrained).
    pub constraint_names: Vec<String>,
    /// Fitted surrogate per constraint (same order as `constraint_names`).
    pub(crate) constraint_models: Vec<models::FittedSurrogate>,
    /// Constraint value per trial (row = trial, column = constraint; same order
    /// as `constraint_names`). Used to compute the feasible incumbent.
    pub(crate) constraint_values: Vec<Vec<f64>>,
    /// History of automatic model selection (`auto_select = true`). None when
    /// the model was specified manually. `model_kind` holds the concrete model
    /// kind that was chosen.
    pub model_selection: Option<ModelSelectionReport>,
}

/// Configuration for the optimization stage (run against an already-fitted model).
pub struct SurrogateOptimizeSpec {
    pub minimize: bool,
    pub optimizer: OptimizerKind,
    pub slice_params: Option<(usize, usize)>,
    pub n_grid: usize,
}

/// A 2D slice of the response surface through the optimum (other dimensions
/// fixed at the optimum).
#[derive(Debug, Clone)]
pub struct SurfaceSlice {
    pub param_x_idx: usize,
    pub param_y_idx: usize,
    /// X-axis grid values (original units).
    pub x_values: Vec<f64>,
    /// Y-axis grid values (original units).
    pub y_values: Vec<f64>,
    /// Predicted value grid. `z_values[i][j] = f(x_values[i], y_values[j])`.
    pub z_values: Vec<Vec<f64>>,
    /// Grid of predicted standard deviations (original units, same shape as
    /// `z_values`). Some only for models with a posterior variance (GP family);
    /// None for Ridge / LightGBM.
    pub z_std: Option<Vec<Vec<f64>>>,
}

/// Result of surrogate optimization.
#[derive(Debug, Clone)]
pub struct SurrogateOptResult {
    /// Parameter values at the estimated optimum (original units, same order as
    /// `param_names`).
    pub best_params: Vec<f64>,
    /// Surrogate prediction at the estimated optimum (original units).
    pub best_value: f64,
    /// Predicted standard deviation (Gaussian-process models only; None for Ridge).
    pub predicted_std: Option<f64>,
    /// Coefficient of determination of the surrogate on the training data.
    pub r_squared: f64,
    /// Response-surface slice through the optimum (only when `slice_params` is
    /// given).
    pub slice: Option<SurfaceSlice>,
    /// Best value among the observed data (original units). The minimum when
    /// minimizing, the maximum when maximizing.
    pub best_observed_value: f64,
    /// Predicted value of each constraint surrogate at the estimated optimum
    /// (original units, same order as `constraint_names`). Empty when
    /// unconstrained (`constraint_names` is empty).
    pub predicted_constraints: Vec<f64>,
    /// Feasibility probability at the estimated optimum (0.0-1.0). None when
    /// unconstrained.
    pub feasibility_probability: Option<f64>,
}

/// Input to multi-objective surrogate optimization.
pub struct SurrogateMultiOptRequest {
    /// Training data (row = trial, column = parameter), in original units.
    pub x_matrix: Vec<Vec<f64>>,
    /// Value column per objective. `ys[k][i]` = value of objective k for trial i.
    pub ys: Vec<Vec<f64>>,
    /// Name of each parameter column.
    pub param_names: Vec<String>,
    /// Objective names, same order as `ys`.
    pub objective_names: Vec<String>,
    /// Per-objective true = minimize. Same length as `ys`.
    pub minimize: Vec<bool>,
    /// Surrogate model to use.
    pub model: SurrogateModelKind,
    /// Column indices of the two parameters for the response-surface slice
    /// (for display).
    pub slice_params: Option<(usize, usize)>,
    /// Number of points along one side of the slice grid.
    pub n_grid: usize,
}

/// A single point on the predicted Pareto front.
#[derive(Debug, Clone)]
pub struct ParetoFrontPoint {
    /// Parameter values (original units, same order as `param_names`).
    pub params: Vec<f64>,
    /// Surrogate-predicted value for each objective (original units, same order
    /// as `objective_names`).
    pub values: Vec<f64>,
}

/// Result of multi-objective surrogate optimization.
#[derive(Debug, Clone)]
pub struct SurrogateMultiOptResult {
    /// Predicted Pareto front, sorted ascending by the first objective's value.
    pub front: Vec<ParetoFrontPoint>,
    /// Training-data coefficient of determination per objective (same order as
    /// `objective_names`).
    pub r_squared: Vec<f64>,
    /// Response-surface slice per objective (only when `slice_params` is given,
    /// same order as `objective_names`; empty when unspecified/invalid).
    pub slices: Vec<SurfaceSlice>,
}

/// Configuration for the multi-objective optimization stage (run against a set
/// of already-fitted models).
pub struct SurrogateMultiOptimizeSpec {
    /// Per-objective true = minimize. Same length as `trained`.
    pub minimize: Vec<bool>,
    pub slice_params: Option<(usize, usize)>,
    pub n_grid: usize,
}

#[cfg(test)]
impl TrainedSurrogate {
    /// For tests: assembles a `TrainedSurrogate` from an analytic mock surrogate.
    ///
    /// An entry point for testing "surface-consuming" logic (optimization,
    /// slicing, multi-objective fronts, acquisition functions, feasibility)
    /// without ever running a GP fit. Pass a known surface built with
    /// [`models::FittedSurrogate::analytic`] as `surrogate`. `x_matrix` / `y`
    /// are used only to compute the optimization start point (observed best);
    /// the surface itself is defined entirely by `surrogate`.
    pub(crate) fn analytic_mock(
        x_matrix: Vec<Vec<f64>>,
        y: Vec<f64>,
        surrogate: models::FittedSurrogate,
    ) -> Self {
        let n_dims = surrogate.col_stats.len();
        TrainedSurrogate {
            surrogate,
            model_kind: SurrogateModelKind::GpFitc,
            param_names: (0..n_dims).map(|d| format!("x{d}")).collect(),
            objective_name: "obj".to_string(),
            x_matrix,
            y,
            validation: SurrogateValidationReport::placeholder(),
            param_importance: None,
            constraint_names: vec![],
            constraint_models: vec![],
            constraint_values: vec![],
            model_selection: None,
        }
    }

    /// Adds one constraint surrogate to an analytic mock (used together with
    /// [`analytic_mock`]). `values` is the constraint value per trial (same row
    /// order as `x_matrix`).
    pub(crate) fn with_analytic_constraint(
        mut self,
        name: &str,
        values: Vec<f64>,
        model: models::FittedSurrogate,
    ) -> Self {
        self.constraint_names.push(name.to_string());
        self.constraint_models.push(model);
        if self.constraint_values.len() != values.len() {
            self.constraint_values = values.iter().map(|&v| vec![v]).collect();
        } else {
            for (row, &v) in self.constraint_values.iter_mut().zip(values.iter()) {
                row.push(v);
            }
        }
        self
    }
}
