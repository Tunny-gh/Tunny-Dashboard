use super::require_study;
use crate::state::app_state::AppState;
use crate::state::layout_state::ChartId;
use crate::state::types::Direction;
use crate::ui::widget_states::WidgetStates;
use tunny_core::export::{CsvField, CsvWriter};

/// Recomputes the histogram with the current column selection / bin settings and turns
/// it into CSV. Applies the same fallback as when the widget renders (objective ->
/// parameter's first numeric column).
pub(super) fn build_histogram_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    let obj_names = &study.meta.objective_names;
    let param_names = &study.meta.param_names;
    let candidates: Vec<&String> = obj_names
        .iter()
        .chain(param_names.iter())
        .filter(|n| study.view.numeric_column(n).is_some())
        .collect();
    let selected = widgets.histogram.selected_col.as_str();
    let col = if candidates.iter().any(|c| c.as_str() == selected) {
        selected
    } else {
        candidates.first()?.as_str()
    };
    let values = study.view.numeric_column(col)?;
    let rule = widgets
        .histogram
        .rule
        .to_core(widgets.histogram.manual_bins);
    let hist = tunny_core::statistics::compute_histogram(values, rule)?;

    let mut w = CsvWriter::new();
    w.header(["bin_start", "bin_end", "count"]);
    for (edge, &count) in hist.bin_edges.windows(2).zip(&hist.counts) {
        w.row([
            CsvField::Num(edge[0]),
            CsvField::Num(edge[1]),
            CsvField::UInt(count as u64),
        ]);
    }
    Some(w.finish())
}

/// Uses the same fallback, groups, scale, and statistics as the renderer.
pub(super) fn build_box_plot_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    let prepared = widgets.box_plot.selection.prepare(&study.view);
    let value = prepared.selection.value.as_ref()?.label();
    let group_by = prepared
        .selection
        .group_by
        .as_ref()
        .map(|n| format!("Parameter: {n}"))
        .unwrap_or_else(|| "None".into());
    let scale = if prepared.selection.normalize {
        "normalized [0,1]"
    } else {
        "raw"
    };
    let stats = prepared.boxes();
    let mut w = CsvWriter::new();
    w.header([
        "value",
        "group_by",
        "scale",
        "group",
        "group_identity",
        "n",
        "mean",
        "min",
        "q1",
        "median",
        "q3",
        "max",
        "whisker_low",
        "whisker_high",
        "n_outliers",
    ]);
    for (name, s) in &stats {
        w.row([
            CsvField::Text(&value),
            CsvField::Text(&group_by),
            CsvField::Text(scale),
            CsvField::Text(name),
            CsvField::Text(prepared.group_identity(name)),
            CsvField::UInt(s.n as u64),
            CsvField::Num(s.mean),
            CsvField::Num(s.min),
            CsvField::Num(s.q1),
            CsvField::Num(s.median),
            CsvField::Num(s.q3),
            CsvField::Num(s.max),
            CsvField::Num(s.whisker_low),
            CsvField::Num(s.whisker_high),
            CsvField::UInt(s.outliers.len() as u64),
        ]);
    }
    (!stats.is_empty()).then(|| w.finish())
}

/// One row per displayed group/grid point; no export when all groups are skipped.
pub(super) fn build_violin_plot_csv(
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let study = require_study(app_state)?;
    let prepared = widgets.violin_plot.selection.prepare(&study.view);
    let value_name = prepared.selection.value.as_ref()?.label();
    let group_by = prepared
        .selection
        .group_by
        .as_ref()
        .map(|n| format!("Parameter: {n}"))
        .unwrap_or_else(|| "None".into());
    let scale = if prepared.selection.normalize {
        "normalized [0,1]"
    } else {
        "raw"
    };
    let (curves, _) = prepared.violins();
    let mut w = CsvWriter::new();
    w.header([
        "value",
        "group_by",
        "scale",
        "group",
        "group_identity",
        "grid",
        "density",
    ]);
    for (label, curve) in &curves {
        for (&value, &density) in curve.grid.iter().zip(curve.density.iter()) {
            w.row([
                CsvField::Text(&value_name),
                CsvField::Text(&group_by),
                CsvField::Text(scale),
                CsvField::Text(label),
                CsvField::Text(prepared.group_identity(label)),
                CsvField::Num(value),
                CsvField::Num(density),
            ]);
        }
    }
    (!curves.is_empty()).then(|| w.finish())
}

/// Recomputes the correlation matrix with the current Method/column-group settings and
/// turns it into CSV in wide format. NaN cells are output as an empty string.
pub(super) fn build_correlation_matrix_csv(
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let study = require_study(app_state)?;
    if !widgets.correlation_matrix.include_params && !widgets.correlation_matrix.include_objectives
    {
        return None;
    }
    let mut names: Vec<&String> = Vec::new();
    if widgets.correlation_matrix.include_params {
        names.extend(study.meta.param_names.iter());
    }
    if widgets.correlation_matrix.include_objectives {
        names.extend(study.meta.objective_names.iter());
    }
    let columns: Vec<(String, Vec<f64>)> = names
        .into_iter()
        .filter_map(|name| {
            study
                .view
                .numeric_column(name)
                .map(|c| (name.clone(), c.to_vec()))
        })
        .collect();
    if columns.is_empty() {
        return None;
    }
    let matrix = tunny_core::statistics::compute_correlation_matrix(
        &columns,
        widgets.correlation_matrix.method,
    )?;

    let mut w = CsvWriter::new();
    let mut header: Vec<&str> = vec![""];
    header.extend(matrix.labels.iter().map(String::as_str));
    w.header(header);
    for (i, label) in matrix.labels.iter().enumerate() {
        let mut fields = vec![CsvField::Text(label)];
        for &val in &matrix.values[i] {
            fields.push(if val.is_nan() {
                CsvField::Empty
            } else {
                CsvField::Num(val)
            });
        }
        w.row(fields);
    }
    Some(w.finish())
}

/// Outputs the EDF (empirical distribution function) point list for all trials (doesn't
/// apply the display-only log filter, no thinning).
pub(super) fn build_edf_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let obj_idx = widgets.edf_plot.obj_idx;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let values: Vec<f64> = study.view.numeric_column(obj_name)?.to_vec();
    let points = crate::ui::widgets::edf_plot::build_edf_points(&values, false);
    if points.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([obj_name.as_str(), "cumulative_fraction"]);
    for &[x, y] in &points {
        w.row([CsvField::Num(x), CsvField::Num(y)]);
    }
    Some(w.finish())
}

/// Outputs all trials of Rank Plot (including NaN/missing values).
pub(super) fn build_rank_plot_csv(
    chart_id: &ChartId,
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let study = require_study(app_state)?;
    let (axes, obj_idx) = if matches!(chart_id, ChartId::RankPlot3D) {
        let w = &widgets.rank_plot_3d;
        (vec![w.x_param_idx, w.y_param_idx, w.z_param_idx], w.obj_idx)
    } else {
        let w = &widgets.rank_plot_2d;
        (vec![w.x_param_idx, w.y_param_idx], w.obj_idx)
    };
    let names: Vec<_> = axes
        .iter()
        .map(|&i| study.meta.param_names.get(i))
        .collect::<Option<_>>()?;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let minimize = !matches!(
        study.meta.directions.get(obj_idx),
        Some(Direction::Maximize)
    );
    let obj_values: Vec<f64> = study
        .view
        .numeric_column(obj_name)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    let ranks = crate::ui::widgets::rank_plot::compute_rank_percentiles(&obj_values, minimize);
    let mut w = CsvWriter::new();
    let mut header = vec!["trial_id"];
    header.extend(names.iter().map(|n| n.as_str()));
    header.extend([obj_name.as_str(), "rank_percentile"]);
    w.header(header);
    for (i, &tid) in study.view.trial_ids.iter().enumerate() {
        let obj_val = obj_values.get(i).copied().unwrap_or(f64::NAN);
        let rank = ranks.get(i).copied().unwrap_or(f64::NAN);
        let mut fields = vec![CsvField::UInt(tid as u64)];
        fields.extend(names.iter().map(|name| {
            CsvField::Num(
                study
                    .view
                    .numeric_column(name)
                    .and_then(|c| c.get(i))
                    .copied()
                    .unwrap_or(f64::NAN),
            )
        }));
        fields.extend([CsvField::Num(obj_val), CsvField::Num(rank)]);
        w.row(fields);
    }
    Some(w.finish())
}
