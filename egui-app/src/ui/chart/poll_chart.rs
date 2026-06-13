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

fn build_xy_for_objective(
    ctx: &crate::state::app_state::StudyContext,
    objective: &str,
    feasible_only: bool,
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let param_names = &ctx.meta.param_names;
    let n = ctx.view.row_count();

    let param_cols = ctx.view.numeric_columns(param_names);
    let obj_col = ctx.view.numeric_column(objective);
    // 実行可能解フィルタ。is_feasible 列が無い（制約なし）場合は全行を対象とする。
    let feas = ctx.view.feasibility();

    let mut x_matrix: Vec<Vec<f64>> = Vec::with_capacity(n);
    let mut y: Vec<f64> = Vec::with_capacity(n);
    for i in 0..n {
        if feasible_only && !feas.is_feasible(i) {
            continue;
        }
        x_matrix.push(
            param_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                .collect(),
        );
        y.push(obj_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0));
    }

    (x_matrix, y)
}

/// 感度分析用の DataFrame を返す。feasible_only の場合は実行可能解のみの
/// コピーを作る（コア関数は DataFrame を直接受け取るため）。
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

/// 選択手法について、全パラメータ × 全目的の感度行列 `values[param][obj]` を計算する。
/// Sobol（First/Total）は一度の全目的計算から指数を取り出し、その他は目的ごとに
/// 単一目的メトリクスを評価して列を埋める。手法→コアメトリクスの対応は
/// `core_sensitivity_metric` を ImportanceChart と共有する。
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
        // first_order / total_effect はともに [param][obj] 形状で全目的を一括で返す。
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

/// 単一目的の計算結果（コア `SensitivityResult`）から、指定パラメータのスコアを取り出す。
/// 木ベース（RF-Anova/MDI/SHAP/Permutation）は `importances[param][0]`、Spearman は
/// `spearman[param][0]`、Ridge は `ridge[0].beta[param]`。Sobol はこの経路を通らない。
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
        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => 0.0,
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

                // ユーザー指定の参照点（元の目的値）を正規化空間へ変換して渡す。
                // 次元が目的数と一致しない指定は無視（None 扱い）して自動算出に委ねる。
                let ref_override_norm: Option<Vec<f64>> = app_state
                    .hv_ref_point_override
                    .as_ref()
                    .filter(|r| r.len() == obj_names.len())
                    .map(|r| crate::state::ref_point_to_normalized(r, &is_minimize));
                let is_minimize_for_back = is_minimize.clone();

                widgets.hv_history.computing = true;
                let tx = tx.clone();
                crate::app::spawn_task(tx, move || {
                    let result = tunny_core::pareto::compute_hv_history_with_ref(
                        &sampled_ids,
                        &sampled_objs,
                        &is_minimize,
                        ref_override_norm.as_deref(),
                    );
                    AppMessage::HvHistoryDone {
                        trial_ids: result.trial_ids,
                        hv_values: result.hv_values,
                        sample_step: step,
                        // 表示用に参照点を元の目的値の単位へ戻す。
                        ref_point: crate::state::ref_point_to_original(
                            &result.ref_point,
                            &is_minimize_for_back,
                        ),
                    }
                });
            }
        }
        ChartId::ImportanceChart => {
            if let Some((metric, obj_idx, feasible_only)) =
                widgets.importance.pending_compute.take()
            {
                use crate::state::results::{
                    MdiResult, PermutationResult, RfAnovaResult, RidgeResult, SensitivityResult,
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
                    app_state.importance_cache.contains_key(&(
                        metric.cache_id(),
                        obj_idx,
                        feasible_only,
                    ))
                };

                if already_cached {
                    widgets.importance.computing = false;
                } else {
                    let ctx = app_state.current_study.as_ref().unwrap();
                    // 共有ストアの DataFrame を Arc::clone して直接利用（trial_rows 再構築不要）。
                    // feasible_only の場合は実行可能解のみのコピーを使う。
                    let df = sensitivity_df(ctx, feasible_only);
                    let tx = tx.clone();
                    match metric {
                        ImportanceMetric::SobolFirst | ImportanceMetric::SobolTotal => {
                            crate::app::spawn_task(tx, move || {
                                match tunny_core::sensitivity::compute_sobol_from_df(
                                    &df,
                                    SOBOL_SAMPLE_COUNT,
                                ) {
                                    Some(r) => AppMessage::SobolDone {
                                        key: (obj_idx, feasible_only),
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
                            let Some(core_metric) = core_sensitivity_metric(metric) else {
                                return;
                            };
                            let key = (metric.cache_id(), obj_idx, feasible_only);
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
        ChartId::SensitivityHeatmap => {
            // 選択手法の全パラメータ × 全目的の感度行列を非同期計算する。
            // 計算要求は widgets.sensitivity_heatmap.pending_compute に積まれ
            // （Run ボタン、または低コスト手法の自動トリガー）、結果は手法ごとに
            // app_state.sensitivity_heatmap_cache へ集約される。
            if let Some((metric, feasible_only)) =
                widgets.sensitivity_heatmap.pending_compute.take()
            {
                if app_state
                    .sensitivity_heatmap_cache
                    .contains_key(&(metric.cache_id(), feasible_only))
                {
                    widgets.sensitivity_heatmap.computing = false;
                } else {
                    let ctx = app_state.current_study.as_ref().unwrap();
                    let df = sensitivity_df(ctx, feasible_only);
                    widgets.sensitivity_heatmap.computing = true;
                    let tx = tx.clone();
                    crate::app::spawn_task(tx, move || {
                        compute_sensitivity_heatmap(metric, feasible_only, &df)
                    });
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
            let (x_matrix, y) = build_xy_for_objective(ctx, &req.objective, req.feasible_only);
            let param_names_owned = ctx.meta.param_names.clone();
            let (param, objective, model_type) = (req.param, req.objective, req.model_type);
            let (n_grid, feasible_only) = (req.n_grid, req.feasible_only);
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
                    feasible_only,
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
        ChartId::McdmRankChart | ChartId::McdmScatterChart | ChartId::McdmScatterChart3D => {
            // 各 MCDM チャートは独自の controls を持つが、ディスパッチ処理は共通。
            // 対象チャートの controls と source だけを選び、同じ 2 ステップを実行する。
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
        ChartId::ArtifactGallery => {
            use crate::ui::widgets::artifact_gallery::ArtifactViewMode;
            match widgets.artifact_gallery.mode {
                ArtifactViewMode::Cluster => {
                    if let Some(req) = widgets.artifact_gallery.cluster_pending.take() {
                        match build_cluster_matrix(
                            &ctx.view,
                            param_names,
                            obj_names,
                            req.target_space,
                        ) {
                            Ok(matrix) => {
                                let tx = tx.clone();
                                crate::app::spawn_task(tx, move || {
                                    run_cluster_compute(
                                        ClusterChartSource::ArtifactGallery,
                                        req,
                                        matrix,
                                    )
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
                let (x_matrix, y) = build_xy_for_objective(ctx, &req.objective, req.feasible_only);
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
        ChartId::SurrogateOpt => {
            // フィット段階を最優先で処理する（optimize より先に take する）。
            if let Some(fit_req) = widgets.surrogate_opt.pending_fit.take() {
                let ctx = app_state.current_study.as_ref().unwrap();
                // カテゴリカル列を除いた数値パラメータのみで X 行列を作る
                //（render_chart 側のコンボに出す一覧と同じ絞り込み）。
                let numeric_params: Vec<String> = ctx
                    .meta
                    .param_names
                    .iter()
                    .filter(|p| ctx.view.numeric_column(p).is_some())
                    .cloned()
                    .collect();
                if numeric_params.is_empty() {
                    widgets.surrogate_opt.error_message =
                        Some("No numeric parameters available".to_string());
                    return;
                }
                let n = ctx.view.row_count();
                let param_cols = ctx.view.numeric_columns(&numeric_params);
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
                    .numeric_column(&fit_req.objective)
                    .map(|col| col.to_vec())
                    .unwrap_or_else(|| vec![0.0; n]);

                // フィット開始前に前の学習結果・最適化結果をクリアする。
                widgets.surrogate_opt.fitting = true;
                widgets.surrogate_opt.trained = None;
                widgets.surrogate_opt.result = None;
                widgets.surrogate_opt.error_message = None;

                // 制約列を抽出する（use_constraints かつ制約列がある場合）。
                let constraints: Vec<tunny_core::surrogate_opt::ConstraintData> =
                    if fit_req.use_constraints {
                        ctx.view
                            .df
                            .constraint_col_names()
                            .iter()
                            .filter_map(|col_name| {
                                ctx.view.df.get_numeric_column(col_name).map(|col| {
                                    tunny_core::surrogate_opt::ConstraintData {
                                        name: col_name.clone(),
                                        values: col.to_vec(),
                                    }
                                })
                            })
                            .collect()
                    } else {
                        vec![]
                    };

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
                    };
                    match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_core_req) {
                        Ok(t) => AppMessage::SurrogateFitDone(std::sync::Arc::new(t)),
                        Err(e) => AppMessage::SurrogateFitFailed(e),
                    }
                });
            } else if let Some(multi_fit_req) = widgets.surrogate_opt.pending_multi_fit.take() {
                // 多目的フィット段階: 全目的に対してサロゲートを学習する。
                let ctx = app_state.current_study.as_ref().unwrap();
                let numeric_params: Vec<String> = ctx
                    .meta
                    .param_names
                    .iter()
                    .filter(|p| ctx.view.numeric_column(p).is_some())
                    .cloned()
                    .collect();
                if numeric_params.is_empty() {
                    widgets.surrogate_opt.error_message =
                        Some("No numeric parameters available".to_string());
                    return;
                }
                let n = ctx.view.row_count();
                let param_cols = ctx.view.numeric_columns(&numeric_params);
                let x_matrix: Vec<Vec<f64>> = (0..n)
                    .map(|i| {
                        param_cols
                            .iter()
                            .map(|col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                            .collect()
                    })
                    .collect();
                // 全目的の y 列を収集する。
                let ys: Vec<(String, Vec<f64>)> = obj_names
                    .iter()
                    .map(|name| {
                        let col = ctx
                            .view
                            .numeric_column(name)
                            .map(|c| c.to_vec())
                            .unwrap_or_else(|| vec![0.0; n]);
                        (name.clone(), col)
                    })
                    .collect();

                // フィット開始前に前の多目的結果をクリアする。
                widgets.surrogate_opt.fitting = true;
                widgets.surrogate_opt.multi_trained = None;
                widgets.surrogate_opt.multi_result = None;
                widgets.surrogate_opt.error_message = None;

                let tx = tx.clone();
                crate::app::spawn_task(tx, move || {
                    let mut trained_vec = Vec::with_capacity(ys.len());
                    for (obj_name, y) in ys {
                        let fit_req = tunny_core::surrogate_opt::SurrogateFitRequest {
                            x_matrix: x_matrix.clone(),
                            y,
                            param_names: numeric_params.clone(),
                            objective_name: obj_name.clone(),
                            model: multi_fit_req.model,
                            auto_select: false,  // 多目的は Auto 非対応
                            constraints: vec![], // 多目的は制約対象外
                        };
                        match tunny_core::surrogate_opt::fit_surrogate_with_validation(&fit_req) {
                            Ok(t) => trained_vec.push(t),
                            Err(e) => {
                                return AppMessage::SurrogateMultiFitFailed(format!(
                                    "Fitting failed for objective '{}': {}",
                                    obj_name, e
                                ));
                            }
                        }
                    }
                    AppMessage::SurrogateMultiFitDone(std::sync::Arc::new(trained_vec))
                });
            } else if let Some(opt_req) = widgets.surrogate_opt.pending_optimize.take() {
                // 最適化段階は学習済みモデルが必要。
                let Some(trained) = widgets.surrogate_opt.trained.clone() else {
                    widgets.surrogate_opt.error_message =
                        Some("No trained model available. Run Fit & Validate first.".to_string());
                    return;
                };

                let obj_name = trained.objective_name.clone();
                let obj_idx = obj_names.iter().position(|o| *o == obj_name);
                let minimize = obj_idx
                    .and_then(|i| directions.get(i))
                    .map(|d| matches!(d, Direction::Minimize))
                    .unwrap_or(true);

                // スライス軸インデックスは訓練済みモデルの param_names から解決する。
                let slice_params = trained
                    .param_names
                    .iter()
                    .position(|p| p == &opt_req.slice_x)
                    .zip(
                        trained
                            .param_names
                            .iter()
                            .position(|p| p == &opt_req.slice_y),
                    )
                    .filter(|(a, b)| a != b);

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
                        slice: r.slice,
                        best_observed_value: r.best_observed_value,
                        predicted_constraints,
                        feasibility_probability: r.feasibility_probability,
                    })
                });
            } else if let Some(multi_opt_req) = widgets.surrogate_opt.pending_multi_optimize.take()
            {
                // 多目的最適化段階: 学習済みサロゲート群が必要。
                let Some(multi_trained) = widgets.surrogate_opt.multi_trained.clone() else {
                    widgets.surrogate_opt.error_message = Some(
                        "No trained multi-objective model. Run Fit & Validate first.".to_string(),
                    );
                    return;
                };

                // 目的ごとの minimize フラグを directions から解決する。
                let minimize_flags: Vec<bool> = (0..obj_names.len())
                    .map(|i| {
                        directions
                            .get(i)
                            .map(|d| matches!(d, Direction::Minimize))
                            .unwrap_or(true)
                    })
                    .collect();

                // スライス軸インデックスは trained[0].param_names から解決する。
                let first_param_names = multi_trained
                    .first()
                    .map(|t| t.param_names.clone())
                    .unwrap_or_default();
                let slice_params = first_param_names
                    .iter()
                    .position(|p| p == &multi_opt_req.slice_x)
                    .zip(
                        first_param_names
                            .iter()
                            .position(|p| p == &multi_opt_req.slice_y),
                    )
                    .filter(|(a, b)| a != b);

                let objective_names_owned = obj_names.to_vec();
                widgets.surrogate_opt.optimizing = true;
                let tx = tx.clone();
                crate::app::spawn_task(tx, move || {
                    use crate::state::messages::SurrogateMultiOptUiResult;
                    let refs: Vec<&tunny_core::surrogate_opt::TrainedSurrogate> =
                        multi_trained.iter().collect();
                    let spec = tunny_core::surrogate_opt::SurrogateMultiOptimizeSpec {
                        minimize: minimize_flags.clone(),
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
                                minimize: minimize_flags,
                                front: r.front,
                                r_squared: r.r_squared,
                                slices: r.slices,
                            })
                        }
                        Err(e) => AppMessage::SurrogateMultiOptFailed(e),
                    }
                });
            } else if let Some(suggest_req) = widgets.surrogate_opt.pending_suggest.take() {
                // 候補提案段階: 学習済み GP サロゲートが必要。
                let Some(trained) = widgets.surrogate_opt.trained.clone() else {
                    widgets.surrogate_opt.error_message =
                        Some("No trained model available. Run Fit & Validate first.".to_string());
                    return;
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
                        Ok(candidates) => {
                            AppMessage::SurrogateSuggestDone(SurrogateSuggestUiResult {
                                candidates,
                                param_names,
                                objective_name,
                            })
                        }
                        Err(e) => AppMessage::SurrogateSuggestFailed(e),
                    }
                });
            } else if let Some(multi_suggest_req) =
                widgets.surrogate_opt.pending_multi_suggest.take()
            {
                // 多目的候補提案段階（EHVI）: 学習済み GP サロゲート群が必要。
                let Some(multi_trained) = widgets.surrogate_opt.multi_trained.clone() else {
                    widgets.surrogate_opt.error_message = Some(
                        "No trained multi-objective model. Run Fit & Validate first.".to_string(),
                    );
                    return;
                };

                // 目的ごとの minimize フラグを directions から解決する。
                let minimize_flags: Vec<bool> = (0..obj_names.len())
                    .map(|i| {
                        directions
                            .get(i)
                            .map(|d| matches!(d, Direction::Minimize))
                            .unwrap_or(true)
                    })
                    .collect();

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
            }
        }
        _ => {}
    }
}

/// 統合トライアルテーブル（`PanelItem::TrialTable`）の非同期計算をディスパッチする。
/// 現在のモードに応じて、Cluster なら クラスタリング、MCDM なら MCDM 計算を起動する。
/// 計算結果は Cluster/MCDM テーブルと同じ `ClusterChartSource::Table` /
/// `McdmChartSource::Table` で共有・キャッシュされる。
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
