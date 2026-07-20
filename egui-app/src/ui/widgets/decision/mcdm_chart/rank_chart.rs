use crate::state::results::McdmResult;
use crate::theme::chart_colors::{
    COLOR_BAR_ACCENT, COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY, COLOR_EMPTY_STATE,
};

use super::controls::McdmControls;
use super::ranking::enumerate_ranked;

/// UI state for the MCDM ranking bar chart.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmRankChart {
    pub controls: McdmControls,
}

impl McdmRankChart {
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, obj_names: &[String], result: Option<&McdmResult>) {
        if !self.controls.show_controls(ui, obj_names, "mcdm_rank") {
            return;
        }

        if self.controls.computing {
            return;
        }

        let Some(result) = result else {
            ui.vertical_centered(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE(), "Press Run to compute MCDM ranking");
            });
            return;
        };

        ui.label(format!(
            "Computed in {:.1}ms ({})",
            result.duration_ms(),
            result.method_label()
        ));

        let label_width = 100.0_f32;
        let bar_height = 20.0_f32;
        let bar_gap = 4.0_f32;
        let value_text_width = 60.0_f32;

        if let McdmResult::PrometheeI(r) = result {
            let top_n = self.controls.top_n.value().min(r.ranked_indices_i.len());
            if top_n == 0 {
                ui.label("No data");
                return;
            }
            let max_val = r
                .phi_plus
                .iter()
                .chain(r.phi_minus.iter())
                .fold(0.0_f64, |a, &b| a.max(b));
            egui::ScrollArea::vertical().show(ui, |ui| {
                let available_width = ui.available_width() - label_width - value_text_width - 8.0;
                let bar_max_width = (available_width / 2.0).max(25.0);
                for rank in 0..top_n {
                    let idx = r.ranked_indices_i[rank] as usize;
                    // Access defensively via index, same as the neighboring incomparable_counts.
                    let phi_plus = r.phi_plus.get(idx).copied().unwrap_or(0.0);
                    let phi_minus = r.phi_minus.get(idx).copied().unwrap_or(0.0);
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [label_width, bar_height],
                            egui::Label::new(
                                egui::RichText::new(format!("Trial {idx}"))
                                    .text_style(egui::TextStyle::Body),
                            )
                            .truncate(),
                        );
                        let phi_plus_w = if max_val > 0.0 {
                            (phi_plus / max_val * bar_max_width as f64) as f32
                        } else {
                            0.0
                        };
                        let phi_minus_w = if max_val > 0.0 {
                            (phi_minus / max_val * bar_max_width as f64) as f32
                        } else {
                            0.0
                        };
                        let (rect_plus, _) = ui.allocate_exact_size(
                            egui::vec2(bar_max_width, bar_height - bar_gap),
                            egui::Sense::hover(),
                        );
                        if ui.is_rect_visible(rect_plus) {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect_plus.min,
                                    egui::vec2(phi_plus_w, rect_plus.height()),
                                ),
                                2.0,
                                COLOR_BAR_PRIMARY(),
                            );
                        }
                        let (rect_minus, _) = ui.allocate_exact_size(
                            egui::vec2(bar_max_width, bar_height - bar_gap),
                            egui::Sense::hover(),
                        );
                        if ui.is_rect_visible(rect_minus) {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect_minus.min,
                                    egui::vec2(phi_minus_w, rect_minus.height()),
                                ),
                                2.0,
                                COLOR_BAR_NEGATIVE(),
                            );
                        }
                        let incomparable = r
                            .incomparable_counts
                            .get(idx)
                            .copied()
                            .unwrap_or(0);
                        if incomparable > 0 {
                            ui.label(format!(
                                "Φ+{:.3} Φ-{:.3} \u{21F9}{incomparable}",
                                phi_plus, phi_minus
                            ))
                            .on_hover_text(format!(
                                "\u{21F9}{incomparable}: incomparable with {incomparable} trial(s) in the PROMETHEE I partial order \
                                 (neither trial outranks the other on both \u{3a6}+ and \u{3a6}-). \
                                 The displayed order is a tie-break for reference only."
                            ));
                        } else {
                            ui.label(format!("Φ+{:.3} Φ-{:.3}", phi_plus, phi_minus));
                        }
                    });
                }
            });
            return;
        }

        if let McdmResult::PrometheeII(r) = result {
            let top_n = self.controls.top_n.value().min(r.ranked_indices_ii.len());
            if top_n == 0 {
                ui.label("No data");
                return;
            }
            let max_abs = r.phi_net.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
            egui::ScrollArea::vertical().show(ui, |ui| {
                let available_width = ui.available_width() - label_width - value_text_width - 8.0;
                let bar_max_width = available_width.max(50.0);
                for rank in 0..top_n {
                    let idx = r.ranked_indices_ii[rank] as usize;
                    let phi_net = r.phi_net.get(idx).copied().unwrap_or(0.0);
                    let bar_w = if max_abs > 0.0 {
                        (phi_net.abs() / max_abs * bar_max_width as f64) as f32
                    } else {
                        0.0
                    };
                    let color = if phi_net >= 0.0 {
                        COLOR_BAR_PRIMARY()
                    } else {
                        COLOR_BAR_ACCENT()
                    };
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [label_width, bar_height],
                            egui::Label::new(
                                egui::RichText::new(format!("Trial {idx}"))
                                    .text_style(egui::TextStyle::Body),
                            )
                            .truncate(),
                        );
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(bar_max_width, bar_height - bar_gap),
                            egui::Sense::hover(),
                        );
                        if ui.is_rect_visible(rect) {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(bar_w, rect.height()),
                                ),
                                2.0,
                                color,
                            );
                        }
                        ui.label(format!("{phi_net:.4}"));
                    });
                }
            });
            return;
        }

        let entries = enumerate_ranked(result, self.controls.top_n.value());
        if entries.is_empty() {
            ui.label("No data");
            return;
        }

        let max_score = entries.iter().map(|e| e.score).fold(0.0_f64, f64::max);
        let bar_color = COLOR_BAR_PRIMARY();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width() - label_width - value_text_width - 8.0;
            let bar_max_width = available_width.max(50.0);

            for entry in &entries {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [label_width, bar_height],
                        egui::Label::new(
                            egui::RichText::new(format!("Trial {}", entry.trial_idx))
                                .text_style(egui::TextStyle::Body),
                        )
                        .truncate(),
                    );

                    let bar_width = if max_score > 0.0 {
                        (entry.score / max_score * bar_max_width as f64) as f32
                    } else {
                        0.0
                    };

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(bar_max_width, bar_height - bar_gap),
                        egui::Sense::hover(),
                    );
                    if ui.is_rect_visible(rect) {
                        let bar_rect = egui::Rect::from_min_size(
                            rect.min,
                            egui::vec2(bar_width, rect.height()),
                        );
                        ui.painter().rect_filled(bar_rect, 2.0, bar_color);
                    }

                    ui.label(format!("{:.4}", entry.score));
                });
            }
        });
    }
}
