use std::sync::mpsc;

use crate::state::app_state::{AppState, Direction, StudyContext};
use crate::state::layout_state::ChartId;
use crate::state::messages::{AppMessage, ClusterChartSource, McdmChartSource};
use crate::state::results::{EntropyResult, McdmMethod, McdmResult};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::cluster_scatter::{
    build_cluster_matrix, ClusterCacheKey, ClusterComputeRequest, ClusterMatrix, KSelectionMode,
};
use crate::ui::widgets::mcdm_chart::{McdmCacheKey, McdmComputeRequest, McdmControls};

/// Resolves the per-objective minimize flag from `directions` (shared by multi-objective optimization paths).
/// Returns `n_obj` entries; objectives missing from `directions` fall back to Minimize(true).
fn minimize_flags(directions: &[Direction], n_obj: usize) -> Vec<bool> {
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
fn collect_constraints(
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
type NumericFitXy = (
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
fn build_numeric_fit_xy(ctx: &StudyContext, objective: &str) -> Option<NumericFitXy> {
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
type NumericFitXyMulti = (
    Vec<String>,
    Vec<Vec<f64>>,
    Vec<Vec<f64>>,
    Vec<Option<(f64, f64)>>,
    Vec<usize>,
);

fn build_numeric_fit_xy_multi(
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
fn build_xy_for_objective(
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
fn sensitivity_df(
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
fn compute_sensitivity_heatmap(
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

pub(crate) fn poll_chart_work(
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if app_state.current_study.is_none() {
        return;
    }

    match chart_id {
        ChartId::ParetoScatter2D
        | ChartId::ParetoScatter3D
        | ChartId::OptimizationHistory
        | ChartId::ParallelCoordinates
        | ChartId::ScatterMatrix
        | ChartId::SliceChart
        | ChartId::Histogram
        | ChartId::BoxPlot
        | ChartId::CorrelationMatrix
        | ChartId::RadarComparison
        | ChartId::ComparisonTable
        | ChartId::PcaBiplot
        | ChartId::SomMap
        | ChartId::Dendrogram
        | ChartId::IntermediateValues
        | ChartId::Timeline
        | ChartId::EdfPlot
        | ChartId::RankPlot => return,
        _ => {}
    }

    // Dispatches to a named helper per ChartId. Each helper may assume current_study is
    // Some (guaranteed by the early return above).
    match chart_id {
        ChartId::ConvergenceIndicators => poll_convergence_indicators(app_state, widgets, tx),
        ChartId::ImportanceChart => poll_importance_chart(app_state, widgets, tx),
        ChartId::SensitivityHeatmap => poll_sensitivity_heatmap(app_state, widgets, tx),
        ChartId::PdpChart => poll_pdp_chart(app_state, widgets, tx),
        ChartId::PdpChart2D => poll_pdp_chart_2d(widgets, tx),
        ChartId::ClusterScatter => poll_cluster_scatter(app_state, widgets, tx),
        ChartId::ClusterScatter3D => poll_cluster_scatter_3d(app_state, widgets, tx),
        ChartId::McdmRankChart | ChartId::McdmScatterChart | ChartId::McdmScatterChart3D => {
            poll_mcdm_charts(app_state, widgets, chart_id, tx)
        }
        ChartId::ArtifactGallery => poll_artifact_gallery(app_state, widgets, tx),
        ChartId::ObservedContour => poll_observed_contour(app_state, widgets, tx),
        ChartId::SurrogateOpt => poll_surrogate_opt(app_state, widgets, tx),
        ChartId::Robustness => poll_robustness(app_state, widgets, tx),
        ChartId::ResponseSurface3D => poll_response_surface(app_state, widgets, tx),
        ChartId::SurrogateCompare => poll_surrogate_compare(app_state, widgets, tx),
        _ => {}
    }
}

/// Asynchronously computes the progression of convergence indicators (Hypervolume, etc.). The baseline
/// Study and comparison Studies are computed together so they can be normalized against a common set of reference points.
fn poll_convergence_indicators(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    if app_state.convergence_history.is_some() || widgets.convergence.computing {
        return;
    }

    let is_minimize: Vec<bool> = directions
        .iter()
        .map(|d| matches!(d, Direction::Minimize))
        .collect();

    // Downsample to limit computation cost (up to 50 points).
    const TARGET_POINTS: usize = 50;
    let n_trials = ctx.view.row_count();
    let step = (n_trials / TARGET_POINTS).max(1);
    let obj_cols = ctx.view.numeric_columns(obj_names);
    let sampled_indices: Vec<usize> = (0..n_trials).step_by(step).collect();
    let sampled_ids: Vec<u32> = sampled_indices
        .iter()
        .map(|&i| ctx.view.trial_ids.get(i).copied().unwrap_or(i as u32))
        .collect();
    let sampled_objs: Vec<Vec<f64>> = sampled_indices
        .iter()
        .map(|&i| {
            obj_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    // Downsample independently for each comparison Study.
    let mut comp_ids: Vec<Vec<u32>> = Vec::new();
    let mut comp_objs: Vec<Vec<Vec<f64>>> = Vec::new();
    let mut comp_steps: Vec<usize> = Vec::new();
    for study in &app_state.comparison_studies {
        let comp_obj_names = &study.meta.objective_names;
        let cn = study.view.row_count();
        let cs = (cn / TARGET_POINTS).max(1);
        let comp_obj_cols = study.view.numeric_columns(comp_obj_names);
        let cidxs: Vec<usize> = (0..cn).step_by(cs).collect();
        let cids: Vec<u32> = cidxs
            .iter()
            .map(|&i| study.view.trial_ids.get(i).copied().unwrap_or(i as u32))
            .collect();
        let cobjs: Vec<Vec<f64>> = cidxs
            .iter()
            .map(|&i| {
                comp_obj_cols
                    .iter()
                    .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                    .collect()
            })
            .collect();
        comp_ids.push(cids);
        comp_objs.push(cobjs);
        comp_steps.push(cs);
    }

    // Convert the user-specified reference point (original objective values) into normalized space before passing it in.
    // A specification whose dimensionality doesn't match the objective count is ignored (treated as None), deferring to automatic computation.
    let ref_override_norm: Option<Vec<f64>> = app_state
        .hv_ref_point_override
        .as_ref()
        .filter(|r| r.len() == obj_names.len())
        .map(|r| crate::state::ref_point_to_normalized(r, &is_minimize));
    let is_minimize_for_back = is_minimize.clone();
    let indicator = app_state.convergence_indicator;

    widgets.convergence.computing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::results::ConvergenceHistory;
        use tunny_core::indicators::SeriesInput;

        // Compute all series (baseline + comparisons) together and normalize with a common reference set.
        let mut series = vec![SeriesInput {
            trial_ids: &sampled_ids,
            objectives: &sampled_objs,
        }];
        for i in 0..comp_ids.len() {
            series.push(SeriesInput {
                trial_ids: &comp_ids[i],
                objectives: &comp_objs[i],
            });
        }
        let hist = tunny_core::indicators::compute_indicator_histories(
            &series,
            &is_minimize,
            indicator,
            ref_override_norm.as_deref(),
        );

        let base = if let Some(h) = hist.first() {
            ConvergenceHistory {
                trial_ids: h.trial_ids.clone(),
                values: h.values.clone(),
                sample_step: step,
                // Convert the reference point back to the original objective-value units for display.
                ref_point: crate::state::ref_point_to_original(&h.ref_point, &is_minimize_for_back),
            }
        } else {
            ConvergenceHistory {
                trial_ids: Vec::new(),
                values: Vec::new(),
                sample_step: step,
                ref_point: Vec::new(),
            }
        };

        let comparisons: Vec<ConvergenceHistory> = comp_steps
            .iter()
            .enumerate()
            .map(|(i, &cs)| {
                if let Some(h) = hist.get(i + 1) {
                    ConvergenceHistory {
                        trial_ids: h.trial_ids.clone(),
                        values: h.values.clone(),
                        sample_step: cs,
                        ref_point: Vec::new(),
                    }
                } else {
                    ConvergenceHistory {
                        trial_ids: Vec::new(),
                        values: Vec::new(),
                        sample_step: cs,
                        ref_point: Vec::new(),
                    }
                }
            })
            .collect();

        AppMessage::IndicatorHistoryDone {
            indicator,
            base,
            comparisons,
        }
    });
}

/// Dispatches asynchronous computation of parameter importance (sensitivity analysis).
fn poll_importance_chart(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some((metric, obj_idx, feasible_only)) = widgets.importance.pending_compute.take() else {
        return;
    };
    use crate::state::results::{
        ArdResult, MdiResult, PermutationResult, RfAnovaResult, RidgeResult, SensitivityResult,
        ShapResult, SobolResult,
    };
    use crate::ui::widgets::importance_chart::{
        core_sensitivity_metric, ImportanceMetric, SOBOL_SAMPLE_COUNT,
    };

    let already_cached = if metric.is_sobol() {
        app_state
            .sobol_cache
            .contains_key(&(obj_idx, feasible_only))
    } else {
        app_state
            .importance_cache
            .contains_key(&(metric.cache_id(), obj_idx, feasible_only))
    };

    if already_cached {
        widgets.importance.computing = false;
        return;
    }

    let ctx = app_state.current_study.as_ref().unwrap();
    // Directly use the shared store's DataFrame via Arc::clone (no need to rebuild trial_rows).
    // When feasible_only is set, use a copy containing only feasible solutions.
    let df = sensitivity_df(ctx, feasible_only);
    let tx = tx.clone();
    match metric {
        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => {
            crate::app::spawn_task(
                tx,
                move || match tunny_core::sensitivity::compute_sobol_from_df(
                    &df,
                    SOBOL_SAMPLE_COUNT,
                ) {
                    Some(r) => AppMessage::SobolDone {
                        key: (obj_idx, feasible_only),
                        result: SobolResult {
                            param_names: r.param_names,
                            first_order: r.first_order,
                            total_effect: r.total_effect,
                            r_squared: r.r_squared,
                        },
                    },
                    None => AppMessage::SensitivityError("Sobol computation failed".into()),
                },
            );
        }
        ImportanceMetric::Ard => {
            // ARD trains a GP-FITC and derives importance from its length scales
            // (not a DataFrame metric, so it takes a dedicated path like Sobol).
            let key = (metric.cache_id(), obj_idx, feasible_only);
            crate::app::spawn_task(tx, move || {
                match tunny_core::surrogate_opt::compute_ard_importance_from_df(&df, obj_idx) {
                    Some(r) => AppMessage::SensitivityDone {
                        key,
                        result: SensitivityResult {
                            param_names: r.param_names,
                            spearman: vec![],
                            ridge: vec![],
                            rf_anova: None,
                            mdi: None,
                            shap: None,
                            permutation: None,
                            ard: Some(ArdResult {
                                importances: r.importances,
                                r_squared: r.r_squared,
                            }),
                        },
                    },
                    None => AppMessage::SensitivityError(
                        "ARD importance requires a GP fit (need more trials)".into(),
                    ),
                }
            });
        }
        _ => {
            let Some(core_metric) = core_sensitivity_metric(metric) else {
                return;
            };
            let key = (metric.cache_id(), obj_idx, feasible_only);
            crate::app::spawn_task(tx, move || {
                let mut results = tunny_core::sensitivity::compute_sensitivity_single_obj(
                    &df,
                    vec![core_metric],
                    obj_idx,
                );
                let r = match results.pop() {
                    Some(r) => r,
                    None => {
                        return AppMessage::SensitivityError(
                            "Sensitivity computation failed".into(),
                        )
                    }
                };
                let n_params = r.spearman.len();
                let spearman: Vec<Vec<f64>> = if n_params > 0 {
                    vec![(0..n_params)
                        .map(|pi| {
                            r.spearman
                                .get(pi)
                                .and_then(|row| row.first())
                                .copied()
                                .unwrap_or(0.0)
                        })
                        .collect()]
                } else {
                    vec![]
                };
                AppMessage::SensitivityDone {
                    key,
                    result: SensitivityResult {
                        param_names: r.param_names,
                        spearman,
                        ridge: r
                            .ridge
                            .into_iter()
                            .map(|x| RidgeResult {
                                beta: x.beta,
                                r_squared: x.r_squared,
                            })
                            .collect(),
                        rf_anova: r.rf_anova.map(|x| RfAnovaResult {
                            importances: x.0.importances,
                            r_squared: x.0.r_squared,
                        }),
                        mdi: r.mdi.map(|x| MdiResult {
                            importances: x.0.importances,
                            r_squared: x.0.r_squared,
                        }),
                        shap: r.shap.map(|x| ShapResult {
                            importances: x.0.importances,
                            r_squared: x.0.r_squared,
                        }),
                        permutation: r.permutation.map(|x| PermutationResult {
                            importances: x.0.importances,
                            r_squared: x.0.r_squared,
                        }),
                        ard: None,
                    },
                }
            });
        }
    }
}

/// Dispatches asynchronous computation of the sensitivity heatmap (all parameters x all objectives).
fn poll_sensitivity_heatmap(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    // Asynchronously computes the all-parameter x all-objective sensitivity matrix for the selected method.
    // Compute requests are queued in widgets.sensitivity_heatmap.pending_compute (via the Run button, or
    // auto-triggered for low-cost methods), and results are collected per method into
    // app_state.sensitivity_heatmap_cache.
    let Some((metric, feasible_only)) = widgets.sensitivity_heatmap.pending_compute.take() else {
        return;
    };
    if app_state
        .sensitivity_heatmap_cache
        .contains_key(&(metric.cache_id(), feasible_only))
    {
        widgets.sensitivity_heatmap.computing = false;
        return;
    }
    let ctx = app_state.current_study.as_ref().unwrap();
    let df = sensitivity_df(ctx, feasible_only);
    widgets.sensitivity_heatmap.computing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        compute_sensitivity_heatmap(metric, feasible_only, &df)
    });
}

/// Dispatches asynchronous computation of PDP (1D).
fn poll_pdp_chart(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(req) = widgets.pdp_chart.pending_compute.take() else {
        return;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let Some(target_param_idx) = ctx.meta.param_names.iter().position(|p| p == &req.param) else {
        return;
    };
    let (x_matrix, y) = build_xy_for_objective(ctx, &req.objective, req.feasible_only);
    let param_names_owned = ctx.meta.param_names.clone();
    let (param, objective, model_type) = (req.param, req.objective, req.model_type);
    let (n_grid, feasible_only) = (req.n_grid, req.feasible_only);
    widgets.pdp_chart.computing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::PdpResult1d;
        let r = tunny_core::pdp::compute_pdp_from_data(
            x_matrix,
            y,
            param_names_owned,
            &objective,
            target_param_idx,
            n_grid,
            &model_type,
        );
        AppMessage::PdpDone {
            param,
            objective,
            model_type,
            feasible_only,
            result: PdpResult1d {
                x_values: r.grid,
                y_values: r.values,
                y_upper: r.y_upper,
                y_lower: r.y_lower,
                ice_lines: vec![],
                r2: Some(r.r_squared),
                param_name: r.param_name,
            },
        }
    });
}

/// Dispatches asynchronous computation of PDP (2D).
fn poll_pdp_chart_2d(widgets: &mut WidgetStates, tx: &mpsc::SyncSender<AppMessage>) {
    let Some(req) = widgets.pdp_2d.pending_compute.take() else {
        return;
    };
    widgets.pdp_2d.computing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let result = tunny_core::pdp::compute_pdp_2d(
            &req.param1,
            &req.param2,
            &req.objective,
            req.n_grid,
            &req.model_type,
            req.feasible_only,
        );
        match result {
            Some(r) => {
                use crate::state::messages::PdpResult2d;
                AppMessage::Pdp2dDone(PdpResult2d {
                    x_values: r.x_values,
                    y_values: r.y_values,
                    z_values: r.z_values,
                    param1_name: r.param1_name,
                    param2_name: r.param2_name,
                    objective_name: r.objective_name,
                    uncertainties: r.uncertainties,
                })
            }
            None => AppMessage::Error("PDP 2D computation failed".into()),
        }
    });
}

/// Dispatches the clustering computation for the cluster scatter plot (2D).
fn poll_cluster_scatter(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let ctx = app_state.current_study.as_ref().unwrap();
    let param_names = &ctx.meta.param_names;
    let obj_names = &ctx.meta.objective_names;
    let Some(req) = widgets.cluster_scatter.pending_compute.take() else {
        return;
    };
    match build_cluster_matrix(&ctx.view, param_names, obj_names, req.target_space) {
        Ok(matrix) => {
            let tx = tx.clone();
            crate::app::spawn_task(tx, move || {
                run_cluster_compute(ClusterChartSource::Scatter2D, req, matrix)
            });
        }
        Err(err) => {
            widgets.cluster_scatter.set_error(err);
        }
    }
}

/// Dispatches the clustering computation for the cluster scatter plot (3D).
fn poll_cluster_scatter_3d(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let ctx = app_state.current_study.as_ref().unwrap();
    let param_names = &ctx.meta.param_names;
    let obj_names = &ctx.meta.objective_names;
    let Some(req) = widgets.cluster_scatter_3d.pending_compute.take() else {
        return;
    };
    match build_cluster_matrix(&ctx.view, param_names, obj_names, req.target_space) {
        Ok(matrix) => {
            let tx = tx.clone();
            crate::app::spawn_task(tx, move || {
                run_cluster_compute(ClusterChartSource::Scatter3D, req, matrix)
            });
        }
        Err(err) => {
            widgets.cluster_scatter_3d.set_error(err);
        }
    }
}

/// Dispatches computation for MCDM-family charts (rank / 2D scatter / 3D scatter).
/// Each chart has its own controls, but the dispatch logic is shared.
fn poll_mcdm_charts(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    // Select only the target chart's controls and source, then run the same two steps.
    let (controls, source) = match chart_id {
        ChartId::McdmRankChart => (&mut widgets.mcdm_chart.controls, McdmChartSource::Rank),
        ChartId::McdmScatterChart => (
            &mut widgets.scatter_chart.controls,
            McdmChartSource::Scatter2D,
        ),
        _ => (
            &mut widgets.mcdm_scatter_3d.controls,
            McdmChartSource::Scatter3D,
        ),
    };
    dispatch_mcdm_entropy(controls, ctx, obj_names, source, tx);
    dispatch_mcdm_compute(controls, ctx, obj_names, directions, source, tx);
}

/// Dispatches asynchronous computation for the Artifact Gallery (Cluster / MCDM modes).
fn poll_artifact_gallery(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    use crate::ui::widgets::artifact_gallery::ArtifactViewMode;
    let ctx = app_state.current_study.as_ref().unwrap();
    let param_names = &ctx.meta.param_names;
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    match widgets.artifact_gallery.mode {
        ArtifactViewMode::Cluster => {
            if let Some(req) = widgets.artifact_gallery.cluster_pending.take() {
                match build_cluster_matrix(&ctx.view, param_names, obj_names, req.target_space) {
                    Ok(matrix) => {
                        let tx = tx.clone();
                        crate::app::spawn_task(tx, move || {
                            run_cluster_compute(ClusterChartSource::ArtifactGallery, req, matrix)
                        });
                    }
                    Err(err) => {
                        widgets.artifact_gallery.set_cluster_error(err);
                    }
                }
            }
        }
        ArtifactViewMode::Mcdm => {
            let controls = &mut widgets.artifact_gallery.mcdm;
            dispatch_mcdm_entropy(
                controls,
                ctx,
                obj_names,
                McdmChartSource::ArtifactGallery,
                tx,
            );
            dispatch_mcdm_compute(
                controls,
                ctx,
                obj_names,
                directions,
                McdmChartSource::ArtifactGallery,
                tx,
            );
        }
        ArtifactViewMode::All => {}
    }
}

/// Dispatches asynchronous computation for the Observed Contour (contour interpolated from observed points).
fn poll_observed_contour(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let ctx = app_state.current_study.as_ref().unwrap();
    let Some(req) = widgets.observed_contour.pending_compute.take() else {
        return;
    };
    let (Some(x_col), Some(y_col), Some(v_col)) = (
        ctx.view.numeric_column(&req.x),
        ctx.view.numeric_column(&req.y),
        ctx.view.numeric_column(&req.value),
    ) else {
        widgets.observed_contour.error_message = Some("Selected column not found".to_string());
        widgets.observed_contour.computing = false;
        return;
    };
    let feas = ctx.view.feasibility();
    let n = ctx.view.row_count();
    let mut points: Vec<[f64; 3]> = Vec::with_capacity(n);
    let mut point_trial_ids: Vec<u32> = Vec::with_capacity(n);
    for i in 0..n {
        if req.feasible_only && !feas.is_feasible(i) {
            continue;
        }
        let (Some(&px), Some(&py), Some(&pv)) = (x_col.get(i), y_col.get(i), v_col.get(i)) else {
            continue;
        };
        if !px.is_finite() || !py.is_finite() || !pv.is_finite() {
            continue;
        }
        points.push([px, py, pv]);
        point_trial_ids.push(ctx.view.trial_ids.get(i).copied().unwrap_or(i as u32));
    }
    let (x_name, y_name, value_name) = (req.x, req.y, req.value);
    let n_grid = req.n_grid;
    let max_edge_ratio = req.max_edge_ratio;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::ObservedContourResult;
        if points.len() < 3 {
            return AppMessage::ObservedContourFailed(
                "Not enough finite points to interpolate (need >= 3).".to_string(),
            );
        }
        let surface = tunny_core::contour::observed_surface(&points, n_grid, max_edge_ratio);
        if surface.x_values.is_empty() {
            return AppMessage::ObservedContourFailed(
                "Points are collinear or degenerate; cannot interpolate.".to_string(),
            );
        }
        AppMessage::ObservedContourDone(ObservedContourResult {
            x_name,
            y_name,
            value_name,
            surface,
            points,
            point_trial_ids,
        })
    });
}

/// Processes each stage of surrogate optimization (SurrogateOpt) in priority order.
/// In the order fit -> multi-objective fit -> optimize -> multi-objective optimize -> suggest candidates ->
/// multi-objective suggest candidates, only the first stage with a pending request is executed
/// (same behavior as the original else-if chain).
fn poll_surrogate_opt(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if surrogate_stage_fit(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_multi_fit(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_optimize(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_multi_optimize(app_state, widgets, tx) {
        return;
    }
    if surrogate_stage_suggest(widgets, tx) {
        return;
    }
    surrogate_stage_multi_suggest(app_state, widgets, tx);
}

/// SurrogateOpt: single-objective fit stage. Returns true if a pending request was consumed.
fn surrogate_stage_fit(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(fit_req) = widgets.surrogate_opt.pending_fit.take() else {
        return false;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let Some((numeric_params, x_matrix, y, param_bounds, kept_rows)) =
        build_numeric_fit_xy(ctx, &fit_req.objective)
    else {
        widgets.surrogate_opt.error_message = Some("No numeric parameters available".to_string());
        return true;
    };

    // Clear the previous training and optimization results before starting the fit.
    widgets.surrogate_opt.fitting = true;
    widgets.surrogate_opt.trained = None;
    widgets.surrogate_opt.result = None;
    widgets.surrogate_opt.error_message = None;

    // Extract constraint columns (when use_constraints is set and constraint columns exist). Filtered
    // by kept_rows to stay aligned with the rows excluded by the non-finite filter.
    let constraints = if fit_req.use_constraints {
        collect_constraints(ctx, &kept_rows)
    } else {
        vec![]
    };

    // Shared progress/cancellation handle (shared between the UI and the training thread).
    let progress = tunny_core::surrogate_opt::FitProgress::new();
    widgets.surrogate_opt.fit_progress = Some(progress.clone());

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix,
            y,
            param_names: numeric_params,
            objective_name: fit_req.objective,
            model: fit_req.model,
            auto_select: fit_req.auto_select,
            constraints,
            priority_rows: vec![],
            param_bounds: Some(param_bounds),
        };
        match tunny_core::surrogate_opt::fit_surrogate_with_validation_tracked(
            &fit_core_req,
            &progress,
        ) {
            Ok(t) => AppMessage::SurrogateFitDone(std::sync::Arc::new(t)),
            Err(e) => {
                // Don't show an error for failures caused by cancellation.
                if progress.is_cancelled() {
                    AppMessage::SurrogateFitCancelled
                } else {
                    AppMessage::SurrogateFitFailed(e)
                }
            }
        }
    });
    true
}

/// SurrogateOpt: multi-objective fit stage (trains all objectives). Returns true if a pending request was consumed.
fn surrogate_stage_multi_fit(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(multi_fit_req) = widgets.surrogate_opt.pending_multi_fit.take() else {
        return false;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;
    let objective_names: Vec<String> = obj_names.to_vec();
    let Some((numeric_params, x_matrix, objective_values, param_bounds, _kept_rows)) =
        build_numeric_fit_xy_multi(ctx, obj_names)
    else {
        widgets.surrogate_opt.error_message = Some("No numeric parameters available".to_string());
        return true;
    };
    // Resolve the per-objective minimize flag from directions (same approach as the multi-objective optimization path).
    let minimize_flags = minimize_flags(directions, obj_names.len());

    // Clear the previous multi-objective results before starting the fit.
    widgets.surrogate_opt.fitting = true;
    widgets.surrogate_opt.multi_trained = None;
    widgets.surrogate_opt.multi_result = None;
    widgets.surrogate_opt.error_message = None;

    // Shared progress/cancellation handle (shared between the UI and the training thread).
    let progress = tunny_core::surrogate_opt::FitProgress::new();
    widgets.surrogate_opt.fit_progress = Some(progress.clone());

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        // Train all objectives with Pareto-front concentration
        // (concentrates inducing points on non-dominated trials when N exceeds the GP inducing-point cap).
        match tunny_core::surrogate_opt::fit_multi_surrogates_tracked(
            &x_matrix,
            &objective_values,
            &numeric_params,
            &objective_names,
            multi_fit_req.model,
            &minimize_flags,
            Some(&param_bounds),
            &progress,
        ) {
            Ok(trained_vec) => AppMessage::SurrogateMultiFitDone(std::sync::Arc::new(trained_vec)),
            Err(e) => {
                if progress.is_cancelled() {
                    AppMessage::SurrogateMultiFitCancelled
                } else {
                    AppMessage::SurrogateMultiFitFailed(e)
                }
            }
        }
    });
    true
}

/// SurrogateOpt: single-objective optimization stage. Returns true if a pending request was consumed.
fn surrogate_stage_optimize(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(opt_req) = widgets.surrogate_opt.pending_optimize.take() else {
        return false;
    };
    // The optimization stage requires a trained model.
    let Some(trained) = widgets.surrogate_opt.trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained model available. Run Fit & Validate first.".to_string());
        return true;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    let obj_name = trained.objective_name.clone();
    let obj_idx = obj_names.iter().position(|o| *o == obj_name);
    let minimize = obj_idx
        .and_then(|i| directions.get(i))
        .map(|d| matches!(d, Direction::Minimize))
        .unwrap_or(true);

    // Response-surface slices have been removed, so none are generated.
    let slice_params: Option<(usize, usize)> = None;

    widgets.surrogate_opt.optimizing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateOptUiResult;
        let param_names_owned = trained.param_names.clone();
        let spec = tunny_core::surrogate_opt::SurrogateOptimizeSpec {
            minimize,
            optimizer: opt_req.optimizer,
            slice_params,
            n_grid: tunny_core::surrogate_opt::DEFAULT_SLICE_GRID,
        };
        let constraint_names = trained.constraint_names.clone();
        let r = tunny_core::surrogate_opt::optimize_on_trained(&trained, &spec);
        let predicted_constraints: Vec<(String, f64)> = constraint_names
            .into_iter()
            .zip(r.predicted_constraints)
            .collect();
        AppMessage::SurrogateOptDone(SurrogateOptUiResult {
            best_params: param_names_owned.into_iter().zip(r.best_params).collect(),
            best_value: r.best_value,
            predicted_std: r.predicted_std,
            r_squared: r.r_squared,
            objective_name: obj_name,
            minimize,
            best_observed_value: r.best_observed_value,
            predicted_constraints,
            feasibility_probability: r.feasibility_probability,
        })
    });
    true
}

/// SurrogateOpt: multi-objective optimization stage. Returns true if a pending request was consumed.
fn surrogate_stage_multi_optimize(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    if widgets
        .surrogate_opt
        .pending_multi_optimize
        .take()
        .is_none()
    {
        return false;
    }
    // Multi-objective optimization stage: requires a set of trained surrogates.
    let Some(multi_trained) = widgets.surrogate_opt.multi_trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained multi-objective model. Run Fit & Validate first.".to_string());
        return true;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    // Resolve the per-objective minimize flag from directions.
    let minimize_flags = minimize_flags(directions, obj_names.len());

    // Response-surface slices have been removed, so none are generated.
    let slice_params: Option<(usize, usize)> = None;

    let objective_names_owned = obj_names.to_vec();
    widgets.surrogate_opt.optimizing = true;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateMultiOptUiResult;
        let refs: Vec<&tunny_core::surrogate_opt::TrainedSurrogate> =
            multi_trained.iter().collect();
        let spec = tunny_core::surrogate_opt::SurrogateMultiOptimizeSpec {
            minimize: minimize_flags,
            slice_params,
            n_grid: tunny_core::surrogate_opt::DEFAULT_SLICE_GRID,
        };
        match tunny_core::surrogate_opt::optimize_multi_on_trained(&refs, &spec) {
            Ok(r) => {
                let param_names = refs
                    .first()
                    .map(|t| t.param_names.clone())
                    .unwrap_or_default();
                AppMessage::SurrogateMultiOptDone(SurrogateMultiOptUiResult {
                    param_names,
                    objective_names: objective_names_owned,
                    front: r.front,
                    r_squared: r.r_squared,
                })
            }
            Err(e) => AppMessage::SurrogateMultiOptFailed(e),
        }
    });
    true
}

/// SurrogateOpt: single-objective candidate-suggestion stage. Returns true if a pending request was consumed.
fn surrogate_stage_suggest(widgets: &mut WidgetStates, tx: &mpsc::SyncSender<AppMessage>) -> bool {
    let Some(suggest_req) = widgets.surrogate_opt.pending_suggest.take() else {
        return false;
    };
    // Candidate-suggestion stage: requires a trained GP surrogate.
    let Some(trained) = widgets.surrogate_opt.trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained model available. Run Fit & Validate first.".to_string());
        return true;
    };

    let param_names = trained.param_names.clone();
    let objective_name = trained.objective_name.clone();
    widgets.surrogate_opt.suggesting = true;
    widgets.surrogate_opt.error_message = None;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateSuggestUiResult;
        match tunny_core::surrogate_opt::suggest_candidates(
            &trained,
            suggest_req.n_candidates,
            suggest_req.acquisition,
            suggest_req.minimize,
        ) {
            Ok(candidates) => AppMessage::SurrogateSuggestDone(SurrogateSuggestUiResult {
                candidates,
                param_names,
                objective_name,
            }),
            Err(e) => AppMessage::SurrogateSuggestFailed(e),
        }
    });
    true
}

/// SurrogateOpt: multi-objective candidate-suggestion stage (EHVI). Returns true if a pending request was consumed.
fn surrogate_stage_multi_suggest(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) -> bool {
    let Some(multi_suggest_req) = widgets.surrogate_opt.pending_multi_suggest.take() else {
        return false;
    };
    // Multi-objective candidate-suggestion stage (EHVI): requires a set of trained GP surrogates.
    let Some(multi_trained) = widgets.surrogate_opt.multi_trained.clone() else {
        widgets.surrogate_opt.error_message =
            Some("No trained multi-objective model. Run Fit & Validate first.".to_string());
        return true;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;

    // Resolve the per-objective minimize flag from directions.
    let minimize_flags = minimize_flags(directions, obj_names.len());

    let param_names = multi_trained
        .first()
        .map(|t| t.param_names.clone())
        .unwrap_or_default();
    let objective_names = obj_names.to_vec();
    widgets.surrogate_opt.multi_suggesting = true;
    widgets.surrogate_opt.error_message = None;
    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::SurrogateMultiSuggestUiResult;
        match tunny_core::surrogate_opt::suggest_candidates_multi(
            &multi_trained,
            &minimize_flags,
            multi_suggest_req.n_candidates,
        ) {
            Ok(candidates) => {
                AppMessage::SurrogateMultiSuggestDone(SurrogateMultiSuggestUiResult {
                    candidates,
                    param_names,
                    objective_names,
                })
            }
            Err(e) => AppMessage::SurrogateMultiSuggestFailed(e),
        }
    });
    true
}

/// Dispatches the fit stage of robustness analysis.
fn poll_robustness(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(fit_req) = widgets.robustness.pending_fit.take() else {
        return;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    // Check for the existence of numeric parameters first (preserving the original validation order).
    if numeric_param_names(ctx).is_empty() {
        widgets.robustness.fit_error = Some("No numeric parameters available".to_string());
        widgets.robustness.fitting = false;
        return;
    }
    let Some(objective) = obj_names.get(fit_req.objective_index).cloned() else {
        widgets.robustness.fit_error = Some("Invalid objective selection".to_string());
        widgets.robustness.fitting = false;
        return;
    };
    let Some((numeric_params, x_matrix, y, param_bounds, kept_rows)) =
        build_numeric_fit_xy(ctx, &objective)
    else {
        widgets.robustness.fit_error = Some("No numeric parameters available".to_string());
        widgets.robustness.fitting = false;
        return;
    };

    // Robustness analysis also wants the constraint feasibility rate, so always pass constraints when present.
    let constraints = collect_constraints(ctx, &kept_rows);

    widgets.robustness.trained = None;
    widgets.robustness.fit_error = None;

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix,
            y,
            param_names: numeric_params,
            objective_name: objective,
            model: fit_req.model,
            auto_select: false,
            constraints,
            priority_rows: vec![],
            param_bounds: Some(param_bounds),
        };
        match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
            Ok(t) => AppMessage::RobustnessFitDone(std::sync::Arc::new(t)),
            Err(e) => AppMessage::RobustnessFitFailed(e),
        }
    });
}

/// Dispatches the fit stage of the 3D response surface.
fn poll_response_surface(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(fit_req) = widgets.response_surface.pending_fit.take() else {
        return;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    // Check for the existence of numeric parameters first (preserving the original validation order).
    if numeric_param_names(ctx).is_empty() {
        widgets.response_surface.fit_error = Some("No numeric parameters available".to_string());
        widgets.response_surface.fitting = false;
        return;
    }
    let Some(objective) = obj_names.get(fit_req.objective_index).cloned() else {
        widgets.response_surface.fit_error = Some("Invalid objective selection".to_string());
        widgets.response_surface.fitting = false;
        return;
    };
    let Some((numeric_params, x_matrix, y, param_bounds, _kept_rows)) =
        build_numeric_fit_xy(ctx, &objective)
    else {
        widgets.response_surface.fit_error = Some("No numeric parameters available".to_string());
        widgets.response_surface.fitting = false;
        return;
    };

    widgets.response_surface.trained = None;
    widgets.response_surface.fit_error = None;

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
            x_matrix,
            y,
            param_names: numeric_params,
            objective_name: objective,
            model: fit_req.model,
            auto_select: false,
            // Response-surface slice evaluation doesn't handle feasibility, so no constraints are passed.
            constraints: vec![],
            priority_rows: vec![],
            param_bounds: Some(param_bounds),
        };
        match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
            Ok(t) => AppMessage::ResponseSurfaceFitDone(std::sync::Arc::new(t)),
            Err(e) => AppMessage::ResponseSurfaceFitFailed(e),
        }
    });
}

/// Dispatches computation for Compare Surrogates (CV metric comparison across all model kinds + prediction slices).
fn poll_surrogate_compare(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(req) = widgets.surrogate_compare.pending.take() else {
        return;
    };
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let directions = &ctx.meta.directions;
    // Check for the existence of numeric parameters first (preserving the original validation order).
    if numeric_param_names(ctx).is_empty() {
        widgets.surrogate_compare.error = Some("No numeric parameters available".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    }
    let Some(objective) = obj_names.get(req.objective_index).cloned() else {
        widgets.surrogate_compare.error = Some("Invalid objective selection".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };
    let Some((numeric_params, x_matrix, y, param_bounds, kept_rows)) =
        build_numeric_fit_xy(ctx, &objective)
    else {
        widgets.surrogate_compare.error = Some("No numeric parameters available".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };
    if req.slice_param >= numeric_params.len() {
        widgets.surrogate_compare.error = Some("Invalid slice parameter selection".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    }

    // Anchor: the observed best row for the selected objective (direction-aware). Since best_trial_row
    // returns the row index in the original df, map it to its position within kept_rows for the
    // non-finite-filtered x_matrix.
    let Some(best_row) =
        crate::ui::widgets::anchor::best_trial_row(&ctx.view, obj_names, directions, &objective)
    else {
        widgets.surrogate_compare.error = Some("Could not resolve an anchor point".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };
    let Some(anchor) = kept_rows
        .iter()
        .position(|&r| r == best_row)
        .and_then(|pos| x_matrix.get(pos))
        .cloned()
    else {
        widgets.surrogate_compare.error = Some("Could not resolve an anchor point".to_string());
        widgets.surrogate_compare.computing = false;
        return;
    };

    let slice_param = req.slice_param;
    let n = x_matrix.len();
    let observed: Vec<(f64, f64)> = (0..n)
        .filter_map(|i| {
            let xv = x_matrix.get(i)?.get(slice_param).copied()?;
            let yv = y.get(i).copied()?;
            (xv.is_finite() && yv.is_finite()).then_some((xv, yv))
        })
        .collect();

    let kinds = crate::ui::widgets::compare::model_kinds(req.include_moe);
    let param_name = numeric_params[slice_param].clone();
    let objective_name = objective.clone();

    widgets.surrogate_compare.error = None;

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        use crate::state::messages::{SurrogateCompareRow, SurrogateCompareUiResult};

        let mut rows: Vec<SurrogateCompareRow> = Vec::with_capacity(kinds.len());
        let mut slices = Vec::new();

        for kind in kinds {
            let fit_core_req = tunny_core::surrogate_opt::SurrogateFitRequest {
                x_matrix: x_matrix.clone(),
                y: y.clone(),
                param_names: numeric_params.clone(),
                objective_name: objective_name.clone(),
                model: kind,
                auto_select: false,
                // Constraints aren't handled since this is a simple comparison view (same reason as ResponseSurface3D).
                constraints: vec![],
                priority_rows: vec![],
                param_bounds: Some(param_bounds.clone()),
            };
            match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
                Ok(trained) => {
                    let v = &trained.validation;
                    rows.push(SurrogateCompareRow {
                        kind,
                        cv_r2_mean: v.cv_r2_mean,
                        cv_r2_std: v.cv_r2_std,
                        holdout_r2: v.holdout_r2,
                        holdout_rmse: v.holdout_rmse,
                        train_r2: v.train_r2,
                        error: None,
                    });
                    if let Some(slice) =
                        tunny_core::surrogate_opt::line_slice_at(&trained, &anchor, slice_param, 60)
                    {
                        slices.push((kind, slice));
                    }
                }
                Err(e) => {
                    rows.push(SurrogateCompareRow {
                        kind,
                        cv_r2_mean: 0.0,
                        cv_r2_std: 0.0,
                        holdout_r2: 0.0,
                        holdout_rmse: 0.0,
                        train_r2: 0.0,
                        error: Some(e),
                    });
                }
            }
        }

        // Only report Failed if all models failed (partial failures are shown as per-row errors).
        if rows.iter().all(|r| r.error.is_some()) {
            let combined = rows
                .iter()
                .filter_map(|r| r.error.clone())
                .collect::<Vec<_>>()
                .join("; ");
            AppMessage::SurrogateCompareFailed(format!("All models failed to fit: {combined}"))
        } else {
            AppMessage::SurrogateCompareDone(std::sync::Arc::new(SurrogateCompareUiResult {
                rows,
                slices,
                observed,
                param_name,
                objective_name,
                anchor,
            }))
        }
    });
}

/// Dispatches asynchronous computation for the unified trial table (`PanelItem::TrialTable`).
/// Depending on the current mode, launches clustering for Cluster or MCDM computation for MCDM.
/// Results are shared and cached under the same `ClusterChartSource::Table` /
/// `McdmChartSource::Table` as the Cluster/MCDM tables.
pub(crate) fn poll_trial_table_work(
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    use crate::ui::widgets::trial_table::TrialTableMode;

    if app_state.current_study.is_none() {
        return;
    }
    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let param_names = &ctx.meta.param_names;
    let directions = &ctx.meta.directions;

    match widgets.trial_table.mode {
        TrialTableMode::Cluster => {
            if let Some(req) = widgets.trial_table.cluster.pending_compute.take() {
                match build_cluster_matrix(&ctx.view, param_names, obj_names, req.target_space) {
                    Ok(matrix) => {
                        let tx = tx.clone();
                        crate::app::spawn_task(tx, move || {
                            run_cluster_compute(ClusterChartSource::Table, req, matrix)
                        });
                    }
                    Err(err) => {
                        widgets.trial_table.cluster.set_error(err);
                    }
                }
            }
        }
        TrialTableMode::Mcdm => {
            let controls = &mut widgets.trial_table.mcdm.controls;
            dispatch_mcdm_entropy(controls, ctx, obj_names, McdmChartSource::Table, tx);
            dispatch_mcdm_compute(
                controls,
                ctx,
                obj_names,
                directions,
                McdmChartSource::Table,
                tx,
            );
        }
        TrialTableMode::All => {}
    }
}

/// Launches the Entropy weight computation if needed (from each chart's controls).
fn dispatch_mcdm_entropy(
    controls: &mut McdmControls,
    ctx: &StudyContext,
    obj_names: &[String],
    source: McdmChartSource,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if !controls.pending_entropy || controls.computing {
        return;
    }
    let n_trials = ctx.view.row_count();
    let obj_cols = ctx.view.numeric_columns(obj_names);
    let objectives: Vec<f64> = (0..n_trials)
        .flat_map(|i| {
            obj_cols
                .iter()
                .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
        })
        .collect();
    let n_objectives = obj_names.len();
    if n_trials == 0 || n_objectives == 0 {
        return;
    }

    controls.computing = true;
    let tx = tx.clone();
    crate::app::spawn_task(
        tx,
        move || match tunny_core::entropy::compute_entropy_weights(
            &objectives,
            n_trials,
            n_objectives,
        ) {
            Ok(r) => AppMessage::EntropyDone {
                source,
                result: EntropyResult {
                    weights: r.weights,
                    entropies: r.entropies,
                    diversities: r.diversities,
                    duration_ms: r.duration_ms,
                },
            },
            Err(e) => AppMessage::McdmFailed {
                source,
                message: format!("Entropy computation failed: {e}"),
            },
        },
    );
}

/// Launches the MCDM ranking computation if needed (from each chart's controls).
/// The result is returned with a config key and stored in `app_state.mcdm_cache`.
fn dispatch_mcdm_compute(
    controls: &mut McdmControls,
    ctx: &StudyContext,
    obj_names: &[String],
    directions: &[Direction],
    source: McdmChartSource,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(req) = controls.pending_compute.take() else {
        return;
    };
    controls.computing = true;

    let key = McdmCacheKey::from_request(&req, controls.weight_mode);
    let McdmComputeRequest { method, weights, v } = req;

    let n_total = ctx.view.row_count();
    let n_objectives = obj_names.len();

    // Target only the row indices on the Pareto front (rank == 0)
    let pareto_row_indices: Vec<usize> = (0..n_total)
        .filter(|&i| ctx.view.pareto_rank.get(i).copied().unwrap_or(u32::MAX) == 0)
        .collect();
    let n_pareto = pareto_row_indices.len();

    let obj_cols_mcdm = ctx.view.numeric_columns(obj_names);
    let objectives: Vec<f64> = pareto_row_indices
        .iter()
        .flat_map(|&i| {
            obj_cols_mcdm
                .iter()
                .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
        })
        .collect();
    let is_minimize: Vec<bool> = directions
        .iter()
        .map(|d| matches!(d, Direction::Minimize))
        .collect();

    let tx = tx.clone();
    crate::app::spawn_task(tx, move || {
        let computed = compute_mcdm_result(
            method,
            v,
            &weights,
            &objectives,
            n_total,
            n_pareto,
            n_objectives,
            &is_minimize,
            &pareto_row_indices,
        );
        match computed {
            Ok(result) => AppMessage::McdmDone {
                source,
                key,
                result,
            },
            Err(message) => AppMessage::McdmFailed { source, message },
        }
    });
}

/// Computes MCDM over the Pareto-front subset and returns the result expanded to full trial length.
#[allow(clippy::too_many_arguments)]
fn compute_mcdm_result(
    method: McdmMethod,
    v: f64,
    weights: &[f64],
    objectives: &[f64],
    n_total: usize,
    n_pareto: usize,
    n_objectives: usize,
    is_minimize: &[bool],
    pareto_row_indices: &[usize],
) -> Result<McdmResult, String> {
    let start = std::time::Instant::now();

    if n_pareto == 0 {
        return Err("MCDM: Pareto front is empty. Run the optimizer first.".to_string());
    }

    // Helper that converts an index within the subset to a full-trial index
    let remap = |subset_idx: u32| -> u32 {
        pareto_row_indices
            .get(subset_idx as usize)
            .copied()
            .unwrap_or(0) as u32
    };
    let expand_scores = |subset_scores: Vec<f64>| -> Vec<f64> {
        let mut full = vec![0.0f64; n_total];
        for (j, &row) in pareto_row_indices.iter().enumerate() {
            full[row] = subset_scores[j];
        }
        full
    };
    let expand_counts = |subset_counts: Vec<u32>| -> Vec<u32> {
        let mut full = vec![0u32; n_total];
        for (j, &row) in pareto_row_indices.iter().enumerate() {
            full[row] = subset_counts[j];
        }
        full
    };

    match method {
        McdmMethod::Topsis => tunny_core::topsis::compute_topsis(
            objectives,
            n_pareto,
            n_objectives,
            weights,
            is_minimize,
        )
        .map(|r| {
            McdmResult::Topsis(crate::state::results::TopsisResult {
                scores: expand_scores(r.scores),
                ranked_indices: r.ranked_indices.into_iter().map(remap).collect(),
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            })
        })
        .map_err(|e| format!("TOPSIS computation failed: {e}")),
        McdmMethod::Vikor => tunny_core::vikor::compute_vikor(
            objectives,
            n_pareto,
            n_objectives,
            weights,
            is_minimize,
            v,
        )
        .map(|r| {
            McdmResult::Vikor(crate::state::results::VikorResult {
                s_values: expand_scores(r.s_values),
                r_values: expand_scores(r.r_values),
                q_values: expand_scores(r.q_values),
                display_scores: expand_scores(r.display_scores),
                ranked_indices: r.ranked_indices.into_iter().map(remap).collect(),
                compromise_indices: r
                    .compromise_indices
                    .into_iter()
                    .map(|i| remap(i as u32) as usize)
                    .collect(),
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            })
        })
        .map_err(|e| format!("VIKOR computation failed: {e}")),
        McdmMethod::PrometheeI | McdmMethod::PrometheeII => {
            tunny_core::promethee::compute_promethee(
                objectives,
                n_pareto,
                n_objectives,
                weights,
                is_minimize,
            )
            .map(|r| {
                let result = crate::state::results::PrometheeResult {
                    phi_plus: expand_scores(r.phi_plus),
                    phi_minus: expand_scores(r.phi_minus),
                    phi_net: expand_scores(r.phi_net),
                    ranked_indices_i: r.ranked_indices_i.into_iter().map(&remap).collect(),
                    ranked_indices_ii: r.ranked_indices_ii.into_iter().map(remap).collect(),
                    incomparable_counts: expand_counts(r.incomparable_counts),
                    duration_ms: r.duration_ms,
                };
                if method == McdmMethod::PrometheeI {
                    McdmResult::PrometheeI(result)
                } else {
                    McdmResult::PrometheeII(result)
                }
            })
            .map_err(|e| format!("PROMETHEE computation failed: {e}"))
        }
    }
}

fn run_cluster_compute(
    source: ClusterChartSource,
    req: ClusterComputeRequest,
    matrix: ClusterMatrix,
) -> AppMessage {
    let key = ClusterCacheKey::from_request(&req);
    let trial_count = matrix.n_rows; // Number of Pareto-front solutions (rows passed to k-means)
    let n_cols = matrix.n_cols;

    if !matrix.is_valid_for_clustering() {
        return cluster_failed(
            source,
            "At least 2 trials and one feature are required.",
            Some(format!(
                "validation: trial_count({trial_count}), n_cols({n_cols})"
            )),
            false,
        );
    }

    let init_strategy: tunny_core::clustering::InitStrategy = req.init_strategy.into();
    let selected_k = match req.k_mode {
        KSelectionMode::ElbowDefault => {
            let elbow = tunny_core::clustering::estimate_k_elbow(
                &matrix.flat_data,
                n_cols,
                trial_count.min(req.elbow_max_k.clamp(2, 50)),
            );
            elbow.recommended_k.clamp(2, trial_count)
        }
        KSelectionMode::Manual => req.k,
    };

    if selected_k < 2 || selected_k > trial_count {
        return cluster_failed(
            source,
            "k must be in [2, trial_count].",
            Some(format!(
                "validation: k({selected_k}) outside [2, {trial_count}]"
            )),
            true,
        );
    }

    let result =
        tunny_core::clustering::run_kmeans(selected_k, &matrix.flat_data, n_cols, init_strategy);
    if result.labels.len() != trial_count {
        return cluster_failed(
            source,
            "Cluster result is inconsistent. Please run again.",
            Some(format!(
                "validation: labels_len({}) != trial_count({trial_count})",
                result.labels.len()
            )),
            true,
        );
    }

    // Expand Pareto-front labels to cover all trials (solutions not included get -1)
    let mut full_labels = vec![-1i32; matrix.total_trials];
    for (matrix_row, &trial_idx) in matrix.target_indices.iter().enumerate() {
        if let Some(&label) = result.labels.get(matrix_row) {
            full_labels[trial_idx] = label as i32;
        }
    }

    AppMessage::ClusteringDone {
        source,
        key,
        result: crate::state::results::ClusterResult {
            labels: full_labels,
            n_clusters: selected_k,
        },
    }
}

fn cluster_failed(
    source: ClusterChartSource,
    message: &str,
    detail: Option<String>,
    retryable: bool,
) -> AppMessage {
    AppMessage::ClusterFailed {
        source,
        err: crate::state::messages::cluster_ui_error(message, detail, retryable),
    }
}
