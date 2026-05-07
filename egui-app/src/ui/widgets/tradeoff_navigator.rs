use crate::state::app_state::AppState;
use crate::state::messages::AppMessage;

pub fn show_tradeoff_navigator(
    ui: &mut egui::Ui,
    app_state: &mut AppState,
    objective_names: &[String],
    is_minimize: &[bool],
    tx: &std::sync::mpsc::SyncSender<AppMessage>,
) {
    if objective_names.len() < 2 {
        return;
    }

    ui.collapsing("[*] Trade-off Navigator", |ui| {
        if app_state.tradeoff_weights.len() != objective_names.len() {
            let n = objective_names.len();
            app_state.tradeoff_weights = vec![1.0 / n as f64; n];
        }

        let mut changed = false;

        for (i, name) in objective_names.iter().enumerate() {
            let mut val = app_state.tradeoff_weights[i] as f32;
            if ui
                .add(egui::Slider::new(&mut val, 0.0_f32..=1.0_f32).text(name))
                .changed()
            {
                app_state.tradeoff_weights[i] = val as f64;
                changed = true;
            }
        }

        if changed {
            tunny_core::multi_objective::weights::normalize_weights(
                &mut app_state.tradeoff_weights,
            );
            crate::state::message_handler::MessageHandler::trigger_tradeoff_computation(
                app_state.tradeoff_weights.clone(),
                is_minimize.to_vec(),
                tx.clone(),
            );
        }

        if let Some(indices) = &app_state.tradeoff_sorted_indices {
            if let Some(&best_id) = indices.first() {
                ui.label(format!("[*] Best Trial: #{best_id}"));
            }
        }
    });
}
