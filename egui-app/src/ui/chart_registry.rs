use std::sync::mpsc;

use crate::state::app_state::AppState;
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
    let Some(ctx) = app_state.current_study.as_ref() else {
        return;
    };
    let trial_rows = ctx.trial_rows.clone();
    let obj_names = ctx.meta.objective_names.clone();
    let param_names = ctx.meta.param_names.clone();
    let is_minimize = ctx
        .meta
        .directions
        .first()
        .map(|d| matches!(d, crate::state::app_state::Direction::Minimize))
        .unwrap_or(true);
    let sensitivity = app_state.sensitivity_result.clone();
    let hv_history = app_state.hv_history.clone();

    match chart_id {
        ChartId::ParetoScatter2D => {
            widgets.pareto_2d.show(ui, app_state);
        }
        ChartId::OptimizationHistory => {
            widgets.opt_history.show(ui, &trial_rows, is_minimize);
        }
        ChartId::HvHistory => {
            widgets.hv_history.hv_history = hv_history;
            widgets.hv_history.show(ui);
        }
        ChartId::ImportanceChart => {
            let sobol = app_state.sobol_result.as_ref();
            widgets
                .importance
                .show(ui, sensitivity.as_ref(), sobol, &obj_names);
        }
        ChartId::PdpChart => {
            widgets
                .pdp_chart
                .show(ui, &param_names, &obj_names, &trial_rows);
        }
        ChartId::PdpChart2D => {
            let cmap = app_state.selected_colormap.to_colormap();
            widgets.pdp_2d.show(ui, &param_names, &obj_names, cmap);
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
                .show(ui, &trial_rows, &param_names, &obj_names);
        }
        ChartId::ScatterMatrix => {
            widgets.scatter_matrix.show(
                ui,
                &trial_rows,
                &param_names,
                &obj_names,
                &app_state.chart_colors,
            );
        }
        ChartId::ParetoScatter3D => {
            ui.label("3D Pareto chart requires GPU rendering (not yet wired up).");
        }
        ChartId::SensitivityHeatmap => {
            widgets.sensitivity_heatmap.show(ui, sensitivity.as_ref());
        }
        ChartId::ClusterScatter => {
            widgets.cluster_scatter.show(
                ui,
                &trial_rows,
                app_state.cluster_result.as_ref(),
                &param_names,
                &app_state.chart_colors,
            );
        }
    }
}
