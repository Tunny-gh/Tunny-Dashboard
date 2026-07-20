use crate::state::results::McdmResult;
use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_EMPTY_STATE;

use super::controls::McdmControls;
use super::ranking::build_ranking_rows;

/// UI state for the MCDM ranking table.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmTable {
    pub controls: McdmControls,
}

impl McdmTable {
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    /// Draws the MCDM ranking table.
    /// `pinned` is the set of currently pinned trial_ids. Returns the trial_id of the
    /// row whose pin button was clicked (the caller applies `AppState::toggle_pinned_trial`).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        result: Option<&McdmResult>,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        pinned: &[u32],
    ) -> Option<u32> {
        if !self.controls.show_controls(ui, obj_names, "mcdm_table") {
            return None;
        }

        if self.controls.computing {
            return None;
        }

        let Some(result) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE(), "Press Run to compute the MCDM ranking");
            });
            return None;
        };

        use egui_extras::{Column, TableBuilder};

        let rows = build_ranking_rows(
            result,
            view,
            param_names,
            obj_names,
            self.controls.top_n.value(),
        );
        if rows.is_empty() {
            ui.colored_label(COLOR_EMPTY_STATE(), "No results to display");
            return None;
        }

        let mut pin_toggled: Option<u32> = None;

        // Expand each variable/objective into its own column, allowing horizontal scroll
        // (same layout as the Cluster Table).
        egui::ScrollArea::horizontal().show(ui, |ui| {
            // Strengthen the stripe color to make it easier to distinguish even/odd rows.
            ui.visuals_mut().faint_bg_color = crate::theme::TABLE_STRIPE_BG();
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::exact(30.0)) // Pin
                .column(Column::initial(50.0).at_least(40.0)) // Rank
                .column(Column::initial(70.0).at_least(50.0)) // Trial
                .column(Column::initial(80.0).at_least(50.0)) // Score
                .columns(Column::initial(90.0).at_least(50.0), obj_names.len()) // each objective
                .columns(Column::initial(90.0).at_least(50.0), param_names.len()) // each variable
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("📌");
                    });
                    header.col(|ui| {
                        ui.strong("Rank");
                    });
                    header.col(|ui| {
                        ui.strong("Trial");
                    });
                    header.col(|ui| {
                        ui.strong("Score");
                    });
                    for name in obj_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                    for name in param_names {
                        header.col(|ui| {
                            ui.strong(name);
                        });
                    }
                })
                .body(|mut body| {
                    for row_data in &rows {
                        body.row(18.0, |mut row| {
                            let is_pinned = pinned.contains(&row_data.trial_id);
                            row.col(|ui| {
                                let pin_label = if is_pinned { "📌" } else { "·" };
                                if ui.small_button(pin_label).clicked() {
                                    pin_toggled = Some(row_data.trial_id);
                                }
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", row_data.rank));
                            });
                            row.col(|ui| {
                                ui.label(format!("{}", row_data.trial_number));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.4}", row_data.score));
                            });
                            for &val in &row_data.objectives {
                                row.col(|ui| {
                                    ui.label(format!("{:.4}", val));
                                });
                            }
                            for &val in &row_data.parameters {
                                row.col(|ui| {
                                    ui.label(format!("{:.3}", val));
                                });
                            }
                        });
                    }
                });
        });

        pin_toggled
    }
}
