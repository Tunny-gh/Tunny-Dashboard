use super::require_study;
use crate::state::app_state::AppState;
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

/// Recomputes box-plot statistics for each column with the current Source/Normalize
/// settings and turns them into CSV (one row per column).
pub(super) fn build_box_plot_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    use crate::ui::widgets::box_plot::{normalize_minmax, BoxPlotSource};

    let study = require_study(app_state)?;
    let names: &[String] = match widgets.box_plot.source {
        BoxPlotSource::Objectives => &study.meta.objective_names,
        BoxPlotSource::Parameters => &study.meta.param_names,
    };
    let normalize = widgets.box_plot.normalize;
    let mut w = CsvWriter::new();
    w.header([
        "column",
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
    let mut any = false;
    for name in names {
        let Some(raw) = study.view.numeric_column(name) else {
            continue;
        };
        let values = if normalize {
            normalize_minmax(raw)
        } else {
            raw.to_vec()
        };
        let Some(s) = tunny_core::statistics::compute_boxplot(&values) else {
            continue;
        };
        any = true;
        w.row([
            CsvField::Text(name),
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
    any.then(|| w.finish())
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
pub(super) fn build_rank_plot_csv(app_state: &AppState, widgets: &WidgetStates) -> Option<String> {
    let study = require_study(app_state)?;
    let x_name = study.meta.param_names.get(widgets.rank_plot.x_param_idx)?;
    let y_name = study.meta.param_names.get(widgets.rank_plot.y_param_idx)?;
    let obj_idx = widgets.rank_plot.obj_idx;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let minimize = !matches!(
        study.meta.directions.get(obj_idx),
        Some(Direction::Maximize)
    );
    let x_col = study.view.numeric_column(x_name);
    let y_col = study.view.numeric_column(y_name);
    let obj_values: Vec<f64> = study
        .view
        .numeric_column(obj_name)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    let ranks = crate::ui::widgets::rank_plot::compute_rank_percentiles(&obj_values, minimize);
    let mut w = CsvWriter::new();
    w.header(["trial_id", x_name, y_name, obj_name, "rank_percentile"]);
    for (i, &tid) in study.view.trial_ids.iter().enumerate() {
        let x_val = x_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
        let y_val = y_col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
        let obj_val = obj_values.get(i).copied().unwrap_or(f64::NAN);
        let rank = ranks.get(i).copied().unwrap_or(f64::NAN);
        w.row([
            CsvField::UInt(tid as u64),
            CsvField::Num(x_val),
            CsvField::Num(y_val),
            CsvField::Num(obj_val),
            CsvField::Num(rank),
        ]);
    }
    Some(w.finish())
}
