use crate::ui::widget_states::WidgetStates;
use tunny_core::export::{CsvField, CsvWriter};

/// Turns the surrogate optimization's estimated optimum into CSV.
/// If a multi-objective result exists, the front-point table is output preferentially.
/// For single-objective, outputs parameter rows plus a predicted-value summary row.
pub(super) fn build_surrogate_opt_csv(widgets: &WidgetStates) -> Option<String> {
    // Prefer the multi-objective result.
    if let Some(ref multi) = widgets.surrogate_opt.multi_result {
        return Some(build_surrogate_multi_opt_csv(multi));
    }

    let result = widgets.surrogate_opt.result.as_ref()?;
    let mut w = CsvWriter::new();
    w.header(["name", "value"]);
    for (name, value) in &result.best_params {
        w.row([CsvField::Text(name), CsvField::Num(*value)]);
    }
    let direction = if result.minimize {
        "minimize"
    } else {
        "maximize"
    };
    let predicted_label = format!("predicted_{}({})", direction, result.objective_name);
    w.row([
        CsvField::Text(&predicted_label),
        CsvField::Num(result.best_value),
    ]);
    if let Some(std) = result.predicted_std {
        w.row([CsvField::Text("predicted_std"), CsvField::Num(std)]);
    }
    w.row([CsvField::Text("r_squared"), CsvField::Num(result.r_squared)]);

    // Append validation metrics (if a trained model is held).
    if let Some(ref trained) = widgets.surrogate_opt.trained {
        let v = &trained.validation;
        w.row([CsvField::Text("train_r2"), CsvField::Num(v.train_r2)]);
        w.row([CsvField::Text("holdout_r2"), CsvField::Num(v.holdout_r2)]);
        w.row([
            CsvField::Text("holdout_rmse"),
            CsvField::Num(v.holdout_rmse),
        ]);
        w.row([CsvField::Text("cv_r2_mean"), CsvField::Num(v.cv_r2_mean)]);
        w.row([CsvField::Text("cv_r2_std"), CsvField::Num(v.cv_r2_std)]);
        w.row([
            CsvField::Text("cv_rmse_mean"),
            CsvField::Num(v.cv_rmse_mean),
        ]);
        w.row([CsvField::Text("cv_rmse_std"), CsvField::Num(v.cv_rmse_std)]);
    }

    Some(w.finish())
}

/// Turns the robustness analysis's output samples into a single-column CSV. Returns just
/// the header if there's no cached result.
pub(super) fn build_robustness_csv(widgets: &WidgetStates) -> Option<String> {
    let mut w = CsvWriter::new();
    w.header(["sample"]);
    if let Some(result) = widgets.robustness.cached_result() {
        for &v in &result.samples {
            w.row([CsvField::Num(v)]);
        }
    }
    Some(w.finish())
}

/// Turns Compare Surrogates's CV metric comparison table into CSV. Models that failed to
/// fit have their numeric fields left blank. Returns None if there's no result.
pub(super) fn build_surrogate_compare_csv(widgets: &WidgetStates) -> Option<String> {
    let result = widgets.surrogate_compare.result.as_ref()?;
    let mut w = CsvWriter::new();
    w.header([
        "model",
        "cv_r2_mean",
        "cv_r2_std",
        "holdout_r2",
        "holdout_rmse",
        "train_r2",
    ]);
    for row in &result.rows {
        let model_name = crate::ui::widgets::surrogate_opt::model_label(row.kind);
        if row.error.is_some() {
            w.row([
                CsvField::Text(model_name),
                CsvField::Empty,
                CsvField::Empty,
                CsvField::Empty,
                CsvField::Empty,
                CsvField::Empty,
            ]);
        } else {
            w.row([
                CsvField::Text(model_name),
                CsvField::Num(row.cv_r2_mean),
                CsvField::Num(row.cv_r2_std),
                CsvField::Num(row.holdout_r2),
                CsvField::Num(row.holdout_rmse),
                CsvField::Num(row.train_r2),
            ]);
        }
    }
    Some(w.finish())
}

/// Turns the multi-objective surrogate optimization's front-point table into CSV.
/// Header row = objective names + parameter names, one row per front point.
pub(super) fn build_surrogate_multi_opt_csv(
    result: &crate::state::messages::SurrogateMultiOptUiResult,
) -> String {
    let mut w = CsvWriter::new();
    // Header row
    let headers: Vec<&str> = result
        .objective_names
        .iter()
        .map(|s| s.as_str())
        .chain(result.param_names.iter().map(|s| s.as_str()))
        .collect();
    w.header(headers);
    // Data rows (one row per front point)
    for pt in &result.front {
        let fields: Vec<CsvField> = pt
            .values
            .iter()
            .chain(pt.params.iter())
            .map(|&v| CsvField::Num(v))
            .collect();
        w.row(fields);
    }
    w.finish()
}
