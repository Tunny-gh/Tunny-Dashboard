use crate::state::app_state::{Direction, StudyContext};
use crate::state::messages::AppMessage;

/// Resolves the per-objective minimize flag from `directions` (shared by multi-objective optimization paths).
/// Returns `n_obj` entries; objectives missing from `directions` fall back to Minimize(true).
pub(super) fn minimize_flags(directions: &[Direction], n_obj: usize) -> Vec<bool> {
    (0..n_obj)
        .map(|i| {
            directions
                .get(i)
                .map(|d| matches!(d, Direction::Minimize))
                .unwrap_or(true)
        })
        .collect()
}

/// List of numeric parameter names, excluding categorical columns (those that can't be numeric-ized).
/// Provides the common filter used by render_chart's combo display and fit-matrix construction.
pub(crate) fn numeric_param_names(ctx: &StudyContext) -> Vec<String> {
    ctx.meta
        .param_names
        .iter()
        .filter(|p| ctx.view.numeric_column(p).is_some())
        .cloned()
        .collect()
}

/// Extracts constraint columns as `ConstraintData`. Keeps only the rows specified by `kept_rows`,
/// aligning them with the non-finite-filtered fit matrix (X from `build_numeric_fit_xy`).
pub(super) fn collect_constraints(
    ctx: &StudyContext,
    kept_rows: &[usize],
) -> Vec<tunny_core::surrogate_opt::ConstraintData> {
    ctx.view
        .df
        .constraint_col_names()
        .iter()
        .filter_map(|col_name| {
            ctx.view.df.get_numeric_column(col_name).map(|col| {
                tunny_core::surrogate_opt::ConstraintData {
                    name: col_name.clone(),
                    values: kept_rows
                        .iter()
                        .map(|&i| col.get(i).copied().unwrap_or(0.0))
                        .collect(),
                }
            })
        })
        .collect()
}

/// Return value of `build_numeric_fit_xy`.
/// (numeric parameter names, X matrix, y, per-parameter declared range, kept row indices).
pub(super) type NumericFitXy = (
    Vec<String>,
    Vec<Vec<f64>>,
    Vec<f64>,
    Vec<Option<(f64, f64)>>,
    Vec<usize>,
);

/// Builds the X matrix, objective vector y, and declared range (param_bounds) using only numeric
/// parameter columns. Rows containing non-finite values (NaN/inf) are excluded from training (this
/// prevents NaN from pruned/failed trials flowing into the GP/regression training matrix and causing
/// all-NaN predictions or worker panics; same `is_finite` filter policy as observed_contour). Returns
/// None if there are no numeric parameters at all.
/// `kept_rows` holds the original df indices of the kept rows, used to align constraint columns
/// (`collect_constraints`) and anchor row resolution with X.
pub(super) fn build_numeric_fit_xy(ctx: &StudyContext, objective: &str) -> Option<NumericFitXy> {
    let numeric_params = numeric_param_names(ctx);
    if numeric_params.is_empty() {
        return None;
    }

    let n = ctx.view.row_count();
    let param_cols = ctx.view.numeric_columns(&numeric_params);
    // Fill with 0.0 when the objective column is missing (existing behavior). Missing cells in existing rows are also 0.0.
    let obj_col = ctx.view.numeric_column(objective);

    let mut x_matrix: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut y: Vec<f64> = Vec::with_capacity(n);
    let mut kept_rows: Vec<usize> = Vec::with_capacity(n);
    for i in 0..n {
        let row: Vec<f64> = param_cols
            .iter()
            .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
            .collect();
        let yv = obj_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
        // Exclude rows containing non-finite values (NaN/inf).
        if row.iter().all(|v| v.is_finite()) && yv.is_finite() {
            x_matrix.push(row);
            y.push(yv);
            kept_rows.push(i);
        }
    }

    // Collect each numeric parameter's declared range (derived from log) in x_matrix column order.
    // Columns with a declared range use it as the search range; columns without one fall back to the observed range.
    let param_bounds: Vec<Option<(f64, f64)>> = numeric_params
        .iter()
        .map(|p| ctx.meta.param_bounds.get(p).copied())
        .collect();

    Some((numeric_params, x_matrix, y, param_bounds, kept_rows))
}

/// Multi-objective version of `build_numeric_fit_xy`. Extracts the y columns for all objectives together,
/// excluding rows where any objective or X value is non-finite.
pub(super) type NumericFitXyMulti = (
    Vec<String>,
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<Option<(f64, f64)>>,
    Vec<usize>,
);

pub(super) fn build_numeric_fit_xy_multi(
    ctx: &StudyContext,
    objectives: &[String],
) -> Option<NumericFitXyMulti> {
    let numeric_params = numeric_param_names(ctx);
    if numeric_params.is_empty() {
        return None;
    }

    let n = ctx.view.row_count();
    let param_cols = ctx.view.numeric_columns(&numeric_params);
    let obj_cols = ctx.view.numeric_columns(objectives);

    let mut x_matrix: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut kept_rows: Vec<usize> = Vec::with_capacity(n);
    let mut objective_values: Vec<Vec<f64>> = vec![Vec::with_capacity(n); objectives.len()];
    for i in 0..n {
        let row: Vec<f64> = param_cols
            .iter()
            .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
            .collect();
        let ys: Vec<f64> = obj_cols
            .iter()
            .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
            .collect();
        if row.iter().all(|v| v.is_finite()) && ys.iter().all(|v| v.is_finite()) {
            x_matrix.push(row);
            for (o, &v) in ys.iter().enumerate() {
                objective_values[o].push(v);
            }
            kept_rows.push(i);
        }
    }

    let param_bounds: Vec<Option<(f64, f64)>> = numeric_params
        .iter()
        .map(|p| ctx.meta.param_bounds.get(p).copied())
        .collect();

    Some((
        numeric_params,
        x_matrix,
        objective_values,
        param_bounds,
        kept_rows,
    ))
}

/// Builds (X, y) for PDP. When feasible_only is set, only feasible solutions are targeted,
/// and rows containing non-finite values (NaN/inf) are excluded (same filter policy as observed_contour).
pub(super) fn build_xy_for_objective(
    ctx: &StudyContext,
    objective: &str,
    feasible_only: bool,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let param_names = &ctx.meta.param_names;
    let n = ctx.view.row_count();

    let param_cols = ctx.view.numeric_columns(param_names);
    let obj_col = ctx.view.numeric_column(objective);
    // Feasible-solution filter. If there's no is_feasible column (no constraints), all rows are targeted.
    let feas = ctx.view.feasibility();

    let mut x_matrix: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut y: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        if feasible_only && !feas.is_feasible(i) {
            continue;
        }
        let row: Vec<f64> = param_cols
            .iter()
            .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
            .collect();
        let yv = obj_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
        if !row.iter().all(|v| v.is_finite()) || !yv.is_finite() {
            continue;
        }
        x_matrix.push(row);
        y.push(yv);
    }

    (x_matrix, y)
}

/// Returns the DataFrame for sensitivity analysis. When feasible_only is set, makes a copy
/// containing only feasible solutions (since the core functions take a DataFrame directly).
pub(super) fn sensitivity_df(
    ctx: &crate::state::app_state::StudyContext,
    feasible_only: bool,
) -> std::sync::Arc<tunny_core::dataframe::DataFrame> {
    if feasible_only {
        std::sync::Arc::new(ctx.view.df.filter_feasible())
    } else {
        std::sync::Arc::clone(&ctx.view.df)
    }
}

/// Computes the all-parameter x all-objective sensitivity matrix `values[param][obj]` for the selected
/// method. For Sobol (First/Total), indices are extracted from a single all-objective computation;
/// for other methods, each column is filled by evaluating the single-objective metric per objective.
/// The method-to-core-metric mapping is shared with ImportanceChart via `core_sensitivity_metric`.
pub(super) fn compute_sensitivity_heatmap(
    metric: crate::ui::widgets::importance_chart::ImportanceMetric,
    feasible_only: bool,
    df: &tunny_core::dataframe::DataFrame,
) -> AppMessage {
    use crate::state::results::HeatmapMatrix;
    use crate::ui::widgets::importance_chart::{core_sensitivity_metric, SOBOL_SAMPLE_COUNT};

    let param_names = df.param_col_names().to_vec();
    let objective_names = df.objective_col_names().to_vec();
    let n_params = param_names.len();
    let n_objs = objective_names.len();
    let signed = metric.is_signed();

    let mut values = vec![vec![0.0f64; n_objs]; n_params];

    if metric.is_sobol() {
        // Both first_order and total_effect return all objectives at once, shaped as [param][obj].
        if let Some(sobol) = tunny_core::sensitivity::compute_sobol_from_df(df, SOBOL_SAMPLE_COUNT)
        {
            use crate::ui::widgets::importance_chart::ImportanceMetric;
            let data = if metric == ImportanceMetric::SobolFirst {
                &sobol.first_order
            } else {
                &sobol.total_effect
            };
            for (param_idx, row) in data.iter().enumerate() {
                if let Some(dst) = values.get_mut(param_idx) {
                    for (obj_idx, &v) in row.iter().take(n_objs).enumerate() {
                        dst[obj_idx] = v;
                    }
                }
            }
        }
    } else if let Some(core) = core_sensitivity_metric(metric) {
        for obj_idx in 0..n_objs {
            let Some(r) = core.compute(df, obj_idx) else {
                continue;
            };
            for (param_idx, dst) in values.iter_mut().enumerate() {
                dst[obj_idx] = single_obj_param_score(&r, metric, param_idx);
            }
        }
    }

    AppMessage::SensitivityHeatmapDone {
        metric,
        feasible_only,
        result: HeatmapMatrix {
            param_names,
            objective_names,
            values,
            signed,
        },
    }
}

/// Extracts the score for the given parameter from a single-objective computation result (core
/// `SensitivityResult`). Tree-based methods (RF-Anova/MDI/SHAP/Permutation) use `importances[param][0]`,
/// Spearman uses `spearman[param][0]`, Ridge uses `ridge[0].beta[param]`. Sobol doesn't go through this path.
fn single_obj_param_score(
    r: &tunny_core::sensitivity::SensitivityResult,
    metric: crate::ui::widgets::importance_chart::ImportanceMetric,
    param_idx: usize,
) -> f64 {
    use crate::ui::widgets::importance_chart::ImportanceMetric;
    let tree = match metric {
        ImportanceMetric::RfAnova => r.rf_anova.as_ref().map(|x| &x.0),
        ImportanceMetric::Mdi => r.mdi.as_ref().map(|x| &x.0),
        ImportanceMetric::Shap => r.shap.as_ref().map(|x| &x.0),
        ImportanceMetric::Permutation => r.permutation.as_ref().map(|x| &x.0),
        _ => None,
    };
    match metric {
        ImportanceMetric::Spearman => r
            .spearman
            .get(param_idx)
            .and_then(|row| row.first())
            .copied()
            .unwrap_or(0.0),
        ImportanceMetric::Ridge => r
            .ridge
            .first()
            .and_then(|rg| rg.beta.get(param_idx))
            .copied()
            .unwrap_or(0.0),
        ImportanceMetric::RfAnova
        | ImportanceMetric::Mdi
        | ImportanceMetric::Shap
        | ImportanceMetric::Permutation => tree
            .and_then(|t| t.importances.get(param_idx))
            .and_then(|row| row.first())
            .copied()
            .unwrap_or(0.0),
        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal | ImportanceMetric::Ard => 0.0,
    }
}
