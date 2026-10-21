use crate::state::app_state::StudyContext;
use crate::state::messages::AppMessage;

/// List of numeric parameter names, excluding categorical columns (those that can't be numeric-ized).
/// Provides the common filter used by render_chart's combo display and fit-matrix construction.
pub(crate) fn numeric_param_names(ctx: &StudyContext) -> Vec<String> {
    tdashboard_analysis::training::numeric_param_names(&ctx.view.df, &ctx.meta.param_names)
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
    use crate::ui::widgets::importance_chart::SOBOL_SAMPLE_COUNT;
    let result = tdashboard_analysis::sensitivity::heatmap::compute_heatmap(
        df,
        heatmap_method(metric),
        SOBOL_SAMPLE_COUNT,
    );

    AppMessage::SensitivityHeatmapDone {
        metric,
        feasible_only,
        result: HeatmapMatrix {
            param_names: result.param_names,
            unsupported_categorical: result.unsupported_categorical,
            objective_names: result.objective_names,
            values: result.values,
            signed: result.signed,
        },
    }
}

/// Extracts the score for the given parameter from a single-objective computation result (core
/// `SensitivityResult`). Tree-based methods (RF-Anova/MDI/SHAP/Permutation) use `importances[param][0]`,
/// Spearman uses `spearman[param][0]`, Ridge uses `ridge[0].beta[param]`. Sobol doesn't go through this path.
#[cfg(test)]
fn single_obj_param_score(
    r: &tunny_core::sensitivity::SensitivityResult,
    metric: crate::ui::widgets::importance_chart::ImportanceMetric,
    param_idx: usize,
) -> f64 {
    heatmap_method(metric).score(r, param_idx)
}

pub(super) fn heatmap_method(
    metric: crate::ui::widgets::importance_chart::ImportanceMetric,
) -> tdashboard_analysis::sensitivity::heatmap::HeatmapMethod {
    use crate::ui::widgets::importance_chart::ImportanceMetric as M;
    use tdashboard_analysis::sensitivity::heatmap::HeatmapMethod as H;
    match metric {
        M::Spearman => H::Spearman,
        M::Ridge => H::Ridge,
        M::RfAnova => H::RfAnova,
        M::Mdi => H::Mdi,
        M::Shap => H::Shap,
        M::Permutation => H::Permutation,
        M::SobolFirst => H::SobolFirst,
        M::SobolTotal => H::SobolTotal,
        M::Ard => H::Ard,
    }
}

#[cfg(test)]
mod categorical_tests {
    use super::*;
    use crate::ui::widgets::importance_chart::{core_sensitivity_metric, ImportanceMetric};
    use tunny_core::dataframe::{DataFrame, TrialRow};

    #[test]
    fn heatmap_transports_only_numerical_scores_and_explicit_unsupported_names() {
        for mixed in [false, true] {
            let rows: Vec<_> = (0..12)
                .map(|i| TrialRow {
                    trial_id: i,
                    trial_number: i,
                    param_display: [("cat".into(), (i % 3) as f64), ("x".into(), i as f64)].into(),
                    distribution_metadata: Default::default(),
                    param_category_label: [(
                        "cat".into(),
                        ["steel", "wood", "glass"][i as usize % 3].into(),
                    )]
                    .into(),
                    objective_values: vec![i as f64, -(i as f64)],
                    user_attrs_numeric: Default::default(),
                    user_attrs_string: Default::default(),
                    user_attrs_json: Default::default(),
                    constraint_values: vec![],
                })
                .collect();
            let params = if mixed {
                vec!["cat".into(), "x".into()]
            } else {
                vec!["cat".into()]
            };
            let df =
                DataFrame::from_trials(&rows, &params, &["f1".into(), "f2".into()], &[], &[], 0);
            for metric in [ImportanceMetric::Spearman, ImportanceMetric::Ridge] {
                let AppMessage::SensitivityHeatmapDone { result, .. } =
                    compute_sensitivity_heatmap(metric, false, &df)
                else {
                    panic!("expected heatmap");
                };
                assert_eq!(result.unsupported_categorical, vec!["cat"]);
                assert_eq!(result.param_names.len(), usize::from(mixed));
                assert_eq!(result.values.len(), usize::from(mixed));
                if mixed {
                    assert_eq!(result.param_names, vec!["x"]);
                    let core = core_sensitivity_metric(metric).unwrap();
                    for obj in 0..2 {
                        assert_eq!(
                            result.values[0][obj],
                            single_obj_param_score(&core.compute(&df, obj).unwrap(), metric, 0)
                        );
                    }
                }
            }
        }
    }
}
