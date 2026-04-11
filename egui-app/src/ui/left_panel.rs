use crate::state::app_state::{AppState, ColorMode};
use crate::state::layout_state::{ChartId, LayoutState};

/// LeftPanel を描画する
pub fn show_left_panel(ui: &mut egui::Ui, app_state: &mut AppState, layout: &mut LayoutState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        show_study_info(ui, app_state);
        ui.separator();
        show_chart_selection(ui, layout);
        ui.separator();
        show_filter_sliders(ui, app_state);
        ui.separator();
        show_color_mode(ui, app_state);
    });
}

/// Study情報セクション
fn show_study_info(ui: &mut egui::Ui, app_state: &AppState) {
    ui.heading("Study Info");
    if let Some(ctx) = &app_state.current_study {
        ui.label(format!("Study: {}", ctx.meta.name));
        ui.label(format!("Trials: {}", ctx.meta.completed_trials));
        ui.label(format!("Objectives: {}", ctx.meta.objective_names.len()));
        ui.label(format!("Parameters: {}", ctx.meta.param_names.len()));
        ui.label(format!(
            "Selected: {} / {}",
            app_state.selected_indices.len(),
            ctx.trial_rows.len()
        ));
    } else {
        ui.label("Open a file");
    }
}

/// チャート表示選択リスト
fn show_chart_selection(ui: &mut egui::Ui, layout: &mut LayoutState) {
    ui.collapsing("Charts", |ui| {
        for chart_id in ChartId::all() {
            let mut visible = layout.is_chart_visible(chart_id);
            if ui.checkbox(&mut visible, chart_id.label()).changed() {
                layout.toggle_chart(chart_id.clone());
            }
        }
    });
}

/// 変数フィルタースライダー
fn show_filter_sliders(ui: &mut egui::Ui, app_state: &mut AppState) {
    let param_names: Vec<String> = app_state
        .current_study
        .as_ref()
        .map(|ctx| ctx.meta.param_names.clone())
        .unwrap_or_default();

    if param_names.is_empty() {
        return;
    }

    ui.collapsing("Filters", |ui| {
        for param_name in &param_names {
            let (data_min, data_max) = app_state
                .current_study
                .as_ref()
                .map(|ctx| ctx.param_range(param_name))
                .unwrap_or((0.0, 1.0));

            let (mut filter_min, mut filter_max) = app_state
                .filter_ranges
                .get(param_name)
                .copied()
                .unwrap_or((data_min, data_max));

            ui.label(param_name);
            let mut changed = false;
            changed |= ui
                .add(
                    egui::Slider::new(&mut filter_min, data_min..=data_max)
                        .text("min")
                        .clamp_to_range(true),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut filter_max, data_min..=data_max)
                        .text("max")
                        .clamp_to_range(true),
                )
                .changed();

            // min <= max を保証
            if filter_min > filter_max {
                filter_min = filter_max;
            }

            if changed {
                app_state.set_filter(param_name, filter_min, filter_max);
            }
        }
    });
}

/// カラーモード選択
fn show_color_mode(ui: &mut egui::Ui, app_state: &mut AppState) {
    ui.label("Color Mode:");
    let current_label = app_state.color_mode.label().to_string();
    egui::ComboBox::from_label("")
        .selected_text(current_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(
                &mut app_state.color_mode,
                ColorMode::ParetoRank,
                "Pareto Rank",
            );
            ui.selectable_value(
                &mut app_state.color_mode,
                ColorMode::TrialNumber,
                "Trial Number",
            );
            if let Some(ctx) = &app_state.current_study {
                for obj_name in ctx.meta.objective_names.clone() {
                    let mode = ColorMode::ObjectiveValue(obj_name.clone());
                    ui.selectable_value(
                        &mut app_state.color_mode,
                        mode,
                        format!("Objective: {}", obj_name),
                    );
                }
            }
            if app_state.cluster_result.is_some() {
                ui.selectable_value(
                    &mut app_state.color_mode,
                    ColorMode::ClusterId,
                    "Cluster ID",
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::{AppState, ColorMode};

    #[test]
    fn set_filter_updates_filter_ranges() {
        let mut state = AppState::new();
        state.set_filter("x", 0.0, 0.5);
        let range = state.filter_ranges.get("x").copied();
        assert_eq!(range, Some((0.0, 0.5)));
    }

    #[test]
    fn set_filter_overwrites_existing() {
        let mut state = AppState::new();
        state.set_filter("x", 0.0, 1.0);
        state.set_filter("x", 0.2, 0.8);
        let range = state.filter_ranges.get("x").copied();
        assert_eq!(range, Some((0.2, 0.8)));
    }

    #[test]
    fn color_mode_label_variants() {
        assert_eq!(ColorMode::ParetoRank.label(), "Pareto Rank");
        assert_eq!(ColorMode::TrialNumber.label(), "Trial Number");
        assert_eq!(ColorMode::ClusterId.label(), "Cluster ID");
        assert_eq!(
            ColorMode::ObjectiveValue("y".to_string()).label(),
            "Objective"
        );
    }

    #[test]
    fn color_mode_switch_updates_state() {
        let mut state = AppState::new();
        assert_eq!(state.color_mode, ColorMode::ParetoRank);
        state.color_mode = ColorMode::TrialNumber;
        assert_eq!(state.color_mode, ColorMode::TrialNumber);
    }

    #[test]
    fn chart_id_all_returns_all_variants() {
        let all = ChartId::all();
        assert_eq!(all.len(), 10);
        assert!(all.contains(&ChartId::ParetoScatter2D));
        assert!(all.contains(&ChartId::ClusterScatter));
    }

    #[test]
    fn chart_id_label_not_empty() {
        for chart_id in ChartId::all() {
            assert!(!chart_id.label().is_empty());
        }
    }
}
