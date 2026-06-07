use crate::state::app_state::AppState;
use crate::state::layout_state::LayoutState;

/// LeftPanel を描画する（フィルター専用、チャート選択は右パネルへ移動）
pub fn show_left_panel(ui: &mut egui::Ui, app_state: &mut AppState, _layout: &mut LayoutState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        show_study_info(ui, app_state);
        ui.separator();
        show_filter_sliders(ui, app_state);
        ui.separator();

        // REQ-008: Convergence Card（単目的のみ）
        let obj_count = app_state
            .current_study
            .as_ref()
            .map(|ctx| ctx.meta.objective_names.len())
            .unwrap_or(0);
        if obj_count == 1 {
            crate::ui::widgets::convergence_card::show_convergence_card(ui, app_state);
        }
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
            ctx.trial_count()
        ));
    } else {
        ui.label("Open a file");
    }
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
                        .clamping(egui::SliderClamping::Always),
                )
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut filter_max, data_min..=data_max)
                        .text("max")
                        .clamping(egui::SliderClamping::Always),
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

#[cfg(test)]
mod tests {

    use crate::state::app_state::AppState;
    use crate::state::layout_state::ChartId;

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
    fn chart_id_all_returns_all_variants() {
        let all = ChartId::all();
        assert_eq!(all.len(), 21);
        assert!(all.contains(&ChartId::ParetoScatter2D));
        assert!(all.contains(&ChartId::ClusterScatter));
        assert!(all.contains(&ChartId::ClusterTable));
        assert!(all.contains(&ChartId::McdmRankChart));
        assert!(all.contains(&ChartId::McdmScatterChart));
        assert!(all.contains(&ChartId::McdmTable));
        assert!(all.contains(&ChartId::SliceChart));
    }

    #[test]
    fn chart_id_label_not_empty() {
        for chart_id in ChartId::all() {
            assert!(!chart_id.label().is_empty());
        }
    }
}
