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
            widgets.importance.show(
                ui,
                app_state.sensitivity_result.as_ref(),
                app_state.sobol_result.as_ref(),
                obj_names,
            );
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
            widgets
                .parallel_coords
                .show(ui, trial_rows, param_names, obj_names);
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
            widgets
                .sensitivity_heatmap
                .show(ui, app_state.sensitivity_result.as_ref());
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
