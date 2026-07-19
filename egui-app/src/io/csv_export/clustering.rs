use super::{cluster_result_for_chart, require_study};
use crate::state::app_state::AppState;
use crate::state::layout_state::ChartId;
use crate::state::results::ClusterResult;
use crate::ui::widget_states::WidgetStates;
use tunny_core::export::{CsvField, CsvWriter};

pub(super) fn build_cluster_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let cr = cluster_result_for_chart(chart_id, app_state, widgets)?;
    build_cluster_csv_from_result(cr, app_state)
}

/// Builds CSV by taking a cluster result directly (chart-ID-independent). Used by
/// callers that don't have a ChartId, such as the unified trial table.
pub(super) fn build_cluster_csv_from_result(
    cr: &ClusterResult,
    app_state: &AppState,
) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let n = study.trial_count();
    if cr.labels.len() != n {
        return None;
    }
    let param_names = &study.meta.param_names;
    let obj_names = &study.meta.objective_names;
    let param_cols = study.view.numeric_columns(param_names);
    let obj_cols = study.view.numeric_columns(obj_names);
    let mut w = CsvWriter::new();
    let mut header: Vec<&str> = vec!["trial_id", "trial_number"];
    header.extend(param_names.iter().map(String::as_str));
    header.extend(obj_names.iter().map(String::as_str));
    header.push("cluster_id");
    w.header(header);
    for i in 0..n {
        let trial_id = study.view.trial_ids.get(i).copied().unwrap_or(i as u32);
        let trial_number = study.view.df.get_trial_number(i).unwrap_or(i as u32);
        let mut fields = vec![
            CsvField::UInt(trial_id as u64),
            CsvField::UInt(trial_number as u64),
        ];
        for col in param_cols.iter().chain(&obj_cols) {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            fields.push(CsvField::Num(v));
        }
        let label = cr.labels.get(i).copied().unwrap_or(-1);
        fields.push(CsvField::Int(label as i64));
        w.row(fields);
    }
    Some(w.finish())
}

/// Turns the cached PCA result into a two-column `pc1,pc2` CSV.
pub(super) fn build_pca_biplot_csv(widgets: &WidgetStates) -> Option<String> {
    let result = widgets.pca_biplot.cached_result()?;
    if result.projections.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["pc1", "pc2"]);
    for row in &result.projections {
        let pc1 = row.first().copied().unwrap_or(0.0);
        let pc2 = row.get(1).copied().unwrap_or(0.0);
        w.row([CsvField::Num(pc1), CsvField::Num(pc2)]);
    }
    Some(w.finish())
}

/// Turns the node value grid corresponding to SOM's current display mode (U-matrix /
/// Component Plane / Hits) into CSV in wide format (rows = y, columns = x).
pub(super) fn build_som_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let (grid_w, grid_h, values, _label) = widgets
        .som_map
        .current_grid(&study.meta.param_names, &study.meta.objective_names)?;
    if grid_w == 0 || grid_h == 0 || values.len() != grid_w * grid_h {
        return None;
    }
    let mut w = CsvWriter::new();
    let mut header: Vec<String> = vec!["y".to_string()];
    header.extend((0..grid_w).map(|x| format!("x{x}")));
    w.header(header.iter().map(String::as_str));
    for y in 0..grid_h {
        let mut fields = vec![CsvField::UInt(y as u64)];
        for x in 0..grid_w {
            let v = values[y * grid_w + x];
            fields.push(if v.is_finite() {
                CsvField::Num(v)
            } else {
                CsvField::Empty
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// Turns (original view row index, post-cut cluster label) pairs into CSV, in
/// dendrogram leaf order.
pub(super) fn build_dendrogram_csv(widgets: &WidgetStates) -> Option<String> {
    let assignments = widgets.dendrogram.leaf_assignments()?;
    if assignments.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["row_index", "cluster"]);
    for (row_index, cluster) in assignments {
        w.row([
            CsvField::UInt(row_index as u64),
            CsvField::UInt(cluster as u64),
        ]);
    }
    Some(w.finish())
}

/// Turns the pinned trials' raw values into CSV in wide format (one row per axis,
/// columns = pinned trials) using radar comparison's current axis settings (Include
/// parameters). Outputs raw values before normalization.
pub(super) fn build_radar_comparison_csv(
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let study = require_study(app_state)?;
    if app_state.pinned_trials.is_empty() {
        return None;
    }
    let axes = crate::ui::widgets::radar_comparison::build_axes(
        &study.view,
        &study.meta.param_names,
        &study.meta.objective_names,
        widgets.radar_comparison.include_params,
    );
    if axes.is_empty() {
        return None;
    }
    let pinned_rows: Vec<(u32, usize)> = app_state
        .pinned_trials
        .iter()
        .filter_map(|&trial_id| {
            study
                .view
                .trial_ids
                .iter()
                .position(|&t| t == trial_id)
                .map(|row| (trial_id, row))
        })
        .collect();
    if pinned_rows.is_empty() {
        return None;
    }

    let column_labels: Vec<String> = pinned_rows
        .iter()
        .map(|&(trial_id, row)| {
            let number = study.view.df.get_trial_number(row).unwrap_or(trial_id);
            format!("Trial #{number}")
        })
        .collect();
    let mut header: Vec<&str> = vec!["axis"];
    header.extend(column_labels.iter().map(String::as_str));

    let mut w = CsvWriter::new();
    w.header(header);
    for axis in &axes {
        let mut fields = vec![CsvField::Text(axis.name)];
        for &(_, row) in &pinned_rows {
            fields.push(match axis.col.get(row) {
                Some(&v) if v.is_finite() => CsvField::Num(v),
                _ => CsvField::Empty,
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// Turns the pinned trials' raw values into CSV in wide format (one row per row-def,
/// columns = pinned trials) using the comparison table's current row settings
/// (Parameters / User attrs).
pub(super) fn build_comparison_table_csv(
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let study = require_study(app_state)?;
    if app_state.pinned_trials.is_empty() {
        return None;
    }
    let pinned_rows = crate::ui::widgets::comparison_table::resolve_pinned_rows(
        &study.view,
        &app_state.pinned_trials,
    );
    if pinned_rows.is_empty() {
        return None;
    }
    let rows = crate::ui::widgets::comparison_table::build_rows(
        &study.view,
        &study.meta.param_names,
        &study.meta.objective_names,
        widgets.comparison_table.show_params,
        widgets.comparison_table.show_user_attrs,
    );
    if rows.is_empty() {
        return None;
    }

    let column_labels: Vec<String> = pinned_rows
        .iter()
        .map(|&(trial_id, row)| {
            let number = study.view.df.get_trial_number(row).unwrap_or(trial_id);
            format!("Trial #{number}")
        })
        .collect();
    let mut header: Vec<&str> = vec![""];
    header.extend(column_labels.iter().map(String::as_str));

    let mut w = CsvWriter::new();
    w.header(header);
    for info in &rows {
        let mut fields = vec![CsvField::Text(info.label)];
        for &(_, row) in &pinned_rows {
            fields.push(match info.col.get(row) {
                Some(&v) if v.is_finite() => CsvField::Num(v),
                _ => CsvField::Empty,
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}
