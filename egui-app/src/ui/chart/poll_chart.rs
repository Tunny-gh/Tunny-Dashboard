use crate::state::analysis_job::{spawn as spawn_analysis, DeliverySlot as AnalysisKind};
use crate::state::app_state::AppState;
#[cfg(test)]
use crate::state::app_state::Direction;
use crate::state::layout_state::ChartId;
use crate::state::messages::{AppMessage, ClusterChartSource, McdmChartSource};
use crate::ui::widget_states::WidgetStates;
use std::sync::mpsc;
use tdashboard_analysis::preparation::{FeatureSpace, Preparation};

mod cluster;
mod compute;
mod mcdm;
mod surrogate;
#[cfg(test)]
mod tests;
use cluster::*;
pub(crate) use compute::numeric_param_names;
use compute::*;
use mcdm::*;
use surrogate::*;

pub(crate) fn poll_chart_work(
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    app_state.analysis_consumer.set(widgets.analysis_consumer);
    if app_state.current_study.is_none() {
        return;
    }
    match chart_id {
        ChartId::ConvergenceIndicators => poll_convergence_indicators(app_state, widgets, tx),
        ChartId::ImportanceChart => poll_importance_chart(app_state, widgets, tx),
        ChartId::SensitivityHeatmap => poll_sensitivity_heatmap(app_state, widgets, tx),
        ChartId::PdpChart => poll_pdp_chart(app_state, widgets, tx),
        ChartId::PdpChart2D => poll_pdp_chart_2d(app_state, widgets, tx),
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

fn poll_convergence_indicators(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if app_state.study_streaming
        || app_state.convergence_history.is_some()
        || widgets.convergence.computing
    {
        return;
    }
    let indicator = app_state.convergence_indicator;
    let reference = app_state.hv_ref_point_override.clone();
    #[cfg(test)]
    let presentation_target = app_state.current_study.as_ref().unwrap().view.df.clone();
    widgets.convergence.computing = true;
    spawn_analysis(
        app_state,
        tx.clone(),
        AnalysisKind::Convergence,
        (format!("{indicator:?}"), reference.clone()),
        Preparation::Convergence { target_points: 50 },
        move |prepared| {
            use crate::state::results::ConvergenceHistory;
            use tunny_core::indicators::SeriesInput;
            let series = prepared.data().series();
            let input = &prepared.tables()[0];
            let flags: Vec<_> = input
                .meta
                .directions
                .iter()
                .map(|d| matches!(d, tdashboard_study::Direction::Minimize))
                .collect();
            let reference = reference
                .as_ref()
                .filter(|r| r.len() == input.meta.objective_names.len())
                .map(|r| crate::state::ref_point_to_normalized(r, &flags));
            let views: Vec<_> = series
                .iter()
                .map(|s| SeriesInput {
                    trial_ids: &s.trial_ids,
                    objectives: &s.objectives,
                })
                .collect();
            let hist = tunny_core::indicators::compute_indicator_histories(
                &views,
                &flags,
                indicator,
                reference.as_deref(),
            );
            let history = |i: usize| ConvergenceHistory {
                trial_ids: hist.get(i).map(|h| h.trial_ids.clone()).unwrap_or_default(),
                values: hist.get(i).map(|h| h.values.clone()).unwrap_or_default(),
                sample_step: series[i].step,
                ref_point: if i == 0 {
                    hist.first()
                        .map(|h| crate::state::ref_point_to_original(&h.ref_point, &flags))
                        .unwrap_or_default()
                } else {
                    vec![]
                },
            };
            AppMessage::IndicatorHistoryDone {
                #[cfg(test)]
                source_df: presentation_target,
                indicator,
                base: history(0),
                comparisons: (1..series.len()).map(history).collect(),
            }
        },
    );
}

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
    let cached = if metric.is_sobol() {
        app_state
            .sobol_cache
            .contains_key(&(obj_idx, feasible_only))
    } else {
        app_state
            .importance_cache
            .contains_key(&(metric.cache_id(), obj_idx, feasible_only))
    };
    if cached {
        crate::state::analysis_job::supersede(app_state, AnalysisKind::Importance);
        widgets.importance.computing = false;
        return;
    }
    let selection = Preparation::Sensitivity {
        method: heatmap_method(metric),
        objective: Some(obj_idx),
        feasible_only,
        samples: SOBOL_SAMPLE_COUNT,
    };
    spawn_analysis(
        app_state,
        tx.clone(),
        AnalysisKind::Importance,
        (metric.cache_id(), obj_idx, feasible_only),
        selection,
        move |prepared| {
            let df = &prepared.data().sensitivity().df;
            let key = (metric.cache_id(), obj_idx, feasible_only);
            match metric {
                ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => {
                    match tunny_core::sensitivity::compute_sobol_from_df(df, SOBOL_SAMPLE_COUNT) {
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
                    }
                }
                ImportanceMetric::Ard => {
                    match tunny_core::surrogate_opt::compute_ard_importance_from_df(df, obj_idx) {
                        Some(r) => AppMessage::SensitivityDone {
                            key,
                            result: SensitivityResult {
                                param_names: r.param_names,
                                unsupported_categorical: vec![],
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
                }
                _ => {
                    let core = core_sensitivity_metric(metric)
                        .expect("A non-Sobol/ARD metric has a core method");
                    let Some(r) = tunny_core::sensitivity::compute_sensitivity_single_obj(
                        df,
                        vec![core],
                        obj_idx,
                    )
                    .pop() else {
                        return AppMessage::SensitivityError(
                            "Sensitivity computation failed".into(),
                        );
                    };
                    let n = r.spearman.len();
                    let spearman = if n > 0 {
                        vec![(0..n)
                            .map(|i| {
                                r.spearman
                                    .get(i)
                                    .and_then(|r| r.first())
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
                            unsupported_categorical: r.unsupported_categorical,
                            spearman,
                            ridge: r
                                .ridge
                                .into_iter()
                                .map(|r| RidgeResult {
                                    beta: r.beta,
                                    r_squared: r.r_squared,
                                })
                                .collect(),
                            rf_anova: r.rf_anova.map(|r| RfAnovaResult {
                                importances: r.0.importances,
                                r_squared: r.0.r_squared,
                            }),
                            mdi: r.mdi.map(|r| MdiResult {
                                importances: r.0.importances,
                                r_squared: r.0.r_squared,
                            }),
                            shap: r.shap.map(|r| ShapResult {
                                importances: r.0.importances,
                                r_squared: r.0.r_squared,
                            }),
                            permutation: r.permutation.map(|r| PermutationResult {
                                importances: r.0.importances,
                                r_squared: r.0.r_squared,
                            }),
                            ard: None,
                        },
                    }
                }
            }
        },
    );
}

fn poll_sensitivity_heatmap(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some((metric, feasible_only)) = widgets.sensitivity_heatmap.pending_compute.take() else {
        return;
    };
    if app_state
        .sensitivity_heatmap_cache
        .contains_key(&(metric.cache_id(), feasible_only))
    {
        crate::state::analysis_job::supersede(app_state, AnalysisKind::Heatmap);
        widgets.sensitivity_heatmap.computing = false;
        return;
    }
    widgets.sensitivity_heatmap.computing = true;
    spawn_analysis(
        app_state,
        tx.clone(),
        AnalysisKind::Heatmap,
        (metric.cache_id(), feasible_only),
        Preparation::Sensitivity {
            method: heatmap_method(metric),
            objective: None,
            feasible_only,
            samples: crate::ui::widgets::importance_chart::SOBOL_SAMPLE_COUNT,
        },
        move |p| compute_sensitivity_heatmap(metric, feasible_only, &p.data().sensitivity().df),
    );
}

fn poll_pdp_chart(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(req) = widgets.pdp_chart.pending_compute.take() else {
        return;
    };
    let key = crate::ui::widgets::pdp_chart::cache_key(
        &req.param,
        &req.objective,
        &req.model_type,
        req.feasible_only,
    );
    if let Some(cached) = widgets.pdp_chart.cache.get(&key) {
        crate::state::analysis_job::supersede(app_state, AnalysisKind::Pdp1d);
        widgets.pdp_chart.result = Some(cached.clone());
        widgets.pdp_chart.computing = false;
        return;
    }
    let (param, objective, model_type) = (req.param, req.objective, req.model_type);
    let (n_grid, feasible_only) = (req.n_grid, req.feasible_only);
    widgets.pdp_chart.computing = true;
    spawn_analysis(
        app_state,
        tx.clone(),
        AnalysisKind::Pdp1d,
        (
            param.clone(),
            objective.clone(),
            model_type.clone(),
            feasible_only,
            n_grid,
        ),
        Preparation::Pdp {
            params: vec![param.clone()],
            objective: objective.clone(),
            feasible_only,
            two_dimensional: false,
        },
        move |p| {
            let data = p.data().pdp();
            let r = tunny_core::pdp::compute_pdp_from_data(
                data.x.clone(),
                data.y.clone(),
                data.names.clone(),
                &data.objective,
                data.targets[0],
                n_grid,
                &model_type,
            );
            AppMessage::PdpDone {
                param,
                objective,
                model_type,
                feasible_only,
                result: crate::state::messages::PdpResult1d {
                    x_values: r.grid,
                    y_values: r.values,
                    y_upper: r.y_upper,
                    y_lower: r.y_lower,
                    ice_lines: vec![],
                    r2: Some(r.r_squared),
                    param_name: r.param_name,
                },
            }
        },
    );
}

fn poll_pdp_chart_2d(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(req) = widgets.pdp_2d.pending_compute.take() else {
        return;
    };
    widgets.pdp_2d.computing = true;
    spawn_analysis(
        app_state,
        tx.clone(),
        AnalysisKind::Pdp2d,
        (
            req.param1.clone(),
            req.param2.clone(),
            req.objective.clone(),
            req.n_grid,
            req.model_type.clone(),
            req.feasible_only,
        ),
        Preparation::Pdp {
            params: vec![req.param1, req.param2],
            objective: req.objective,
            feasible_only: req.feasible_only,
            two_dimensional: true,
        },
        move |p| match tunny_core::pdp::compute_pdp_2d_from_data(
            p.data().pdp(),
            req.n_grid,
            &req.model_type,
        ) {
            Some(r) => AppMessage::Pdp2dDone(crate::state::messages::PdpResult2d {
                x_values: r.x_values,
                y_values: r.y_values,
                z_values: r.z_values,
                param1_name: r.param1_name,
                param2_name: r.param2_name,
                objective_name: r.objective_name,
                uncertainties: r.uncertainties,
            }),
            None => AppMessage::Error("PDP 2D computation failed".into()),
        },
    );
}

fn dispatch_cluster(
    app_state: &AppState,
    req: crate::ui::widgets::cluster_scatter::ClusterComputeRequest,
    source: ClusterChartSource,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    use crate::ui::widgets::cluster_scatter::{ClusterMatrix, ClusterSpace, KSelectionMode};
    let space = match req.target_space {
        ClusterSpace::Variable => FeatureSpace::Parameters,
        ClusterSpace::Objective => FeatureSpace::Objectives,
        ClusterSpace::Combined => FeatureSpace::Both,
    };
    let manual_k = (req.k_mode == KSelectionMode::Manual).then_some(req.k);
    spawn_analysis(
        app_state,
        tx.clone(),
        AnalysisKind::Cluster(source),
        cluster_conditions(&req),
        Preparation::Cluster { space, manual_k },
        move |p| {
            let data = p.data().features();
            run_cluster_compute(
                source,
                req,
                ClusterMatrix {
                    flat_data: data.matrix.iter().flatten().copied().collect(),
                    n_rows: data.rows.len(),
                    n_cols: data.names.len(),
                    total_trials: data.total_rows,
                    target_indices: data.rows.clone(),
                },
            )
        },
    );
}
fn poll_cluster_scatter(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if !app_state.study_streaming {
        if let Some(req) = widgets.cluster_scatter.pending_compute.take() {
            dispatch_cluster(app_state, req, ClusterChartSource::Scatter2D, tx);
        }
    }
}
fn poll_cluster_scatter_3d(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if !app_state.study_streaming {
        if let Some(req) = widgets.cluster_scatter_3d.pending_compute.take() {
            dispatch_cluster(app_state, req, ClusterChartSource::Scatter3D, tx);
        }
    }
}
fn poll_mcdm_charts(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let ctx = app_state.current_study.as_ref().unwrap();
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
    dispatch_mcdm_entropy(
        app_state,
        controls,
        ctx,
        &ctx.meta.objective_names,
        source,
        tx,
    );
    dispatch_mcdm_compute(
        app_state,
        controls,
        ctx,
        &ctx.meta.objective_names,
        &ctx.meta.directions,
        source,
        tx,
    );
}
fn poll_artifact_gallery(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    use crate::ui::widgets::artifact_gallery::ArtifactViewMode;
    let ctx = app_state.current_study.as_ref().unwrap();
    match widgets.artifact_gallery.mode {
        ArtifactViewMode::Cluster if !app_state.study_streaming => {
            if let Some(req) = widgets.artifact_gallery.cluster_pending.take() {
                dispatch_cluster(app_state, req, ClusterChartSource::ArtifactGallery, tx);
            }
        }
        ArtifactViewMode::Mcdm => {
            let controls = &mut widgets.artifact_gallery.mcdm;
            dispatch_mcdm_entropy(
                app_state,
                controls,
                ctx,
                &ctx.meta.objective_names,
                McdmChartSource::ArtifactGallery,
                tx,
            );
            dispatch_mcdm_compute(
                app_state,
                controls,
                ctx,
                &ctx.meta.objective_names,
                &ctx.meta.directions,
                McdmChartSource::ArtifactGallery,
                tx,
            );
        }
        _ => {}
    }
}
fn poll_observed_contour(
    app_state: &AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    let Some(req) = widgets.observed_contour.pending_compute.take() else {
        return;
    };
    spawn_analysis(
        app_state,
        tx.clone(),
        AnalysisKind::Contour,
        (
            req.x.clone(),
            req.y.clone(),
            req.value.clone(),
            req.n_grid,
            req.max_edge_ratio,
            req.feasible_only,
        ),
        Preparation::Contour {
            x: req.x.clone(),
            y: req.y.clone(),
            value: req.value.clone(),
            feasible_only: req.feasible_only,
            n_grid: req.n_grid,
        },
        move |p| {
            let data = p.data().contour();
            let surface = tunny_core::contour::observed_surface_prepared(
                &data.geometry,
                req.n_grid,
                req.max_edge_ratio,
            );
            if surface.x_values.is_empty() {
                return AppMessage::ObservedContourFailed(
                    "Points are collinear or degenerate; cannot interpolate.".into(),
                );
            }
            AppMessage::ObservedContourDone(crate::state::messages::ObservedContourResult {
                x_name: req.x,
                y_name: req.y,
                value_name: req.value,
                surface,
                points: data.points.clone(),
                point_trial_ids: data.trial_ids.clone(),
            })
        },
    );
}
pub(crate) fn poll_trial_table_work(
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    app_state.analysis_consumer.set(widgets.analysis_consumer);
    use crate::ui::widgets::trial_table::TrialTableMode;
    let Some(ctx) = app_state.current_study.as_ref() else {
        return;
    };
    match widgets.trial_table.mode {
        TrialTableMode::Cluster if !app_state.study_streaming => {
            if let Some(req) = widgets.trial_table.cluster.pending_compute.take() {
                dispatch_cluster(app_state, req, ClusterChartSource::Table, tx);
            }
        }
        TrialTableMode::Mcdm => {
            let controls = &mut widgets.trial_table.mcdm.controls;
            dispatch_mcdm_entropy(
                app_state,
                controls,
                ctx,
                &ctx.meta.objective_names,
                McdmChartSource::Table,
                tx,
            );
            dispatch_mcdm_compute(
                app_state,
                controls,
                ctx,
                &ctx.meta.objective_names,
                &ctx.meta.directions,
                McdmChartSource::Table,
                tx,
            );
        }
        _ => {}
    }
}
