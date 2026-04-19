use std::sync::mpsc;

use crate::state::app_state::{AppState, Direction};
use crate::state::layout_state::ChartId;
use crate::state::messages::AppMessage;
use crate::ui::widget_states::WidgetStates;

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

    // ParetoScatter2D は &mut AppState が必要なため、ctx を借用する前に処理する
    if matches!(chart_id, ChartId::ParetoScatter2D) {
        widgets.pareto_2d.show(ui, app_state);
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
                    MdiResult, RfAnovaResult, RidgeResult, SensitivityResult, SobolResult,
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
        ChartId::ParetoScatter3D => {
            ui.label("3D Pareto chart requires GPU rendering (not yet wired up).");
        }
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
    }
}
