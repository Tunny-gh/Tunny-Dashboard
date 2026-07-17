//! The CSV import confirmation dialog.
//!
//! Since a flat CSV doesn't carry optimization directions (maximize/minimize) or declared
//! variable ranges, this modal presents defaults derived from the observed values right
//! after loading, letting the user confirm or adjust them. Once confirmed, the edited
//! values are applied to `StudyMeta`; that direction determines the Pareto rank, and that
//! range determines the search box for surrogate optimization.

use egui::RichText;

use crate::state::app_state::CsvImportSettings;
use crate::ui::widgets::common::modal::ModalScaffold;

/// The dialog's operation result.
pub enum CsvImportAction {
    /// Loads the Study with the current edited values.
    Apply,
}

/// Draws the CSV import confirmation modal.
///
/// Returns `Some(CsvImportAction::Apply)` only when confirmed. The dialog stays open
/// while the return value is `None`. Esc / a click outside the dialog also confirms if
/// the range is valid.
pub fn show(ctx: &egui::Context, settings: &mut CsvImportSettings) -> Option<CsvImportAction> {
    let mut load_clicked = false;
    let valid = settings.bounds_valid();

    let outcome = ModalScaffold::new("csv_import_settings_modal", 440.0)
        .heading("CSV Import Settings")
        .show(ctx, |ui| {
            ui.label(
                RichText::new(format!("Study: {}", settings.study_name))
                    .color(crate::theme::TEXT_SECONDARY()),
            );
            ui.add_space(4.0);
            ui.label(
                "CSV files don't carry optimization directions or parameter ranges. \
                 Please confirm or adjust them before loading.",
            );
            ui.separator();

            // ── Objective optimization directions ──────────────────────────────────
            ui.label(RichText::new("Objective Directions").strong());
            egui::Grid::new("csv_import_directions")
                .num_columns(2)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    // Iterates only over the corresponding elements via zip, rather than
                    // direct indexing, so this doesn't panic even on CSV metadata where
                    // `objective_names` and `maximize` lengths disagree (the remainder is
                    // ignored).
                    for (i, (name, is_max)) in settings
                        .objective_names
                        .iter()
                        .zip(settings.maximize.iter_mut())
                        .enumerate()
                    {
                        ui.label(name);
                        egui::ComboBox::from_id_salt(("csv_import_dir", i))
                            .selected_text(if *is_max { "Maximize" } else { "Minimize" })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(is_max, false, "Minimize");
                                ui.selectable_value(is_max, true, "Maximize");
                            });
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);

            // ── Numeric parameter ranges ────────────────────────────
            ui.label(RichText::new("Parameter Ranges").strong());
            if settings.param_bounds.is_empty() {
                ui.label(
                    RichText::new("No numeric parameters.").color(crate::theme::TEXT_SECONDARY()),
                );
            } else {
                egui::Grid::new("csv_import_bounds")
                    .num_columns(3)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Parameter").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("Min").color(crate::theme::TEXT_SECONDARY()));
                        ui.label(RichText::new("Max").color(crate::theme::TEXT_SECONDARY()));
                        ui.end_row();
                        for pb in settings.param_bounds.iter_mut() {
                            ui.label(&pb.name);
                            // Uses 1% of the observed width as one step; double-clicking also allows direct entry.
                            let speed = ((pb.high - pb.low).abs() * 0.01).max(0.01);
                            ui.add(egui::DragValue::new(&mut pb.low).speed(speed));
                            ui.add(egui::DragValue::new(&mut pb.high).speed(speed));
                            ui.end_row();
                        }
                    });
            }

            if !valid {
                ui.add_space(4.0);
                ui.colored_label(
                    crate::theme::ERROR_COLOR(),
                    "Each parameter's Min must be a finite value smaller than its Max.",
                );
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(valid, egui::Button::new("Load")).clicked() {
                    load_clicked = true;
                }
            });
        });

    // Esc / a click outside the dialog also confirms if the range is valid (stays open when invalid).
    if load_clicked || (outcome.should_close && valid) {
        Some(CsvImportAction::Apply)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::state::app_state::{CsvImportSettings, Direction, StudyMeta};
    use std::collections::HashMap;

    fn make_meta() -> StudyMeta {
        let mut param_bounds = HashMap::new();
        param_bounds.insert("x".to_string(), (0.0, 10.0));
        param_bounds.insert("a".to_string(), (-1.0, 1.0));
        StudyMeta {
            study_id: 0,
            name: "data".to_string(),
            directions: vec![Direction::Minimize, Direction::Minimize],
            completed_trials: 3,
            param_names: vec!["a".to_string(), "x".to_string()],
            objective_names: vec!["f1".to_string(), "f2".to_string()],
            param_bounds,
        }
    }

    #[test]
    fn from_meta_sorts_bounds_and_defaults_to_minimize() {
        let s = CsvImportSettings::from_meta(&make_meta());
        assert_eq!(s.maximize, vec![false, false]);
        // Parameter names in ascending order.
        assert_eq!(s.param_bounds[0].name, "a");
        assert_eq!(s.param_bounds[1].name, "x");
        assert!(s.bounds_valid());
    }

    #[test]
    fn apply_to_overwrites_directions_and_bounds() {
        let mut s = CsvImportSettings::from_meta(&make_meta());
        s.maximize = vec![false, true];
        s.param_bounds[1].low = 2.0;
        s.param_bounds[1].high = 20.0;
        let mut meta = make_meta();
        s.apply_to(&mut meta);
        assert_eq!(meta.directions[1], Direction::Maximize);
        assert_eq!(meta.param_bounds["x"], (2.0, 20.0));
    }

    #[test]
    fn bounds_valid_rejects_inverted_or_nonfinite() {
        let mut s = CsvImportSettings::from_meta(&make_meta());
        s.param_bounds[0].low = 5.0;
        s.param_bounds[0].high = 1.0;
        assert!(!s.bounds_valid());
        s.param_bounds[0].high = f64::NAN;
        assert!(!s.bounds_valid());
    }
}
