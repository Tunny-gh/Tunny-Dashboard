use std::sync::mpsc;

use crate::state::app_state::{AppState, Direction};
use crate::state::layout_state::ChartId;
use crate::state::messages::{AppMessage, ClusterChartSource, McdmChartSource};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::cluster_scatter::build_cluster_matrix;

mod cluster;
mod compute;
mod mcdm;
mod surrogate;

use cluster::*;
use compute::*;
use mcdm::*;
use surrogate::*;

pub(crate) use compute::numeric_param_names;

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
