use crate::state::app_state::AppState;
use crate::state::types::Direction;

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
                let is_minimize = app_state
                    .current_study
                    .as_ref()
                    .and_then(|ctx| ctx.meta.directions.first())
                    .map(|d| matches!(d, Direction::Minimize))
                    .unwrap_or(true);
                let (best_trial_id, best_value) = if is_minimize {
                    history
                        .iter()
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .copied()
                        .unwrap()
                } else {
                    history
                        .iter()
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .copied()
                        .unwrap()
                };
                ui.label(format!("Best: {:.6}", best_value));
                ui.label(format!("Best Trial: #{best_trial_id}"));
                let rate =
                    tunny_core::convergence::compute_improvement_rate(history, 100, is_minimize);
                ui.label(format!("Improvement (last 100): {:.1}%", rate * 100.0));
            }
        }
    });
}
