use super::require_study;
use crate::state::app_state::AppState;
use crate::ui::widget_states::WidgetStates;
use tunny_core::export::{CsvField, CsvWriter};

pub(super) fn build_importance_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    if widgets.importance.computing {
        return None;
    }
    use crate::ui::widgets::importance_chart::{compute_sorted_importance, compute_sorted_sobol};
    let metric = &widgets.importance.metric;
    let obj_idx = widgets.importance.objective_index;
    let feasible_only = widgets.importance.feasible_only;
    let method_name = metric.label();
    let pairs: Vec<(String, f64)> = if metric.is_sobol() {
        let sobol = app_state.sobol_cache.get(&(obj_idx, feasible_only))?;
        compute_sorted_sobol(sobol, obj_idx, metric)
    } else {
        let key = (metric.cache_id(), obj_idx, feasible_only);
        let sensitivity = app_state.importance_cache.get(&key)?;
        compute_sorted_importance(sensitivity, metric, obj_idx)
    };
    if pairs.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["variable", "importance_score", "method"]);
    for (name, score) in &pairs {
        w.row([
            CsvField::Text(name),
            CsvField::Num(*score),
            CsvField::Text(method_name),
        ]);
    }
    Some(w.finish())
}

pub(super) fn build_sensitivity_csv(
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let m = app_state.sensitivity_heatmap_cache.get(&(
        widgets.sensitivity_heatmap.metric.cache_id(),
        widgets.sensitivity_heatmap.feasible_only,
    ))?;
    if !m.is_well_formed() {
        return None;
    }
    let mut w = CsvWriter::new();
    let mut header: Vec<&str> = vec!["variable"];
    header.extend(m.objective_names.iter().map(String::as_str));
    w.header(header);
    for (i, param_name) in m.param_names.iter().enumerate() {
        let mut fields = vec![CsvField::Text(param_name)];
        for &val in &m.values[i] {
            fields.push(CsvField::Num(val));
        }
        w.row(fields);
    }
    Some(w.finish())
}

pub(super) fn build_pdp_csv(_app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let r = widgets.pdp_chart.result.as_ref()?;
    if r.x_values.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([
        "variable",
        "variable_value",
        "predicted_objective",
        "lower_ci",
        "upper_ci",
    ]);
    for (i, (&x, &y)) in r.x_values.iter().zip(r.y_values.iter()).enumerate() {
        let lower = r.y_lower.as_ref().and_then(|v| v.get(i)).copied();
        let upper = r.y_upper.as_ref().and_then(|v| v.get(i)).copied();
        w.row([
            CsvField::Text(&r.param_name),
            CsvField::Num(x),
            CsvField::Num(y),
            lower.map(CsvField::Num).unwrap_or(CsvField::Empty),
            upper.map(CsvField::Num).unwrap_or(CsvField::Empty),
        ]);
    }
    Some(w.finish())
}

pub(super) fn build_pdp_2d_csv(_app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let result = widgets.pdp_2d.result.as_ref()?;
    if result.x_values.is_empty() || result.y_values.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([
        "param1_name",
        "param1_value",
        "param2_name",
        "param2_value",
        "predicted_objective",
    ]);
    for (xi, &x) in result.x_values.iter().enumerate() {
        for (yi, &y) in result.y_values.iter().enumerate() {
            let z = result
                .z_values
                .get(xi)
                .and_then(|row| row.get(yi))
                .copied()
                .unwrap_or(f64::NAN);
            w.row([
                CsvField::Text(&result.param1_name),
                CsvField::Num(x),
                CsvField::Text(&result.param2_name),
                CsvField::Num(y),
                CsvField::Num(z),
            ]);
        }
    }
    Some(w.finish())
}

pub(super) fn build_slice_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    let param_idx = widgets.slice_chart.selected_param_idx;
    let obj_idx = widgets.slice_chart.selected_obj_idx;
    let param_name = study.meta.param_names.get(param_idx)?;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let param_col = study.view.numeric_column(param_name);
    let obj_col = study.view.numeric_column(obj_name);
    // Pareto membership is the per-row rank == 0 in the view (row-aligned).
    let mut w = CsvWriter::new();
    w.header(["trial_id", param_name, obj_name, "is_pareto"]);
    for (i, &tid) in study.view.trial_ids.iter().enumerate() {
        let param_val = param_col
            .and_then(|c| c.get(i))
            .copied()
            .unwrap_or(f64::NAN);
        let obj_val = obj_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
        let is_pareto = study.view.pareto_rank.get(i).copied() == Some(0);
        w.row([
            CsvField::UInt(tid as u64),
            CsvField::Num(param_val),
            CsvField::Num(obj_val),
            CsvField::Text(if is_pareto { "true" } else { "false" }),
        ]);
    }
    Some(w.finish())
}

/// Turns the response surface slice's z grid into CSV (x values in the column headers,
/// y value at the start of each row).
pub(super) fn build_response_surface_csv(widgets: &WidgetStates) -> Option<String> {
    let slice = widgets.response_surface.cached_slice()?;
    let nx = slice.x_values.len();
    let ny = slice.y_values.len();
    if nx == 0 || ny == 0 {
        return None;
    }
    let mut w = CsvWriter::new();
    let mut header: Vec<String> = vec!["y\\x".to_string()];
    header.extend(slice.x_values.iter().map(|x| x.to_string()));
    w.header(header.iter().map(String::as_str));
    for yi in 0..ny {
        let mut fields = vec![CsvField::Num(slice.y_values[yi])];
        for xi in 0..nx {
            let z = slice
                .z_values
                .get(xi)
                .and_then(|row| row.get(yi))
                .copied()
                .unwrap_or(f64::NAN);
            fields.push(if z.is_finite() {
                CsvField::Num(z)
            } else {
                CsvField::Empty
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// Outputs the Observed Contour interpolation grid in long format (masked cells
/// excluded).
pub(super) fn build_observed_contour_csv(widgets: &WidgetStates) -> Option<String> {
    let r = widgets.observed_contour.result.as_ref()?;
    let surf = &r.surface;
    if surf.x_values.is_empty() || surf.y_values.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([r.x_name.as_str(), r.y_name.as_str(), r.value_name.as_str()]);
    for (i, &x) in surf.x_values.iter().enumerate() {
        for (j, &y) in surf.y_values.iter().enumerate() {
            if let Some(Some(v)) = surf.z.get(i).map(|col| col[j]) {
                w.row([CsvField::Num(x), CsvField::Num(y), CsvField::Num(v)]);
            }
        }
    }
    Some(w.finish())
}
