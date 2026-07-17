use crate::state::results::{EntropyResult, McdmMethod, McdmResult, WeightMode};
use crate::state::types::StudyView;
use crate::theme::chart_colors::{
    COLOR_BAR_ACCENT, COLOR_BAR_NEGATIVE, COLOR_BAR_PRIMARY, COLOR_EMPTY_STATE,
};

/// MCDM compute request payload
pub struct McdmComputeRequest {
    pub method: McdmMethod,
    pub weights: Vec<f64>,
    pub v: f64,
}

/// Cache key for MCDM results.
/// Each chart (Ranking / Scatter2D / Scatter3D / Table) references
/// `app_state.mcdm_cache` with this key so that results computed for the same
/// settings (method, weight mode, weights, v value) can be shared and reused.
///
/// Weights and v are continuous values, so they are quantized (6 decimal places)
/// to make the key Hash/Eq-able.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct McdmCacheKey {
    pub method: McdmMethod,
    pub weight_mode: WeightMode,
    pub weights_q: Vec<i64>,
    pub v_q: i64,
}

impl McdmCacheKey {
    fn quantize(x: f64) -> i64 {
        (x * 1_000_000.0).round() as i64
    }

    /// Builds a key from already-normalized weights.
    fn from_normalized(
        method: McdmMethod,
        weight_mode: WeightMode,
        weights: &[f64],
        v: f64,
    ) -> Self {
        let weights_q = weights.iter().map(|&w| Self::quantize(w)).collect();
        // v is only meaningful for VIKOR, so normalize it to 0 for other methods.
        let v_q = if method == McdmMethod::Vikor {
            Self::quantize(v)
        } else {
            0
        };
        Self {
            method,
            weight_mode,
            weights_q,
            v_q,
        }
    }

    /// Builds a key from the current settings (unnormalized weights).
    pub fn from_settings(
        method: McdmMethod,
        weight_mode: WeightMode,
        weights: &[f64],
        v: f64,
    ) -> Self {
        Self::from_normalized(method, weight_mode, &normalize_weights(weights), v)
    }

    /// Builds a key from a compute request (weights already normalized).
    pub fn from_request(req: &McdmComputeRequest, weight_mode: WeightMode) -> Self {
        Self::from_normalized(req.method, weight_mode, &req.weights, req.v)
    }
}

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

/// UI state for the MCDM ranking bar chart.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmRankChart {
    pub controls: McdmControls,
}

/// UI state for the MCDM ranking table.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmTable {
    pub controls: McdmControls,
}

/// Returns normalized weights (delegates to `tunny_core::mcdm::normalize_weights`).
pub fn normalize_weights(weights: &[f64]) -> Vec<f64> {
    tunny_core::mcdm::normalize_weights(weights)
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

/// Common extracted data for the top-N ranking entries.
struct RankingEntry {
    rank: usize,
    trial_idx: usize,
    score: f64,
}

/// Generates the top-N ranking entries from a McdmResult.
fn enumerate_ranked(result: &McdmResult, top_n: usize) -> Vec<RankingEntry> {
    let scores = result.primary_scores();
    let ranked = result.ranked_indices();
    let count = top_n.min(ranked.len());

    (0..count)
        .map(|rank| {
            let trial_idx = ranked[rank] as usize;
            let score = scores.get(trial_idx).copied().unwrap_or(0.0);
            RankingEntry {
                rank: rank + 1,
                trial_idx,
                score,
            }
        })
        .collect()
}

/// Table row data.
pub struct RankingRow {
    pub rank: usize,
    /// Global trial_id used for pinning/highlighting.
    pub trial_id: u32,
    /// Optuna trial.number for display (0-based creation order within the Study).
    pub trial_number: u32,
    pub score: f64,
    pub parameters: Vec<f64>,
    pub objectives: Vec<f64>,
}

/// Generates the top-N table row data from a McdmResult.
pub fn build_ranking_rows(
    result: &McdmResult,
    view: &StudyView,
    param_names: &[String],
    obj_names: &[String],
    top_n: usize,
) -> Vec<RankingRow> {
    let param_cols = view.numeric_columns(param_names);
    let obj_cols = view.numeric_columns(obj_names);
    enumerate_ranked(result, top_n)
        .into_iter()
        .map(|e| {
            let parameters: Vec<f64> = param_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(e.trial_idx)).copied().unwrap_or(0.0))
                .collect();
            let objectives: Vec<f64> = obj_cols
                .iter()
                .map(|col| col.and_then(|c| c.get(e.trial_idx)).copied().unwrap_or(0.0))
                .collect();
            RankingRow {
                rank: e.rank,
                trial_id: view
                    .trial_ids
                    .get(e.trial_idx)
                    .copied()
                    .unwrap_or(e.trial_idx as u32),
                // Display the Optuna trial.number rather than the row index
                // (they diverge in a Study that includes pruned/failed trials).
                trial_number: view
                    .df
                    .get_trial_number(e.trial_idx)
                    .unwrap_or(e.trial_idx as u32),
                score: e.score,
                parameters,
                objectives,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::TopsisResult;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

    #[test]
    fn adopt_compute_state_syncs_runtime_and_preserves_ui_settings() {
        let mut item = McdmRankChart {
            controls: McdmControls {
                computing: true,
                method: McdmMethod::Vikor,
                top_n: McdmTopN::Top20,
                v_param: 0.7,
                ..Default::default()
            },
        };
        let global = McdmRankChart {
            controls: McdmControls {
                computing: false,
                weights: vec![0.25, 0.75],
                ..Default::default()
            },
        };

        item.adopt_compute_state(&global);

        // Execution state and shared output are adopted.
        assert!(!item.controls.computing);
        assert_eq!(item.controls.weights, vec![0.25, 0.75]);
        // UI settings remain item-specific.
        assert_eq!(item.controls.method, McdmMethod::Vikor);
        assert_eq!(item.controls.top_n, McdmTopN::Top20);
        assert_eq!(item.controls.v_param, 0.7);
    }

    fn make_simple_view(n: usize) -> StudyView {
        if n == 0 {
            let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
            return StudyView::new(Arc::new(df), vec![]);
        }
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &[], &[], &[], 0);
        StudyView::new(Arc::new(df), vec![0; n])
    }

    fn make_view_with_objectives(objective_rows: &[Vec<f64>]) -> (StudyView, Vec<String>) {
        let n = objective_rows.len();
        if n == 0 {
            return (make_simple_view(0), vec![]);
        }
        let n_obj = objective_rows[0].len();
        let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: objective_rows[i].clone(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
        (StudyView::new(Arc::new(df), vec![0; n]), obj_names)
    }

    fn make_topsis_result(scores: Vec<f64>, ranked_indices: Vec<u32>) -> McdmResult {
        McdmResult::Topsis(TopsisResult {
            scores,
            ranked_indices,
            duration_ms: 10.0,
        })
    }

    #[test]
    fn mcdm_top_n_values() {
        assert_eq!(McdmTopN::Top5.value(), 5);
        assert_eq!(McdmTopN::Top10.value(), 10);
        assert_eq!(McdmTopN::Top20.value(), 20);
    }

    #[test]
    fn normalize_weights_equal() {
        let result = normalize_weights(&[0.5, 0.5]);
        assert!((result[0] - 0.5).abs() < 1e-9);
        assert!((result[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_unequal() {
        let result = normalize_weights(&[1.0, 3.0]);
        assert!((result[0] - 0.25).abs() < 1e-9);
        assert!((result[1] - 0.75).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_three_equal() {
        let result = normalize_weights(&[2.0, 2.0, 2.0]);
        for w in &result {
            assert!((w - 1.0 / 3.0).abs() < 1e-9);
        }
    }

    #[test]
    fn normalize_weights_zero_fallback() {
        let result = normalize_weights(&[0.0, 0.0]);
        assert!((result[0] - 0.5).abs() < 1e-9);
        assert!((result[1] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn normalize_weights_empty() {
        let result = normalize_weights(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn mcdm_rank_chart_default() {
        let chart = McdmRankChart::default();
        let c = &chart.controls;
        assert_eq!(c.method, McdmMethod::Topsis);
        assert_eq!(c.weight_mode, WeightMode::Manual);
        assert!(!c.computing);
        assert!(c.pending_compute.is_none());
        assert!(!c.pending_entropy);
        assert!(c.entropy_result.is_none());
        assert_eq!(c.top_n, McdmTopN::Top10);
        assert!(c.weights.is_empty());
        assert!((c.v_param - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn mcdm_table_default() {
        let table = McdmTable::default();
        assert_eq!(table.controls.top_n, McdmTopN::Top10);
    }

    #[test]
    fn enumerate_ranked_top5_with_5_results() {
        let result = make_topsis_result(vec![0.9, 0.7, 0.5, 0.3, 0.1], vec![0, 1, 2, 3, 4]);
        let view = make_simple_view(5);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 5);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
        assert!((ranking[4].score - 0.1).abs() < 1e-9);
    }

    #[test]
    fn enumerate_ranked_top10_with_20_results() {
        let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
        let ranked: Vec<u32> = (0..20).collect();
        let result = make_topsis_result(scores, ranked);
        let view = make_simple_view(20);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
        assert_eq!(ranking.len(), 10);
    }

    #[test]
    fn enumerate_ranked_top5_with_3_results_min_applied() {
        let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
        let view = make_simple_view(3);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 3);
    }

    #[test]
    fn enumerate_ranked_scores_match_ranked_order() {
        let result = make_topsis_result(vec![0.1, 0.9, 0.5], vec![1, 2, 0]);
        let view = make_simple_view(3);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 10);
        assert_eq!(ranking.len(), 3);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
        assert!((ranking[1].score - 0.5).abs() < 1e-9);
        assert!((ranking[2].score - 0.1).abs() < 1e-9);
    }

    #[test]
    fn enumerate_ranked_empty_result() {
        let result = make_topsis_result(vec![], vec![]);
        let view = make_simple_view(0);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert!(ranking.is_empty());
    }

    #[test]
    fn top_n_toggle_cycle() {
        let mut chart = McdmRankChart::default();
        assert_eq!(chart.controls.top_n, McdmTopN::Top10);
        chart.controls.top_n = McdmTopN::Top5;
        assert_eq!(chart.controls.top_n.value(), 5);
        chart.controls.top_n = McdmTopN::Top20;
        assert_eq!(chart.controls.top_n.value(), 20);
    }

    #[test]
    fn build_ranking_rows_basic() {
        let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![0, 1, 2]);
        let view = make_simple_view(3);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 3);
        assert_eq!(ranking[0].rank, 1);
        assert_eq!(ranking[0].trial_number, 0);
        assert!((ranking[0].score - 0.9).abs() < 1e-9);
    }

    #[test]
    fn build_ranking_rows_top_n_limit() {
        let scores: Vec<f64> = (0..20).map(|i| 1.0 - i as f64 / 20.0).collect();
        let ranked: Vec<u32> = (0..20).collect();
        let result = make_topsis_result(scores, ranked);
        let view = make_simple_view(20);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking.len(), 5);
    }

    #[test]
    fn build_ranking_rows_rank_starts_at_1() {
        let result = make_topsis_result(vec![0.8], vec![0]);
        let view = make_simple_view(1);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert_eq!(ranking[0].rank, 1);
    }

    #[test]
    fn build_ranking_rows_distinguishes_trial_id_and_number() {
        // Verify both are resolved correctly for a Study where trial_id (global, used
        // for pinning) and trial.number (for display) diverge (e.g. when it includes
        // pruned/failed trials).
        let core_rows: Vec<CoreRow> = (0..3)
            .map(|i| CoreRow {
                trial_id: i as u32 + 10,
                trial_number: i as u32 + 100,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &[], &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0; 3]);

        let result = make_topsis_result(vec![0.9, 0.5, 0.1], vec![2, 0, 1]);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        // rank 1 is trial_idx 2 -> trial_id 12 / number 102
        assert_eq!(ranking[0].trial_id, 12);
        assert_eq!(ranking[0].trial_number, 102);
    }

    #[test]
    fn build_ranking_rows_empty() {
        let result = make_topsis_result(vec![], vec![]);
        let view = make_simple_view(0);
        let ranking = build_ranking_rows(&result, &view, &[], &[], 5);
        assert!(ranking.is_empty());
    }

    #[test]
    fn build_ranking_rows_objectives_included() {
        let result = make_topsis_result(vec![0.9, 0.5], vec![0, 1]);
        let (view, obj_names) = make_view_with_objectives(&[vec![1.0, 2.0], vec![3.0, 4.0]]);
        let ranking = build_ranking_rows(&result, &view, &[], &obj_names, 10);
        assert_eq!(ranking[0].objectives, vec![1.0, 2.0]);
        assert_eq!(ranking[1].objectives, vec![3.0, 4.0]);
    }

    // ── E2E / integration tests ──

    fn multi_obj_data() -> Vec<Vec<f64>> {
        vec![
            vec![0.1, 0.9],
            vec![0.5, 0.5],
            vec![0.9, 0.1],
            vec![0.3, 0.7],
            vec![0.7, 0.3],
        ]
    }

    #[test]
    fn topsis_full_pipeline_equal_weights() {
        let data = multi_obj_data();
        let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
        let weights = normalize_weights(&[1.0, 1.0]);
        let is_minimize = vec![true, true];

        let core_result =
            tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();

        let mcdm_result = McdmResult::Topsis(TopsisResult {
            scores: core_result.scores.clone(),
            ranked_indices: core_result.ranked_indices.clone(),
            duration_ms: core_result.duration_ms,
        });

        assert_eq!(mcdm_result.primary_scores().len(), 5);
        assert!(!mcdm_result.primary_scores().iter().any(|s| s.is_nan()));

        let (view, obj_names) = make_view_with_objectives(&data);
        let ranking = build_ranking_rows(&mcdm_result, &view, &[], &obj_names, 5);
        assert_eq!(ranking.len(), 5);
        assert_eq!(ranking[0].rank, 1);
        for i in 1..ranking.len() {
            assert!(ranking[i - 1].score >= ranking[i].score);
        }
    }

    #[test]
    fn topsis_weight_bias_changes_ranking() {
        let data = multi_obj_data();
        let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
        let is_minimize = vec![true, true];

        let weights_obj0 = normalize_weights(&[1.0, 0.0]);
        let r0 = tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj0, &is_minimize)
            .unwrap();

        let weights_obj1 = normalize_weights(&[0.0, 1.0]);
        let r1 = tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights_obj1, &is_minimize)
            .unwrap();

        assert_ne!(
            r0.ranked_indices, r1.ranked_indices,
            "different weights should produce different rankings"
        );
    }

    #[test]
    fn topsis_single_objective_works() {
        let objectives: Vec<f64> = (0..5).map(|i| i as f64 * 0.2).collect();
        let weights = normalize_weights(&[1.0]);
        let is_minimize = vec![true];

        let result = tunny_core::topsis::compute_topsis(&objectives, 5, 1, &weights, &is_minimize);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.scores.len(), 5);
    }

    #[test]
    fn mcdm_chart_run_button_sets_pending_compute() {
        let mut chart = McdmRankChart::default();
        assert!(chart.controls.pending_compute.is_none());
        assert!(!chart.controls.computing);

        let normalized = normalize_weights(&[1.0, 1.0]);
        chart.controls.pending_compute = Some(McdmComputeRequest {
            method: McdmMethod::Topsis,
            weights: normalized,
            v: 0.5,
        });
        chart.controls.computing = true;

        assert!(chart.controls.pending_compute.is_some());
        assert!(chart.controls.computing);

        let payload = chart.controls.pending_compute.take();
        assert!(payload.is_some());
        assert!(chart.controls.pending_compute.is_none());
        assert!(chart.controls.computing);
    }

    #[test]
    fn mcdm_compute_request_vikor_includes_v() {
        let req = McdmComputeRequest {
            method: McdmMethod::Vikor,
            weights: vec![0.5, 0.5],
            v: 0.3,
        };
        assert_eq!(req.method, McdmMethod::Vikor);
        assert!((req.v - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn top_n_toggle_updates_display() {
        let data = multi_obj_data();
        let objectives: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).collect();
        let weights = normalize_weights(&[1.0, 1.0]);
        let is_minimize = vec![true, true];

        let core_result =
            tunny_core::topsis::compute_topsis(&objectives, 5, 2, &weights, &is_minimize).unwrap();
        let mcdm = McdmResult::Topsis(TopsisResult {
            scores: core_result.scores,
            ranked_indices: core_result.ranked_indices,
            duration_ms: core_result.duration_ms,
        });

        let (view, obj_names) = make_view_with_objectives(&data);

        let rows5 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 5);
        assert_eq!(rows5.len(), 5);

        let rows3 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 3);
        assert_eq!(rows3.len(), 3);

        let rows10 = build_ranking_rows(&mcdm, &view, &[], &obj_names, 10);
        assert_eq!(rows10.len(), 5);
    }
}
