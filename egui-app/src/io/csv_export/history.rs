use crate::state::app_state::AppState;
use crate::state::types::Direction;
use crate::ui::widget_states::WidgetStates;
use tunny_core::export::{CsvField, CsvWriter};

pub(super) fn build_optimization_history_csv(
    app_state: &AppState,
    widgets: &WidgetStates,
) -> Option<String> {
    let study = app_state.current_study.as_ref()?;
    let obj_idx = widgets.opt_history.obj_idx;
    let obj_name = study.meta.objective_names.get(obj_idx)?;
    let obj_col = study.view.numeric_column(obj_name)?;
    if obj_col.is_empty() {
        return None;
    }
    let is_minimize = !matches!(
        study.meta.directions.get(obj_idx),
        Some(Direction::Maximize)
    );
    let mut w = CsvWriter::new();
    w.header(["trial_index", "objective_value", "best_value"]);
    let mut best = if is_minimize {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    };
    for (i, &val) in obj_col.iter().enumerate() {
        if val.is_finite() {
            best = if is_minimize {
                best.min(val)
            } else {
                best.max(val)
            };
        }
        w.row([
            CsvField::UInt(i as u64),
            CsvField::Num(val),
            CsvField::Num(best),
        ]);
    }
    Some(w.finish())
}

pub(super) fn build_convergence_csv(app_state: &AppState) -> Option<String> {
    let history = app_state.convergence_history.as_ref()?;
    let label = app_state.convergence_indicator.label();
    let mut w = CsvWriter::new();
    w.header(["trial_index", label]);
    for (i, &val) in history.values.iter().enumerate() {
        let trial_idx = i * history.sample_step;
        w.row([CsvField::UInt(trial_idx as u64), CsvField::Num(val)]);
    }
    Some(w.finish())
}

/// Outputs all trials and all steps of Intermediate Values in long format (no
/// thinning).
pub(super) fn build_intermediate_values_csv() -> Option<String> {
    let extras = tunny_core::dataframe::active_extras_snapshot()?;
    if !extras.has_intermediate() {
        return None;
    }
    // CSV export doesn't apply the display thinning (MAX_CURVES) and outputs all
    // trials.
    let (curves, _total) = crate::ui::widgets::intermediate_values::build_intermediate_curves(
        &extras.trials,
        false,
        usize::MAX,
    );
    if curves.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header(["trial_number", "state", "step", "value"]);
    for c in &curves {
        for &[step, value] in &c.points {
            w.row([
                CsvField::UInt(c.trial_number as u64),
                CsvField::Text(c.state.label()),
                CsvField::Num(step),
                CsvField::Num(value),
            ]);
        }
    }
    Some(w.finish())
}

/// Outputs the start/end (elapsed seconds) of every trial in Timeline.
pub(super) fn build_timeline_csv() -> Option<String> {
    let extras = tunny_core::dataframe::active_extras_snapshot()?;
    if !extras.has_datetimes() {
        return None;
    }
    let bars = crate::ui::widgets::timeline::build_timeline_bars(&extras.trials);
    if bars.is_empty() {
        return None;
    }
    let mut w = CsvWriter::new();
    w.header([
        "trial_number",
        "state",
        "start_elapsed_s",
        "end_elapsed_s",
        "duration_s",
    ]);
    for b in &bars {
        w.row([
            CsvField::UInt(b.trial_number as u64),
            CsvField::Text(b.state.label()),
            CsvField::Num(b.start),
            CsvField::Num(b.end),
            CsvField::Num(b.end - b.start),
        ]);
    }
    Some(w.finish())
}
