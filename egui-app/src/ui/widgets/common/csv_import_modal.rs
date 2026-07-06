//! CSV インポート確認ダイアログ。
//!
//! フラット CSV には最適化方向（最大化/最小化）や変数の宣言レンジが含まれないため、
//! 読み込み直後にこのモーダルで観測値由来の既定値を提示し、ユーザーが確認・修正できる
//! ようにする。確定すると編集値が `StudyMeta` に反映され、その方向で Pareto ランクが、
//! そのレンジでサロゲート最適化の探索箱が決まる。

use egui::RichText;

use crate::state::app_state::CsvImportSettings;

/// ダイアログの操作結果。
pub enum CsvImportAction {
    /// 現在の編集値で Study を読み込む。
    Apply,
}

/// CSV インポート確認モーダルを描画する。
///
/// 確定された場合のみ `Some(CsvImportAction::Apply)` を返す。返り値が `None` の間は
/// ダイアログを開いたままにする。Esc / 背景クリックでもレンジが有効なら確定する。
pub fn show(ctx: &egui::Context, settings: &mut CsvImportSettings) -> Option<CsvImportAction> {
    let mut load_clicked = false;
    let valid = settings.bounds_valid();

    let modal = egui::Modal::new(egui::Id::new("csv_import_settings_modal")).show(ctx, |ui| {
        ui.set_min_width(440.0);
        ui.heading("CSV Import Settings");
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

        // ── 目的の最適化方向 ──────────────────────────────────
        ui.label(RichText::new("Objective Directions").strong());
        egui::Grid::new("csv_import_directions")
            .num_columns(2)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for (i, name) in settings.objective_names.iter().enumerate() {
                    ui.label(name);
                    let is_max = &mut settings.maximize[i];
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

        // ── 数値パラメータのレンジ ────────────────────────────
        ui.label(RichText::new("Parameter Ranges").strong());
        if settings.param_bounds.is_empty() {
            ui.label(RichText::new("No numeric parameters.").color(crate::theme::TEXT_SECONDARY()));
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
                        // 観測幅の 1% を 1 ステップとし、ダブルクリックで直接入力もできる。
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

    // Esc / 背景クリックでもレンジが有効なら確定する（無効時は開いたまま）。
    if load_clicked || (modal.should_close() && valid) {
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
        // パラメータ名昇順。
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
