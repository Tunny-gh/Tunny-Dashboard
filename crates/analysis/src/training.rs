//! Method-specific derived training selections. Missingness and categorical
//! policies intentionally differ between numeric fitting and the 1D PDP path.
use crate::dataframe::DataFrame;
use crate::surrogate_opt::ConstraintData;
use std::collections::HashMap;

pub type NumericFitXy = (
    Vec<String>,
    Vec<Vec<f64>>,
    Vec<f64>,
    Vec<Option<(f64, f64)>>,
    Vec<usize>,
);
pub type NumericFitXyMulti = (
    Vec<String>,
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<Option<(f64, f64)>>,
    Vec<usize>,
);

pub fn numeric_param_names(df: &DataFrame, params: &[String]) -> Vec<String> {
    params
        .iter()
        .filter(|p| df.get_numeric_column(p).is_some())
        .cloned()
        .collect()
}

pub fn numeric_fit_xy_multi(
    df: &DataFrame,
    params: &[String],
    objectives: &[String],
    bounds: &HashMap<String, (f64, f64)>,
) -> Option<NumericFitXyMulti> {
    let names = numeric_param_names(df, params);
    if names.is_empty() {
        return None;
    }
    let columns: Vec<_> = names.iter().map(|p| df.get_numeric_column(p)).collect();
    let objectives: Vec<_> = objectives
        .iter()
        .map(|o| df.get_numeric_column(o))
        .collect();
    let mut x = Vec::new();
    let mut ys = vec![Vec::new(); objectives.len()];
    let mut rows = Vec::new();
    for row in 0..df.row_count() {
        let values: Vec<_> = columns
            .iter()
            .map(|c| c.and_then(|c| c.get(row)).copied().unwrap_or(0.0))
            .collect();
        let targets: Vec<_> = objectives
            .iter()
            .map(|c| c.and_then(|c| c.get(row)).copied().unwrap_or(0.0))
            .collect();
        if values.iter().all(|v| v.is_finite()) && targets.iter().all(|v| v.is_finite()) {
            x.push(values);
            for (column, value) in ys.iter_mut().zip(targets) {
                column.push(value);
            }
            rows.push(row);
        }
    }
    let bounds = names.iter().map(|p| bounds.get(p).copied()).collect();
    Some((names, x, ys, bounds, rows))
}

pub fn numeric_fit_xy(
    df: &DataFrame,
    params: &[String],
    objective: &str,
    bounds: &HashMap<String, (f64, f64)>,
) -> Option<NumericFitXy> {
    let (names, x, mut ys, bounds, rows) =
        numeric_fit_xy_multi(df, params, &[objective.to_string()], bounds)?;
    Some((names, x, ys.remove(0), bounds, rows))
}

pub fn collect_constraints(df: &DataFrame, kept_rows: &[usize]) -> Vec<ConstraintData> {
    df.constraint_col_names()
        .iter()
        .filter_map(|name| {
            let column = df.get_numeric_column(name)?;
            Some(ConstraintData {
                name: name.clone(),
                values: kept_rows
                    .iter()
                    .map(|&row| column.get(row).copied().unwrap_or(0.0))
                    .collect(),
            })
        })
        .collect()
}

pub fn pdp_1d_xy(
    df: &DataFrame,
    params: &[String],
    objective: &str,
    feasible_only: bool,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let (x, y, _) = pdp_rows(df, params, objective, feasible_only, false);
    (x, y)
}

pub fn pdp_rows(
    df: &DataFrame,
    params: &[String],
    objective: &str,
    feasible_only: bool,
    strict_missing: bool,
) -> (Vec<Vec<f64>>, Vec<f64>, Vec<usize>) {
    let columns: Vec<_> = params.iter().map(|p| df.get_numeric_column(p)).collect();
    let objective = df.get_numeric_column(objective);
    let feasibility = df.feasibility();
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut rows = Vec::new();
    for row in 0..df.row_count() {
        if feasible_only && !feasibility.is_feasible(row) {
            continue;
        }
        let values: Vec<_> = columns
            .iter()
            .zip(params)
            .map(|(c, name)| {
                c.and_then(|c| c.get(row)).copied().unwrap_or_else(|| {
                    if !strict_missing || (c.is_none() && df.get_string_column(name).is_some()) {
                        0.0
                    } else {
                        f64::NAN
                    }
                })
            })
            .collect();
        let target = objective
            .and_then(|c| c.get(row))
            .copied()
            .unwrap_or(if strict_missing { f64::NAN } else { 0.0 });
        if values.iter().all(|v| v.is_finite()) && target.is_finite() {
            x.push(values);
            y.push(target);
            rows.push(row);
        }
    }
    (x, y, rows)
}
