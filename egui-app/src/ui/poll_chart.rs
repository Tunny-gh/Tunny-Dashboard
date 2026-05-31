use std::sync::mpsc;

use crate::state::app_state::{AppState, Direction};
use crate::state::layout_state::ChartId;
use crate::state::messages::AppMessage;
use crate::state::results::{AhpResult, EntropyResult, McdmMethod};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::cluster_scatter::{
    build_cluster_matrix, ClusterComputeRequest, ClusterMatrix, KSelectionMode,
};
use crate::ui::widgets::mcdm_chart::McdmComputeRequest;

fn build_xy_for_objective(
    ctx: &crate::state::app_state::StudyContext,
    objective: &str,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let param_names = &ctx.meta.param_names;
    let n = ctx.view.row_count();

    let param_cols = ctx.view.numeric_columns(param_names);

    let x_matrix: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            param_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    let y: Vec<f64> = ctx
        .view
        .numeric_column(objective)
        .map(|col| col.to_vec())
        .unwrap_or_else(|| vec![0.0; n]);

    (x_matrix, y)
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
        | ChartId::SensitivityHeatmap
        | ChartId::McdmScatterChart
        | ChartId::McdmTable
        | ChartId::AhpTable
        | ChartId::SliceChart => return,
        _ => {}
    }

    let ctx = app_state.current_study.as_ref().unwrap();
    let obj_names = &ctx.meta.objective_names;
    let param_names = &ctx.meta.param_names;
    let directions = &ctx.meta.directions;

    match chart_id {
        ChartId::HvHistory => {
            if app_state.hv_history.is_none() && !widgets.hv_history.computing {
                let is_minimize: Vec<bool> = directions
                    .iter()
                    .map(|d| matches!(d, Direction::Minimize))
                    .collect();

                // HV computation is expensive; downsample so each dispatch stays fast
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

                widgets.hv_history.computing = true;
                let tx = tx.clone();
                crate::app::spawn_task(tx, move || {
                    let result = tunny_core::pareto::compute_hv_history_from_data(
                        &sampled_ids,
                        &sampled_objs,
                        &is_minimize,
                    );
                    AppMessage::HvHistoryDone {
                        trial_ids: result.trial_ids,
                        hv_values: result.hv_values,
                        sample_step: step,
                    }
                });
            }
        }
        ChartId::ImportanceChart => {
            if let Some((metric, obj_idx)) = widgets.importance.pending_compute.take() {
                use crate::state::results::{
                    MdiResult, PermutationResult, RfAnovaResult, RidgeResult, SensitivityResult,
                    ShapResult, SobolResult,
                };
                use crate::ui::widgets::importance_chart::ImportanceMetric;

                let already_cached = if metric.is_sobol() {
                    app_state.sobol_cache.contains_key(&obj_idx)
                } else {
                    app_state
                        .importance_cache
                        .contains_key(&(metric.cache_id(), obj_idx))
                };

                if already_cached {
                    widgets.importance.computing = false;
                } else {
                    let ctx = app_state.current_study.as_ref().unwrap();
                    // 共有ストアの DataFrame を Arc::clone して直接利用（trial_rows 再構築不要）
                    let df = std::sync::Arc::clone(&ctx.view.df);
                    let tx = tx.clone();
                    match metric {
                        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => {
                            crate::app::spawn_task(tx, move || {
                                match tunny_core::sensitivity::compute_sobol_from_df(&df, 1024) {
                                    Some(r) => AppMessage::SobolDone {
                                        obj_idx,
                                        result: SobolResult {
                                            param_names: r.param_names,
                                            objective_names: r.objective_names,
                                            first_order: r.first_order,
                                            total_effect: r.total_effect,
                                            r_squared: r.r_squared,
                                        },
                                    },
                                    None => AppMessage::SensitivityError(
                                        "Sobol computation failed".into(),
                                    ),
                                }
                            });
                        }
                        _ => {
                            use tunny_core::sensitivity::{
                                MdiMetric, PermutationMetric, RfAnovaMetric, RidgeMetric,
                                ShapMetric, SpearmanMetric,
                            };
                            let core_metric: Box<dyn tunny_core::sensitivity::SensitivityMetric> =
                                match metric {
                                    ImportanceMetric::Spearman => Box::new(SpearmanMetric),
                                    ImportanceMetric::Ridge => Box::new(RidgeMetric),
                                    ImportanceMetric::RfAnova => Box::new(RfAnovaMetric),
                                    ImportanceMetric::Mdi => Box::new(MdiMetric),
                                    ImportanceMetric::Shap => Box::new(ShapMetric),
                                    ImportanceMetric::Permutation => Box::new(PermutationMetric),
                                    _ => unreachable!(),
                                };
                            let key = (metric.cache_id(), obj_idx);
                            crate::app::spawn_task(tx, move || {
                                let mut results =
                                    tunny_core::sensitivity::compute_sensitivity_single_obj(
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
                                    vec![(0..n_params).map(|pi| r.spearman[pi][0]).collect()]
                                } else {
                                    vec![]
                                };
                                AppMessage::SensitivityDone {
                                    key,
                                    result: SensitivityResult {
                                        param_names: r.param_names,
                                        objective_names: r.objective_names,
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
                                    },
                                }
                            });
                        }
                    }
                }
            }
        }
        ChartId::PdpChart => {
            let Some(req) = widgets.pdp_chart.pending_compute.take() else {
                return;
            };
            // current_study is guaranteed Some by the early return at the top of this function
            let ctx = app_state.current_study.as_ref().unwrap();
            let Some(target_param_idx) = ctx.meta.param_names.iter().position(|p| p == &req.param)
            else {
                return;
            };
            let (x_matrix, y) = build_xy_for_objective(ctx, &req.objective);
            let param_names_owned = ctx.meta.param_names.clone();
            let (param, objective, model_type) = (req.param, req.objective, req.model_type);
            let n_grid = req.n_grid;
            widgets.pdp_chart.computing = true;
            let tx = tx.clone();
            crate::app::spawn_task(tx, move || {
                use crate::state::messages::{PdpResult, PdpResult1d};
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
                    result: PdpResult::OneDim(PdpResult1d {
                        x_values: r.grid,
                        y_values: r.values,
                        y_upper: r.y_upper,
                        y_lower: r.y_lower,
                        ice_lines: vec![],
                        r2: Some(r.r_squared),
                        param_name: r.param_name,
                        objective_name: r.objective_name,
                    }),
                }
            });
        }
        ChartId::PdpChart2D => {
            if let Some(req) = widgets.pdp_2d.pending_compute.take() {
                widgets.pdp_2d.computing = true;
                let tx = tx.clone();
                crate::app::spawn_task(tx, move || {
                    let result = tunny_core::pdp::compute_pdp_2d(
                        &req.param1,
                        &req.param2,
                        &req.objective,
                        req.n_grid,
                        &req.model_type,
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
        }
        ChartId::ClusterScatter => {
            if let Some(req) = widgets.cluster_scatter.pending_compute.take() {
                match build_cluster_matrix(&ctx.view, param_names, obj_names, req.target_space) {
                    Ok(matrix) => {
                        let tx = tx.clone();
                        app_state.cluster_result = None;
                        crate::app::spawn_task(tx, move || run_cluster_compute(req, matrix));
                    }
                    Err(err) => {
                        widgets.cluster_scatter.set_error(err);
                    }
                }
            }
        }
        ChartId::McdmRankChart => {
            if let Some(cached) = widgets.mcdm_chart.pending_restore.take() {
                app_state.mcdm_result = Some(cached);
            }

            if widgets.mcdm_chart.pending_entropy && !widgets.mcdm_chart.computing {
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

                if n_trials > 0 && n_objectives > 0 {
                    widgets.mcdm_chart.computing = true;
                    let tx = tx.clone();
                    crate::app::spawn_task(tx, move || {
                        match tunny_core::entropy::compute_entropy_weights(
                            &objectives,
                            n_trials,
                            n_objectives,
                        ) {
                            Ok(r) => AppMessage::EntropyDone(EntropyResult {
                                weights: r.weights,
                                entropies: r.entropies,
                                diversities: r.diversities,
                                duration_ms: r.duration_ms,
                            }),
                            Err(e) => {
                                AppMessage::Error(format!("Entropy computation failed: {}", e))
                            }
                        }
                    });
                }
            }

            if let Some(req) = widgets.mcdm_chart.pending_compute.take() {
                widgets.mcdm_chart.computing = true;

                let McdmComputeRequest { method, weights, v } = req;

                let n_trials = ctx.view.row_count();
                let n_objectives = obj_names.len();
                let obj_cols_mcdm = ctx.view.numeric_columns(obj_names);
                let objectives: Vec<f64> = (0..n_trials)
                    .flat_map(|i| {
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
                    let start = std::time::Instant::now();

                    match method {
                        McdmMethod::Topsis => {
                            match tunny_core::topsis::compute_topsis(
                                &objectives,
                                n_trials,
                                n_objectives,
                                &weights,
                                &is_minimize,
                            ) {
                                Ok(r) => {
                                    AppMessage::McdmDone(crate::state::results::McdmResult::Topsis(
                                        crate::state::results::TopsisResult {
                                            scores: r.scores,
                                            ranked_indices: r.ranked_indices,
                                            positive_ideal: r.positive_ideal,
                                            negative_ideal: r.negative_ideal,
                                            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                                        },
                                    ))
                                }
                                Err(e) => {
                                    AppMessage::Error(format!("TOPSIS computation failed: {}", e))
                                }
                            }
                        }
                        McdmMethod::Vikor => {
                            match tunny_core::vikor::compute_vikor(
                                &objectives,
                                n_trials,
                                n_objectives,
                                &weights,
                                &is_minimize,
                                v,
                            ) {
                                Ok(r) => {
                                    AppMessage::McdmDone(crate::state::results::McdmResult::Vikor(
                                        crate::state::results::VikorResult {
                                            s_values: r.s_values,
                                            r_values: r.r_values,
                                            q_values: r.q_values,
                                            display_scores: r.display_scores,
                                            ranked_indices: r.ranked_indices,
                                            best_values: r.best_values,
                                            worst_values: r.worst_values,
                                            duration_ms: start.elapsed().as_secs_f64() * 1000.0,
                                        },
                                    ))
                                }
                                Err(e) => {
                                    AppMessage::Error(format!("VIKOR computation failed: {}", e))
                                }
                            }
                        }
                        McdmMethod::PrometheeI | McdmMethod::PrometheeII => {
                            match tunny_core::promethee::compute_promethee(
                                &objectives,
                                n_trials,
                                n_objectives,
                                &weights,
                                &is_minimize,
                            ) {
                                Ok(r) => {
                                    let result = crate::state::results::PrometheeResult {
                                        phi_plus: r.phi_plus,
                                        phi_minus: r.phi_minus,
                                        phi_net: r.phi_net,
                                        ranked_indices_i: r.ranked_indices_i,
                                        ranked_indices_ii: r.ranked_indices_ii,
                                        duration_ms: r.duration_ms,
                                    };
                                    let mcdm = if method == McdmMethod::PrometheeI {
                                        crate::state::results::McdmResult::PrometheeI(result)
                                    } else {
                                        crate::state::results::McdmResult::PrometheeII(result)
                                    };
                                    AppMessage::McdmDone(mcdm)
                                }
                                Err(e) => {
                                    AppMessage::Error(format!("PROMETHEE computation failed: {e}"))
                                }
                            }
                        }
                    }
                });
            }
        }
        ChartId::AhpRankChart => {
            if let Some(req) = widgets.ahp_chart.pending_compute.take() {
                widgets.ahp_chart.computing = true;
                let tx = tx.clone();
                crate::app::spawn_task(tx, move || {
                    match tunny_core::ahp::compute_ahp(
                        &req.objectives,
                        req.n_trials,
                        req.n_objectives,
                        &req.pairwise_matrix,
                        &req.is_minimize,
                    ) {
                        Ok(r) => AppMessage::AhpDone(AhpResult {
                            priority_vector: r.priority_vector,
                            scores: r.scores,
                            ranked_indices: r.ranked_indices,
                            lambda_max: r.lambda_max,
                            ci: r.ci,
                            ri: r.ri,
                            cr: r.cr,
                            is_consistent: r.is_consistent,
                            duration_ms: r.duration_ms,
                        }),
                        Err(e) => AppMessage::Error(format!("AHP computation failed: {}", e)),
                    }
                });
            }
        }
        ChartId::SurfacePlot => {
            if let Some(req) = widgets.surface_plot.pending_compute.take() {
                let ctx = app_state.current_study.as_ref().unwrap();
                let Some(px_idx) = ctx.meta.param_names.iter().position(|p| p == &req.param_x)
                else {
                    widgets.surface_plot.error_message =
                        Some(format!("Parameter '{}' not found", req.param_x));
                    return;
                };
                let Some(py_idx) = ctx.meta.param_names.iter().position(|p| p == &req.param_y)
                else {
                    widgets.surface_plot.error_message =
                        Some(format!("Parameter '{}' not found", req.param_y));
                    return;
                };
                let (x_matrix, y) = build_xy_for_objective(ctx, &req.objective);
                let param_names_owned = ctx.meta.param_names.clone();
                let (param_x, param_y, objective, n_grid) = (
                    req.param_x.clone(),
                    req.param_y.clone(),
                    req.objective.clone(),
                    req.n_grid,
                );
                widgets.surface_plot.computing = true;
                let tx = tx.clone();
                crate::app::spawn_task(tx, move || {
                    use crate::state::messages::SurfacePlotResult;
                    let r = tunny_core::pdp::compute_surface_from_data(
                        x_matrix,
                        y,
                        param_names_owned,
                        &objective,
                        px_idx,
                        py_idx,
                        n_grid,
                        "ridge",
                    );
                    AppMessage::SurfacePlotDone(SurfacePlotResult {
                        x_values: r.x_values,
                        y_values: r.y_values,
                        z_values: r.z_values,
                        param_x_name: param_x,
                        param_y_name: param_y,
                        objective_name: objective,
                        r2: Some(r.r_squared),
                    })
                });
            }
        }
        _ => {}
    }
}

fn run_cluster_compute(req: ClusterComputeRequest, matrix: ClusterMatrix) -> AppMessage {
    let trial_count = matrix.n_rows;
    let n_cols = matrix.n_cols;

    if !matrix.is_valid_for_clustering() {
        return cluster_failed(
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
                trial_count.min(10),
            );
            elbow.recommended_k.clamp(2, trial_count)
        }
        KSelectionMode::Manual => req.k,
    };

    if selected_k < 2 || selected_k > trial_count {
        return cluster_failed(
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
            "Cluster result is inconsistent. Please run again.",
            Some(format!(
                "validation: labels_len({}) != trial_count({trial_count})",
                result.labels.len()
            )),
            true,
        );
    }

    AppMessage::ClusteringDone(crate::state::results::ClusterResult {
        labels: result.labels.into_iter().map(|v| v as i32).collect(),
        n_clusters: selected_k,
    })
}

fn cluster_failed(message: &str, detail: Option<String>, retryable: bool) -> AppMessage {
    AppMessage::ClusterFailed(crate::state::messages::cluster_ui_error(
        message, detail, retryable,
    ))
}
