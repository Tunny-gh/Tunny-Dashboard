//! Computes ARD (Automatic Relevance Determination) parameter importance directly
//! from a DataFrame.
//!
//! Fits a GP-FITC surrogate and extracts relative parameter importance from its ARD
//! length scales. Used as one method for sensitivity analysis (the Importance
//! widget). Like Sobol's `sensitivity::compute_sobol_from_df`, it provides a
//! "DataFrame -> single-objective importance" entry point. Both numeric and
//! categorical parameter columns are numericized via `get_param_numeric_values`
//! before being passed to the GP (categories are label-encoded, same as Sobol).

use super::{fit_surrogate_with_validation, SurrogateFitRequest, SurrogateModelKind};
use crate::dataframe::DataFrame;
use crate::sensitivity::get_param_numeric_values;

/// Result of ARD parameter importance (for a single objective).
pub struct ArdImportanceResult {
    /// Parameter names (same order as `importances`).
    pub param_names: Vec<String>,
    /// Relative importance of each parameter (sums to 1.0, same order as `param_names`).
    pub importances: Vec<f64>,
    /// Cross-validation R² of the fitted GP (a rough gauge of the importance's reliability).
    pub r_squared: f64,
}

/// Fits GP-FITC for the given objective (`obj_idx`) and returns the ARD-derived
/// parameter importance.
///
/// Returns `None` when there are too few trials to fit, or the GP does not expose
/// ARD (e.g. training failure).
pub fn compute_ard_importance_from_df(
    df: &DataFrame,
    obj_idx: usize,
) -> Option<ArdImportanceResult> {
    let param_names = df.param_col_names().to_vec();
    let n = df.row_count();
    let n_params = param_names.len();
    if n_params == 0 {
        return None;
    }
    let objective_name = df.objective_col_names().get(obj_idx)?.clone();

    // Parameter columns (numeric or label-encoded) -> row-major X matrix.
    let param_columns: Vec<Vec<f64>> = param_names
        .iter()
        .map(|name| get_param_numeric_values(df, name, n).unwrap_or_else(|| vec![0.0; n]))
        .collect();
    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| param_columns.iter().map(|col| col[i]).collect())
        .collect();
    let y: Vec<f64> = df
        .get_numeric_column(&objective_name)?
        .iter()
        .take(n)
        .copied()
        .collect();

    let req = SurrogateFitRequest {
        x_matrix,
        y,
        param_names,
        objective_name,
        model: SurrogateModelKind::GpFitc,
        auto_select: false,
        constraints: vec![],
        priority_rows: vec![],
        param_bounds: None,
    };
    // Input validation (minimum trial count, etc.) is handled by fit_surrogate_with_validation.
    let trained = fit_surrogate_with_validation(&req).ok()?;
    let importances = trained.param_importance?;
    Some(ArdImportanceResult {
        param_names: trained.param_names,
        importances,
        r_squared: trained.validation.cv_r2_mean,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataframe::{DataFrame, TrialRow};
    use std::collections::HashMap;

    fn make_row(trial_id: u32, params: &[(&str, f64)], objectives: Vec<f64>) -> TrialRow {
        TrialRow {
            trial_id,
            trial_number: trial_id,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values: objectives,
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    /// Wiring check: fitting GP-FITC yields ARD importance that is consistent with
    /// param_names and sums to 1.0. Does not verify the numerical quality of the
    /// egobox GP itself.
    #[test]
    fn ard_importance_from_df_wires_through_gp() {
        // Constructed so x0 strongly drives the response and x1 is nearly irrelevant.
        let rows: Vec<TrialRow> = (0..30)
            .map(|i| {
                let x0 = i as f64 / 30.0;
                let x1 = ((i * 7) % 30) as f64 / 30.0;
                make_row(i, &[("x0", x0), ("x1", x1)], vec![3.0 * x0 + 0.01 * x1])
            })
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &["x0".to_string(), "x1".to_string()],
            &["obj".to_string()],
            &[],
            &[],
            0,
        );

        let result =
            compute_ard_importance_from_df(&df, 0).expect("GP-FITC should expose ARD importance");
        assert_eq!(result.param_names, vec!["x0".to_string(), "x1".to_string()]);
        assert_eq!(result.importances.len(), 2);
        let sum: f64 = result.importances.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "importances should sum to 1.0, got {sum}"
        );
        // x0 drives the response ~300x more strongly, so if column order is preserved
        // x0's importance should be larger. (Summing to 1.0 and matching length alone
        // cannot detect a param<->importance swap.)
        assert!(
            result.importances[0] > result.importances[1],
            "x0 drives the response far more than x1; ARD importance must rank it higher: {:?}",
            result.importances
        );
        assert!(result.r_squared.is_finite());
    }

    /// An out-of-range objective index returns None.
    #[test]
    fn ard_importance_from_df_out_of_range_objective() {
        let rows: Vec<TrialRow> = (0..12)
            .map(|i| make_row(i, &[("x0", i as f64)], vec![i as f64]))
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &["x0".to_string()],
            &["obj".to_string()],
            &[],
            &[],
            0,
        );
        assert!(compute_ard_importance_from_df(&df, 5).is_none());
    }
}
