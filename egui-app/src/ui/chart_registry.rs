use std::sync::mpsc;

use crate::state::app_state::{AppState, Direction};
use crate::state::layout_state::ChartId;
use crate::state::messages::AppMessage;
use crate::state::results::{EntropyResult, McdmMethod};
use crate::ui::widget_states::WidgetStates;
use crate::ui::widgets::mcdm_chart::McdmComputeRequest;

/// タイトルと区切り線付きでチャートを描画する
pub fn show_cell_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    ui.label(egui::RichText::new(chart_id.label()).strong());
    ui.separator();
    show_chart(ui, app_state, widgets, chart_id, tx);
}

/// ChartId に対応するチャートウィジェットを描画する
pub fn show_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
    tx: &mpsc::SyncSender<AppMessage>,
) {
    if app_state.current_study.is_none() {
        return;
    }

    // &mut AppState が必要なウィジェットは ctx を借用する前に処理する
    if matches!(chart_id, ChartId::ParetoScatter2D) {
        widgets.pareto_2d.show(ui, app_state);
        return;
    }
    if matches!(chart_id, ChartId::ParetoScatter3D) {
        widgets.pareto_3d.show(ui, app_state);
        return;
    }

    // 以降は不変参照のみで足りるため、クローンせずに参照を使う
    let ctx = app_state.current_study.as_ref().unwrap();
    let trial_rows = &ctx.trial_rows;
    let obj_names = &ctx.meta.objective_names;
    let param_names = &ctx.meta.param_names;
    let directions = &ctx.meta.directions;

    match chart_id {
        ChartId::ParetoScatter2D => unreachable!(),
        ChartId::OptimizationHistory => {
            widgets
                .opt_history
                .show(ui, trial_rows, obj_names, directions);
        }
        ChartId::HvHistory => {
            // 未計算かつ計算中でない場合にバックグラウンドで HV 計算を起動する
            if app_state.hv_history.is_none() && !widgets.hv_history.computing {
                let is_minimize: Vec<bool> = directions
                    .iter()
                    .map(|d| matches!(d, Direction::Minimize))
                    .collect();

                // データを main スレッドで抽出（with_active_df はスレッドローカルのため）
                // 約 50 点に 1 点の間隔でダウンサンプリングして計算コストを削減
                const TARGET_POINTS: usize = 50;
                let step = (trial_rows.len() / TARGET_POINTS).max(1);
                let (sampled_ids, sampled_objs): (Vec<u32>, Vec<Vec<f64>>) = trial_rows
                    .iter()
                    .step_by(step)
                    .map(|r| (r.trial_id, r.objectives.clone()))
                    .unzip();

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
            widgets.hv_history.hv_history = app_state.hv_history.clone();
            widgets.hv_history.show(ui);
        }
        ChartId::ImportanceChart => {
            let imp_key = (
                widgets.importance.metric.cache_id(),
                widgets.importance.objective_index,
            );
            let current_sensitivity = app_state.importance_cache.get(&imp_key);
            let current_sobol = app_state
                .sobol_cache
                .get(&widgets.importance.objective_index);
            widgets
                .importance
                .show(ui, current_sensitivity, current_sobol, obj_names);

            if let Some((metric, obj_idx)) = widgets.importance.pending_compute.take() {
                use crate::state::results::{
                    MdiResult, RfAnovaResult, RidgeResult, SensitivityResult, ShapResult,
                    SobolResult,
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
                    // thread_local の GLOBAL_STATE はスレッドをまたいで共有されないため、
                    // メインスレッドで app_state の trial_rows から DataFrame を再構築して渡す
                    let ctx = app_state.current_study.as_ref().unwrap();
                    let selected_obj = ctx
                        .meta
                        .objective_names
                        .get(obj_idx)
                        .cloned()
                        .unwrap_or_default();
                    let core_rows: Vec<tunny_core::dataframe::TrialRow> = ctx
                        .trial_rows
                        .iter()
                        .map(|r| tunny_core::dataframe::TrialRow {
                            trial_id: r.trial_id,
                            param_display: r.params.clone(),
                            param_category_label: Default::default(),
                            objective_values: r.objectives.clone(),
                            user_attrs_numeric: Default::default(),
                            user_attrs_string: Default::default(),
                            constraint_values: vec![],
                        })
                        .collect();
                    let df = tunny_core::dataframe::DataFrame::from_trials(
                        &core_rows,
                        &ctx.meta.param_names,
                        &[selected_obj],
                        &[],
                        &[],
                        0,
                    );
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
                            let core_metric = match metric {
                                ImportanceMetric::Spearman => {
                                    tunny_core::sensitivity::SensitivityMetric::Spearman
                                }
                                ImportanceMetric::Ridge => {
                                    tunny_core::sensitivity::SensitivityMetric::Ridge
                                }
                                ImportanceMetric::RfAnova => {
                                    tunny_core::sensitivity::SensitivityMetric::RfAnova
                                }
                                ImportanceMetric::Mdi => {
                                    tunny_core::sensitivity::SensitivityMetric::Mdi
                                }
                                ImportanceMetric::Shap => {
                                    tunny_core::sensitivity::SensitivityMetric::Shap
                                }
                                _ => unreachable!(),
                            };
                            let key = (metric.cache_id(), obj_idx);
                            crate::app::spawn_task(tx, move || {
                                let r = tunny_core::sensitivity::compute_sensitivity_single_obj(
                                    &df,
                                    &core_metric,
                                    0,
                                );
                                // tunny_core は spearman[param][obj] だが egui-app 側は [obj][param] を期待する
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
                                            importances: x.importances,
                                            r_squared: x.r_squared,
                                        }),
                                        mdi: r.mdi.map(|x| MdiResult {
                                            importances: x.importances,
                                            r_squared: x.r_squared,
                                        }),
                                        shap: r.shap.map(|x| ShapResult {
                                            importances: x.importances,
                                            r_squared: x.r_squared,
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
            widgets
                .pdp_chart
                .show(ui, param_names, obj_names, trial_rows);
            if let Some(req) = widgets.pdp_chart.pending_compute.take() {
                // with_active_df はスレッドローカルなので、データをメインスレッドで抽出する
                if let Some(ctx) = &app_state.current_study {
                    let target_idx = ctx.meta.param_names.iter().position(|p| p == &req.param);
                    if let Some(target_param_idx) = target_idx {
                        let x_matrix: Vec<Vec<f64>> = ctx
                            .trial_rows
                            .iter()
                            .map(|r| {
                                ctx.meta
                                    .param_names
                                    .iter()
                                    .map(|p| r.params.get(p).copied().unwrap_or(0.0))
                                    .collect()
                            })
                            .collect();
                        let y: Vec<f64> = ctx
                            .trial_rows
                            .iter()
                            .map(|r| {
                                ctx.meta
                                    .objective_names
                                    .iter()
                                    .position(|o| o == &req.objective)
                                    .and_then(|i| r.objectives.get(i).copied())
                                    .unwrap_or(0.0)
                            })
                            .collect();
                        let param_names_owned = ctx.meta.param_names.clone();
                        let objective_name_owned = req.objective.clone();
                        let n_grid = req.n_grid;
                        let model_type_owned = req.model_type.clone();
                        let param_for_msg = req.param.clone();
                        let objective_for_msg = req.objective.clone();
                        widgets.pdp_chart.computing = true;
                        let tx = tx.clone();
                        crate::app::spawn_task(tx, move || {
                            use crate::state::messages::{PdpResult, PdpResult1d};
                            let r = tunny_core::pdp::compute_pdp_from_data(
                                x_matrix,
                                y,
                                param_names_owned,
                                &objective_name_owned,
                                target_param_idx,
                                n_grid,
                                &model_type_owned,
                            );
                            AppMessage::PdpDone {
                                param: param_for_msg,
                                objective: objective_for_msg,
                                model_type: model_type_owned.clone(),
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
                }
            }
        }
        ChartId::PdpChart2D => {
            let cmap = app_state.selected_colormap.to_colormap();
            widgets.pdp_2d.show(ui, param_names, obj_names, cmap);
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
        ChartId::ParallelCoordinates => {
            widgets.parallel_coords.show(
                ui,
                trial_rows,
                param_names,
                obj_names,
                &app_state.chart_colors,
            );
        }
        ChartId::ScatterMatrix => {
            widgets.scatter_matrix.show(
                ui,
                trial_rows,
                param_names,
                obj_names,
                &app_state.chart_colors,
            );
        }
        ChartId::ParetoScatter3D => unreachable!(),
        ChartId::SensitivityHeatmap => {
            widgets.sensitivity_heatmap.show(ui);
        }
        ChartId::ClusterScatter => {
            widgets.cluster_scatter.show(
                ui,
                trial_rows,
                app_state.cluster_result.as_ref(),
                param_names,
                &app_state.chart_colors,
            );
        }
        ChartId::McdmRankChart => {
            widgets
                .mcdm_chart
                .show(ui, obj_names, &app_state.mcdm_result, trial_rows);

            // メソッド切替時のキャッシュ復元
            if let Some(cached) = widgets.mcdm_chart.pending_restore.take() {
                app_state.mcdm_result = Some(cached);
            }

            // Entropy dispatch: pending_entropy が true の場合、バックグラウンドでエントロピー計算を実行
            if widgets.mcdm_chart.pending_entropy && !widgets.mcdm_chart.computing {
                let objectives: Vec<f64> = trial_rows
                    .iter()
                    .flat_map(|r| r.objectives.iter().copied())
                    .collect();
                let n_trials = trial_rows.len();
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

                let objectives: Vec<f64> = trial_rows
                    .iter()
                    .flat_map(|r| r.objectives.iter().copied())
                    .collect();
                let n_trials = trial_rows.len();
                let n_objectives = obj_names.len();
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
                    }
                });
            }
        }
        ChartId::McdmTable => {
            widgets
                .mcdm_table
                .show(ui, &app_state.mcdm_result, trial_rows, obj_names);
        }
        ChartId::SliceChart => {
            widgets
                .slice_chart
                .show(ui, trial_rows, param_names, obj_names, directions);
        }
    }
}
