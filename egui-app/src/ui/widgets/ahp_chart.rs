use crate::state::app_state::TrialRow;
use crate::state::results::AhpResult;
use crate::theme::chart_colors::COLOR_BAR_PRIMARY;
use crate::theme::ERROR_COLOR;

#[derive(Debug)]
pub struct AhpComputeRequest {
    pub objectives: Vec<f64>,
    pub n_trials: usize,
    pub n_objectives: usize,
    pub pairwise_matrix: Vec<f64>,
    pub is_minimize: Vec<bool>,
}

/// Read-only context passed from chart_registry to avoid too-many-arguments.
pub struct AhpDataContext<'a> {
    pub values: &'a [f64],
    pub n_trials: usize,
    pub n_objectives: usize,
    pub is_minimize: &'a [bool],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AhpTopN {
    #[default]
    Top5,
    Top10,
    Top20,
}

impl AhpTopN {
    pub fn count(&self) -> usize {
        match self {
            AhpTopN::Top5 => 5,
            AhpTopN::Top10 => 10,
            AhpTopN::Top20 => 20,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            AhpTopN::Top5 => "Top 5",
            AhpTopN::Top10 => "Top 10",
            AhpTopN::Top20 => "Top 20",
        }
    }

    fn show_combo(&mut self, ui: &mut egui::Ui, id: &str) {
        egui::ComboBox::from_id_salt(id)
            .selected_text(self.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(self, AhpTopN::Top5, AhpTopN::Top5.label());
                ui.selectable_value(self, AhpTopN::Top10, AhpTopN::Top10.label());
                ui.selectable_value(self, AhpTopN::Top20, AhpTopN::Top20.label());
            });
    }
}

#[derive(Debug, Default)]
pub struct AhpChart {
    pub pairwise: Vec<f64>,
    pub computing: bool,
    pub pending_compute: Option<AhpComputeRequest>,
    pub top_n: AhpTopN,
}

impl AhpChart {
    pub fn reset_for_objectives(n_objectives: usize) -> Self {
        let upper_len = n_objectives * n_objectives.saturating_sub(1) / 2;
        Self {
            pairwise: vec![1.0; upper_len],
            computing: false,
            pending_compute: None,
            top_n: AhpTopN::default(),
        }
    }

    pub fn upper_tri_index(n: usize, i: usize, j: usize) -> usize {
        tunny_core::ahp::upper_tri_index(n, i, j)
    }

    pub fn show_rank_chart(
        &mut self,
        ui: &mut egui::Ui,
        obj_names: &[String],
        result: &Option<AhpResult>,
        ctx: &AhpDataContext<'_>,
    ) {
        let n_objectives = ctx.n_objectives;

        if n_objectives == 0 {
            ui.vertical_centered(|ui| {
                ui.colored_label(egui::Color32::GRAY, "Select a study first");
            });
            return;
        }

        // Initialize pairwise if size changed
        let expected_len = n_objectives * n_objectives.saturating_sub(1) / 2;
        if self.pairwise.len() != expected_len {
            self.pairwise = vec![1.0; expected_len];
        }

        // Pairwise comparison matrix grid (n >= 2 only)
        if n_objectives >= 2 {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Pairwise Comparison Matrix").strong());
                egui::Grid::new("ahp_pairwise_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        // Header row
                        ui.label("");
                        for name in obj_names {
                            ui.label(egui::RichText::new(name.as_str()).strong().small());
                        }
                        ui.end_row();

                        for (i, row_name) in obj_names.iter().enumerate() {
                            ui.label(egui::RichText::new(row_name.as_str()).strong().small());
                            for j in 0..n_objectives {
                                if i == j {
                                    ui.label("1.0");
                                } else if i < j {
                                    let idx = Self::upper_tri_index(n_objectives, i, j);
                                    ui.add(
                                        egui::DragValue::new(&mut self.pairwise[idx])
                                            .range(1.0..=9.0)
                                            .speed(0.1)
                                            .fixed_decimals(1),
                                    );
                                } else {
                                    let idx = Self::upper_tri_index(n_objectives, j, i);
                                    let reciprocal = 1.0 / self.pairwise[idx];
                                    ui.label(format!("{:.3}", reciprocal));
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        }

        // Run button
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.computing, egui::Button::new("Run"))
                .clicked()
            {
                self.pending_compute = Some(AhpComputeRequest {
                    objectives: ctx.values.to_vec(),
                    n_trials: ctx.n_trials,
                    n_objectives: ctx.n_objectives,
                    pairwise_matrix: self.pairwise.clone(),
                    is_minimize: ctx.is_minimize.to_vec(),
                });
            }

            if self.computing {
                ui.spinner();
                ui.label("Computing...");
            }
        });

        ui.separator();

        if self.computing {
            return;
        }

        let Some(r) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(egui::Color32::GRAY, "Press Run to compute AHP ranking");
            });
            return;
        };

        // CR display
        let (cr_label, cr_color) = if r.is_consistent {
            (
                format!("CR = {:.3}  Consistent", r.cr),
                egui::Color32::GREEN,
            )
        } else {
            (
                format!("CR = {:.3}  Inconsistent (CR > 0.10)", r.cr),
                ERROR_COLOR,
            )
        };
        ui.colored_label(cr_color, &cr_label);

        ui.add_space(4.0);

        // Priority vector bar chart
        ui.label(egui::RichText::new("Priority Vector").strong());
        let max_w = r.priority_vector.iter().copied().fold(0.0_f64, f64::max);
        for (j, &w) in r.priority_vector.iter().enumerate() {
            ui.horizontal(|ui| {
                let name = obj_names.get(j).map(|s| s.as_str()).unwrap_or("?");
                ui.label(format!("{}: {:.3}", name, w));
                let ratio = if max_w > 0.0 { (w / max_w) as f32 } else { 0.0 };
                let (rect, _resp) =
                    ui.allocate_exact_size(egui::vec2(200.0 * ratio, 14.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 2.0, COLOR_BAR_PRIMARY);
            });
        }
    }

    pub fn show_table(
        &mut self,
        ui: &mut egui::Ui,
        obj_names: &[String],
        trial_rows: &[TrialRow],
        result: &Option<AhpResult>,
    ) {
        self.top_n.show_combo(ui, "ahp_top_n_combo");

        ui.separator();

        let Some(r) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(egui::Color32::GRAY, "Press Run to compute AHP ranking");
            });
            return;
        };

        let top_n = self.top_n.count().min(r.ranked_indices.len());

        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Grid::new("ahp_ranking_table")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Rank").strong());
                        ui.label(egui::RichText::new("Trial").strong());
                        ui.label(egui::RichText::new("AHP Score").strong());
                        for name in obj_names {
                            ui.label(egui::RichText::new(name.as_str()).strong().small());
                        }
                        ui.end_row();

                        for (rank, &trial_idx) in r.ranked_indices.iter().take(top_n).enumerate() {
                            ui.label(format!("{}", rank + 1));
                            ui.label(format!("#{}", trial_idx));
                            ui.label(format!("{:.4}", r.scores[trial_idx as usize]));
                            if let Some(row) = trial_rows.get(trial_idx as usize) {
                                for &val in &row.objectives {
                                    ui.label(format!("{:.4}", val));
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_tri_index_n3() {
        assert_eq!(AhpChart::upper_tri_index(3, 0, 1), 0);
        assert_eq!(AhpChart::upper_tri_index(3, 0, 2), 1);
        assert_eq!(AhpChart::upper_tri_index(3, 1, 2), 2);
    }

    #[test]
    fn reset_for_objectives_n3() {
        let chart = AhpChart::reset_for_objectives(3);
        assert_eq!(chart.pairwise.len(), 3);
        assert!(chart
            .pairwise
            .iter()
            .all(|&v| (v - 1.0).abs() < f64::EPSILON));
        assert!(!chart.computing);
        assert!(chart.pending_compute.is_none());
    }

    #[test]
    fn top_n_default_is_top5() {
        assert_eq!(AhpTopN::default(), AhpTopN::Top5);
        assert_eq!(AhpTopN::default().count(), 5);
    }
}
