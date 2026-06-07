use std::sync::mpsc;

use crate::state::app_state::{AppState, Direction, StudyContext};
use crate::state::layout_state::ChartId;
use crate::state::messages::{AppMessage, ClusterChartSource, McdmChartSource};
use crate::state::results::{AhpResult, EntropyResult, McdmMethod, McdmResult};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::cluster_scatter::{
    build_cluster_matrix, ClusterCacheKey, ClusterComputeRequest, ClusterMatrix, KSelectionMode,
};
use crate::ui::widgets::mcdm_chart::{McdmCacheKey, McdmComputeRequest, McdmControls};

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
                        crate::app::spawn_task(tx, move || {
                            run_cluster_compute(ClusterChartSource::Scatter2D, req, matrix)
                        });
                    }
                    Err(err) => {
                        widgets.cluster_scatter.set_error(err);
                    }
                }
            }
        }
        ChartId::ClusterScatter3D => {
            if let Some(req) = widgets.cluster_scatter_3d.pending_compute.take() {
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
        }
        ChartId::ClusterTable => {
            if let Some(req) = widgets.cluster_table.pending_compute.take() {
                match build_cluster_matrix(&ctx.view, param_names, obj_names, req.target_space) {
                    Ok(matrix) => {
                        let tx = tx.clone();
                        crate::app::spawn_task(tx, move || {
                            run_cluster_compute(ClusterChartSource::Table, req, matrix)
                        });
                    }
                    Err(err) => {
                        widgets.cluster_table.set_error(err);
                    }
                }
            }
        }
        ChartId::McdmRankChart => {
            dispatch_mcdm_entropy(
                &mut widgets.mcdm_chart.controls,
                ctx,
                obj_names,
                McdmChartSource::Rank,
                tx,
            );
            dispatch_mcdm_compute(
                &mut widgets.mcdm_chart.controls,
                ctx,
                obj_names,
                directions,
                McdmChartSource::Rank,
                tx,
            );
        }
        ChartId::McdmScatterChart => {
            dispatch_mcdm_entropy(
                &mut widgets.scatter_chart.controls,
                ctx,
                obj_names,
                McdmChartSource::Scatter2D,
                tx,
            );
            dispatch_mcdm_compute(
                &mut widgets.scatter_chart.controls,
                ctx,
                obj_names,
                directions,
                McdmChartSource::Scatter2D,
                tx,
            );
        }
        ChartId::McdmScatterChart3D => {
            dispatch_mcdm_entropy(
                &mut widgets.mcdm_scatter_3d.controls,
                ctx,
                obj_names,
                McdmChartSource::Scatter3D,
                tx,
            );
            dispatch_mcdm_compute(
                &mut widgets.mcdm_scatter_3d.controls,
                ctx,
                obj_names,
                directions,
                McdmChartSource::Scatter3D,
                tx,
            );
        }
        ChartId::McdmTable => {
            dispatch_mcdm_entropy(
                &mut widgets.mcdm_table.controls,
                ctx,
                obj_names,
                McdmChartSource::Table,
                tx,
            );
            dispatch_mcdm_compute(
                &mut widgets.mcdm_table.controls,
                ctx,
                obj_names,
                directions,
                McdmChartSource::Table,
                tx,
            );
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

/// Entropy 重みの計算を必要なら起動する（チャートごとの controls から）。
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

/// MCDM ランキングの計算を必要なら起動する（チャートごとの controls から）。
/// 結果は設定キー付きで返し、`app_state.mcdm_cache` に格納される。
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

    // パレートフロント（rank == 0）の行インデックスのみを対象とする
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

/// パレートフロント部分集合に対して MCDM を計算し、全トライアル長へ展開した結果を返す。
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

    // subset 内のインデックスを全トライアルのインデックスに変換するヘルパー
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
                positive_ideal: r.positive_ideal,
                negative_ideal: r.negative_ideal,
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
                best_values: r.best_values,
                worst_values: r.worst_values,
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
    let trial_count = matrix.n_rows; // パレートフロントの解数（k-means に渡す行数）
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
                trial_count.min(10),
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

    // パレートフロントのラベルを全トライアル分に展開（対象外の解は -1）
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
