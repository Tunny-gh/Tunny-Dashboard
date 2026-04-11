use crate::state::app_state::AppState;
use crate::state::layout_state::{ChartId, LayoutState};
use crate::ui::widget_states::WidgetStates;

/// メインキャンバスを描画する
/// visible_charts に応じて各チャートウィジェットをレンダリングする
pub fn show_main_canvas(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    layout: &mut LayoutState,
    widgets: &mut WidgetStates,
) {
    // スタディが未選択の場合はガイダンスを表示
    if app_state.current_study.is_none() {
        ui.centered_and_justified(|ui| {
            if app_state.all_studies.is_empty() {
                ui.label("Open a journal file to start.");
            } else {
                ui.label("Select a study from the toolbar.");
            }
        });
        return;
    }

    let visible: Vec<ChartId> = ChartId::all()
        .iter()
        .filter(|id| layout.is_chart_visible(id))
        .cloned()
        .collect();

    if visible.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("No charts selected. Enable charts from the left panel.");
        });
        return;
    }

    // チャートを縦スクロールエリアに積み上げ表示
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for chart_id in &visible {
                let label = chart_id.label();
                ui.collapsing(label, |ui| {
                    // 各チャートを固定高さで表示
                    ui.set_min_height(300.0);
                    show_chart(ui, app_state, widgets, chart_id);
                });
                ui.separator();
            }
        });
}

/// ChartId に対応するチャートウィジェットを描画する
fn show_chart(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    widgets: &mut WidgetStates,
    chart_id: &ChartId,
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
            widgets
                .importance
                .show(ui, sensitivity.as_ref(), &obj_names);
        }
        ChartId::PdpChart => {
            widgets.pdp_chart.show(ui, &param_names, &obj_names);
        }
        ChartId::ParallelCoordinates => {
            ui.label("Parallel Coordinates chart is not yet implemented.");
            ui.label(format!("{} trials loaded", trial_rows.len()));
        }
        ChartId::ScatterMatrix => {
            ui.label("Scatter Matrix chart is not yet implemented.");
        }
        ChartId::ParetoScatter3D => {
            ui.label("3D Pareto chart requires GPU rendering (not yet wired up).");
        }
        ChartId::SensitivityHeatmap => {
            if sensitivity.is_some() {
                ui.label("Sensitivity data available. Heatmap rendering not yet implemented.");
            } else {
                ui.label("No sensitivity data. Heatmap rendering not yet implemented.");
            }
        }
        ChartId::ClusterScatter => {
            if app_state.cluster_result.is_some() {
                ui.label("Cluster data available. Cluster scatter not yet implemented.");
            } else {
                ui.label("No cluster data. Cluster scatter not yet implemented.");
            }
        }
    }
}
