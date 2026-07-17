//! The "Report…" dialog: output settings for self-contained reports (HTML / Markdown / JSON).
//!
//! Format selection, language, and Top-N are finalized here, and clicking
//! Export lets the caller (`app.rs`) choose a base path via an rfd save
//! dialog, then runs `tunny_core::report::build_study_report` on a
//! background thread (`crate::io::report_export::spawn_report_export`).
//! The generating/complete/failed state is held by `ReportDialogState`
//! itself, so the pure logic (validation, file name derivation) can be
//! tested without an egui context.

use std::path::{Path, PathBuf};

use egui::RichText;
use tunny_core::report::ReportLang;

use crate::ui::widgets::common::modal::ModalScaffold;

/// Exportable report formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportFormat {
    Html,
    Markdown,
    Json,
}

impl ReportFormat {
    /// File extension (without leading dot).
    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Html => "html",
            ReportFormat::Markdown => "md",
            ReportFormat::Json => "json",
        }
    }

    /// Display label.
    pub fn label(self) -> &'static str {
        match self {
            ReportFormat::Html => "HTML",
            ReportFormat::Markdown => "Markdown",
            ReportFormat::Json => "JSON",
        }
    }
}

/// Editing state of the "Report…" modal. Displayed while `AppState.report_dialog` is `Some`.
#[derive(Debug, Clone)]
pub struct ReportDialogState {
    pub html: bool,
    pub markdown: bool,
    pub json: bool,
    pub lang: ReportLang,
    /// Number of top rows (passed directly to `ReportOptions::top_n`).
    pub top_n: usize,
    /// Validation error such as no format selected, or an error on generation failure.
    pub error: Option<String>,
    /// Whether generation is running in the background (input is disabled while true).
    pub generating: bool,
    /// The list of file paths written after generation completes. Switches to the
    /// completion display while `Some`.
    pub success_paths: Option<Vec<PathBuf>>,
    /// Non-primary sibling files that were silently overwritten
    /// (the primary is not included since it's already confirmed via the OS save dialog).
    pub overwrote_paths: Vec<PathBuf>,
}

impl Default for ReportDialogState {
    fn default() -> Self {
        Self {
            html: true,
            markdown: false,
            json: false,
            lang: ReportLang::En,
            top_n: 10,
            error: None,
            generating: false,
            success_paths: None,
            overwrote_paths: Vec::new(),
        }
    }
}

impl ReportDialogState {
    /// Returns the list of checked formats. `Err` if none are selected.
    pub fn selected_formats(&self) -> Result<Vec<ReportFormat>, &'static str> {
        let mut formats = Vec::new();
        if self.html {
            formats.push(ReportFormat::Html);
        }
        if self.markdown {
            formats.push(ReportFormat::Markdown);
        }
        if self.json {
            formats.push(ReportFormat::Json);
        }
        if formats.is_empty() {
            Err("Select at least one format (HTML / Markdown / JSON).")
        } else {
            Ok(formats)
        }
    }
}

/// The default file name (`report_{study_name}.{ext}`). The extension is derived from
/// the first of the selected formats (HTML if `formats` is empty). This is the entry
/// point that avoids emitting `.html` when only JSON / Markdown is selected, preserving
/// the OS save dialog's overwrite-confirmation invariant.
/// Characters unusable in a file name in the study name are replaced with `_`.
pub fn default_file_name_for(study_name: &str, formats: &[ReportFormat]) -> String {
    let ext = formats.first().map(|f| f.extension()).unwrap_or("html");
    format!("report_{}.{}", sanitize_file_stem(study_name), ext)
}

/// Replaces characters other than those safe for a file name (alphanumeric, `-`, `_`)
/// with `_`. Returns `"study"` if every character was subject to replacement.
fn sanitize_file_stem(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.trim_matches('_').is_empty() {
        "study".to_string()
    } else {
        sanitized
    }
}

/// Derives extension-varying paths for the selected formats from the base path chosen
/// by the user.
/// Example: base=`report_x.html`, formats=[Html, Json] → `report_x.html`, `report_x.json`.
pub fn export_paths(base_path: &Path, formats: &[ReportFormat]) -> Vec<(ReportFormat, PathBuf)> {
    formats
        .iter()
        .map(|&fmt| (fmt, base_path.with_extension(fmt.extension())))
        .collect()
}

/// The dialog's operation result.
pub enum ReportModalAction {
    /// The Export button was pressed (format validation is done by the caller, `app.rs`).
    Export,
    /// The Cancel / Close button was pressed, or a click occurred outside the modal.
    Close,
}

/// Draws the "Report…" modal.
///
/// The dialog stays open while the return value is `None`. While `generating`,
/// input and the Export button are disabled, and once `success_paths` becomes
/// `Some`, it switches to a completion message + Close button.
pub fn show(
    ctx: &egui::Context,
    state: &mut ReportDialogState,
    study_name: Option<&str>,
) -> Option<ReportModalAction> {
    let mut export_clicked = false;
    let mut close_clicked = false;

    let outcome = ModalScaffold::new("report_export_dialog", 380.0)
        .heading("Export Report")
        .show(ctx, |ui| {
            if let Some(name) = study_name {
                ui.label(
                    RichText::new(format!("Study: {name}")).color(crate::theme::TEXT_SECONDARY()),
                );
            }
            ui.add_space(4.0);

            if let Some(paths) = &state.success_paths {
                for path in paths {
                    ui.colored_label(
                        crate::theme::TEXT_SECONDARY(),
                        format!("Saved: {}", path.display()),
                    );
                }
                // Explicitly call out overwrites of sibling files that bypass the save dialog.
                for path in &state.overwrote_paths {
                    ui.colored_label(
                        egui::Color32::from_rgb(202, 138, 4), // amber-600
                        format!("Overwrote existing: {}", path.display()),
                    );
                }
                ui.add_space(8.0);
                if ui.button("Close").clicked() {
                    close_clicked = true;
                }
                return;
            }

            ui.add_enabled_ui(!state.generating, |ui| {
                ui.label("Formats:");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut state.html, ReportFormat::Html.label());
                    ui.checkbox(&mut state.markdown, ReportFormat::Markdown.label());
                    ui.checkbox(&mut state.json, ReportFormat::Json.label());
                });

                ui.add_space(4.0);
                ui.label("Language:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut state.lang, ReportLang::En, "En");
                    ui.selectable_value(&mut state.lang, ReportLang::Ja, "Ja");
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Top-N:");
                    ui.add(egui::DragValue::new(&mut state.top_n).range(1..=100));
                });
            });

            if state.generating {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Generating report...");
                });
            }

            if let Some(err) = &state.error {
                ui.add_space(4.0);
                ui.colored_label(crate::theme::ERROR_COLOR(), err);
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!state.generating, egui::Button::new("Export"))
                    .clicked()
                {
                    export_clicked = true;
                }
                if ui
                    .add_enabled(!state.generating, egui::Button::new("Cancel"))
                    .clicked()
                {
                    close_clicked = true;
                }
            });
        });

    if export_clicked {
        Some(ReportModalAction::Export)
    } else if close_clicked || (outcome.should_close && !state.generating) {
        Some(ReportModalAction::Close)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_html_only() {
        let state = ReportDialogState::default();
        assert!(state.html);
        assert!(!state.markdown);
        assert!(!state.json);
        assert_eq!(state.lang, ReportLang::En);
        assert_eq!(state.top_n, 10);
        assert!(state.error.is_none());
        assert!(!state.generating);
        assert!(state.success_paths.is_none());
    }

    #[test]
    fn selected_formats_rejects_empty_selection() {
        let mut state = ReportDialogState {
            html: false,
            ..Default::default()
        };
        assert!(state.selected_formats().is_err());
        state.markdown = true;
        assert_eq!(
            state.selected_formats().unwrap(),
            vec![ReportFormat::Markdown]
        );
    }

    #[test]
    fn selected_formats_preserves_html_md_json_order() {
        let state = ReportDialogState {
            html: true,
            markdown: true,
            json: true,
            ..Default::default()
        };
        assert_eq!(
            state.selected_formats().unwrap(),
            vec![
                ReportFormat::Html,
                ReportFormat::Markdown,
                ReportFormat::Json
            ]
        );
    }

    #[test]
    fn default_file_name_sanitizes_unsafe_characters() {
        let html = [ReportFormat::Html];
        assert_eq!(
            default_file_name_for("my study/01", &html),
            "report_my_study_01.html"
        );
        assert_eq!(
            default_file_name_for("safe-name_2", &html),
            "report_safe-name_2.html"
        );
    }

    #[test]
    fn default_file_name_falls_back_when_fully_sanitized() {
        let html = [ReportFormat::Html];
        assert_eq!(default_file_name_for("///", &html), "report_study.html");
        assert_eq!(default_file_name_for("", &html), "report_study.html");
    }

    #[test]
    fn default_file_name_for_derives_extension_from_first_format() {
        // .html is not emitted when only JSON / Markdown is selected.
        assert_eq!(
            default_file_name_for("s", &[ReportFormat::Json]),
            "report_s.json"
        );
        assert_eq!(
            default_file_name_for("s", &[ReportFormat::Markdown, ReportFormat::Json]),
            "report_s.md"
        );
        // An empty selection falls back to HTML.
        assert_eq!(default_file_name_for("s", &[]), "report_s.html");
    }

    #[test]
    fn export_paths_derives_sibling_extensions_from_base_path() {
        let base = PathBuf::from("/tmp/out/report_x.html");
        let formats = [
            ReportFormat::Html,
            ReportFormat::Markdown,
            ReportFormat::Json,
        ];
        let paths = export_paths(&base, &formats);
        assert_eq!(
            paths,
            vec![
                (ReportFormat::Html, PathBuf::from("/tmp/out/report_x.html")),
                (
                    ReportFormat::Markdown,
                    PathBuf::from("/tmp/out/report_x.md")
                ),
                (ReportFormat::Json, PathBuf::from("/tmp/out/report_x.json")),
            ]
        );
    }

    #[test]
    fn export_paths_replaces_extension_even_without_dot_in_base() {
        let base = PathBuf::from("report_x");
        let paths = export_paths(&base, &[ReportFormat::Json]);
        assert_eq!(
            paths,
            vec![(ReportFormat::Json, PathBuf::from("report_x.json"))]
        );
    }
}
