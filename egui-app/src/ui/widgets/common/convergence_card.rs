use crate::state::app_state::AppState;

pub fn show_convergence_card(ui: &mut egui::Ui, app_state: &AppState) {
    ui.collapsing("[+] Convergence", |ui| {
        match &app_state.best_trial_history {
            None => {
                ui.label("No data");
            }
            Some(history) if history.is_empty() => {
                ui.label("No trials");
            }
            Some(history) => {
                let (best_trial_id, best_value) = history
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .copied()
                    .unwrap();
                ui.label(format!("Best: {:.6}", best_value));
                ui.label(format!("Best Trial: #{best_trial_id}"));
                let rate = tunny_core::convergence::compute_improvement_rate(history, 100);
                ui.label(format!("Improvement (last 100): {:.1}%", rate * 100.0));
            }
        }
    });
}
