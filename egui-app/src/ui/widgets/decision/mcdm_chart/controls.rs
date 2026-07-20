use crate::state::results::{EntropyResult, McdmMethod, WeightMode};
use crate::theme::chart_colors::COLOR_EMPTY_STATE;

use super::compute::{normalize_weights, McdmCacheKey, McdmComputeRequest};

/// Top-N display toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum McdmTopN {
    Top5,
    Top10,
    Top20,
}

impl McdmTopN {
    pub fn value(&self) -> usize {
        match self {
            McdmTopN::Top5 => 5,
            McdmTopN::Top10 => 10,
            McdmTopN::Top20 => 20,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            McdmTopN::Top5 => "Top 5",
            McdmTopN::Top10 => "Top 10",
            McdmTopN::Top20 => "Top 20",
        }
    }

    fn show_combo(&mut self, ui: &mut egui::Ui, id: &str) {
        egui::ComboBox::from_id_salt(id)
            .selected_text(self.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(self, McdmTopN::Top5, McdmTopN::Top5.label());
                ui.selectable_value(self, McdmTopN::Top10, McdmTopN::Top10.label());
                ui.selectable_value(self, McdmTopN::Top20, McdmTopN::Top20.label());
            });
    }
}

/// Shared configuration and execution state for MCDM charts.
/// Holds method / weight mode / weights / v value / Top N, plus compute execution
/// state (computing / pending). Each of the Ranking / Scatter2D / Scatter3D / Table
/// charts keeps its own instance and displays independent results by referencing
/// `app_state.mcdm_cache` via `cache_key()`.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmControls {
    pub method: McdmMethod,
    pub weight_mode: WeightMode,
    pub weights: Vec<f64>,
    pub v_param: f64,
    pub top_n: McdmTopN,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub pending_compute: Option<McdmComputeRequest>,
    #[serde(skip)]
    pub pending_entropy: bool,
    #[serde(skip)]
    pub entropy_result: Option<EntropyResult>,
}

impl Default for McdmControls {
    fn default() -> Self {
        Self {
            method: McdmMethod::Topsis,
            weight_mode: WeightMode::Manual,
            weights: Vec::new(),
            v_param: 0.5,
            top_n: McdmTopN::Top10,
            computing: false,
            pending_compute: None,
            pending_entropy: false,
            entropy_result: None,
        }
    }
}

impl McdmControls {
    /// Adopts the compute execution state and shared output from the global widget.
    /// Since compute results are aggregated in `app_state.mcdm_cache`, only execution
    /// state such as the computing flag, entropy weights, and entropy details is
    /// propagated to each canvas item. UI settings such as method, WeightMode, Top N,
    /// and v value are item-specific and left untouched.
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.pending_entropy = src.pending_entropy;
        self.weights = src.weights.clone();
        self.entropy_result = src.entropy_result.clone();
    }

    /// Returns the cache key corresponding to the current settings.
    pub fn cache_key(&self) -> McdmCacheKey {
        McdmCacheKey::from_settings(self.method, self.weight_mode, &self.weights, self.v_param)
    }

    /// Draws the settings UI (method / weight mode / Top N / Run / weights / entropy details).
    /// Returns false when there are no objectives, in which case the caller should skip
    /// the rest of the rendering. `id_prefix` separates the egui ID namespace so that
    /// control IDs don't collide when multiple MCDM charts are placed on the same screen.
    pub fn show_controls(
        &mut self,
        ui: &mut egui::Ui,
        obj_names: &[String],
        id_prefix: &str,
    ) -> bool {
        let obj_count = obj_names.len();
        if obj_count == 0 {
            ui.vertical_centered(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE(), "Select a study first");
            });
            return false;
        }

        if self.weights.len() != obj_count {
            self.weights = vec![1.0; obj_count];
        }

        ui.push_id(id_prefix, |ui| {
            // Method selector + WeightMode + Top N + Run button + spinner
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("mcdm_method_combo")
                    .selected_text(self.method.label())
                    .show_ui(ui, |ui| {
                        for m in McdmMethod::all() {
                            ui.selectable_value(&mut self.method, *m, m.label());
                        }
                    });

                // WeightMode selector (next to the method selector)
                let prev_weight_mode = self.weight_mode;
                egui::ComboBox::from_id_salt("mcdm_weight_mode_combo")
                    .selected_text(format!("Weight: {}", self.weight_mode.label()))
                    .show_ui(ui, |ui| {
                        for wm in WeightMode::all() {
                            ui.selectable_value(&mut self.weight_mode, *wm, wm.label());
                        }
                    });

                // WeightMode switch logic
                if prev_weight_mode != self.weight_mode {
                    self.pending_entropy = self.weight_mode == WeightMode::Entropy;
                }

                self.top_n.show_combo(ui, "mcdm_top_n_combo");

                if ui
                    .add_enabled(!self.computing, egui::Button::new("Run"))
                    .clicked()
                {
                    let normalized = normalize_weights(&self.weights);
                    self.pending_compute = Some(McdmComputeRequest {
                        method: self.method,
                        weights: normalized,
                        v: self.v_param,
                    });
                    self.computing = true;
                }

                if self.computing {
                    ui.spinner();
                    ui.label("Computing...");
                }
            });

            ui.separator();

            // Weight sliders
            ui.collapsing("Weights", |ui| {
                let normalized = normalize_weights(&self.weights);
                let is_entropy = self.weight_mode == WeightMode::Entropy;
                for (i, obj_name) in obj_names.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(obj_name.as_str()).strong());
                        if is_entropy {
                            // Entropy mode: read-only slider
                            let mut w = self.weights[i];
                            ui.add_enabled(
                                false,
                                egui::Slider::new(&mut w, 0.0..=1.0).text("weight"),
                            );
                        } else {
                            let mut w = self.weights[i];
                            if ui
                                .add(egui::Slider::new(&mut w, 0.0..=1.0).text("weight"))
                                .changed()
                            {
                                self.weights[i] = w;
                            }
                        }
                        ui.label(format!("(norm: {:.2})", normalized[i]));
                    });
                }
                let norm_sum: f64 = normalized.iter().sum();
                ui.label(format!("Sum: {:.2}", norm_sum));

                if self.method == McdmMethod::Vikor {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Strategy weight v").strong());
                        ui.add(egui::Slider::new(&mut self.v_param, 0.0..=1.0).text("v"));
                        ui.label("(0=min-regret, 1=max-consensus)");
                    });
                }
            });

            // Entropy results table
            if self.weight_mode == WeightMode::Entropy {
                if let Some(ref entropy) = self.entropy_result {
                    ui.collapsing("Entropy Details", |ui| {
                        ui.label(format!("Computed in {:.1}ms", entropy.duration_ms));

                        use egui_extras::{Column, TableBuilder};
                        let n_obj = entropy.weights.len();
                        if n_obj == 0 {
                            ui.colored_label(COLOR_EMPTY_STATE(), "No data");
                            return;
                        }

                        TableBuilder::new(ui)
                            .striped(true)
                            .column(Column::exact(120.0))
                            .columns(Column::remainder(), n_obj)
                            .header(20.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong("Metric");
                                });
                                for name in obj_names.iter().take(n_obj) {
                                    header.col(|ui| {
                                        ui.strong(name);
                                    });
                                }
                            })
                            .body(|mut body| {
                                // Entropy row
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label("Entropy");
                                    });
                                    for &e in &entropy.entropies {
                                        row.col(|ui| {
                                            ui.label(format!("{:.4}", e));
                                        });
                                    }
                                });
                                // Diversity row
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label("Diversity");
                                    });
                                    for &d in &entropy.diversities {
                                        row.col(|ui| {
                                            ui.label(format!("{:.4}", d));
                                        });
                                    }
                                });
                                // Weight row
                                body.row(18.0, |mut row| {
                                    row.col(|ui| {
                                        ui.strong("Weight");
                                    });
                                    for &w in &entropy.weights {
                                        row.col(|ui| {
                                            ui.strong(format!("{:.4}", w));
                                        });
                                    }
                                });
                            });
                    });
                }
            }

            ui.separator();
        });

        true
    }
}
