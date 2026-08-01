//! The open-source license display modal.
//!
//! Lists the licenses (SPDX and full license text) of the dependency crates bundled in
//! the distributed binary. Uses [`crate::licenses::LICENSES`], the data collected by
//! `build.rs`. Tunny Dashboard's own MIT License is listed first, from the `LICENSE`
//! file embedded at build time — the beta notice points at it, so it has to be
//! readable without leaving the app.

use egui::RichText;

use crate::licenses::{LicenseEntry, APP_REPOSITORY, APP_VERSION, LICENSES};
use crate::ui::widgets::common::modal::ModalScaffold;

/// This application's own license, embedded from the repository root `LICENSE`.
static APP_ENTRY: LicenseEntry = LicenseEntry {
    name: "Tunny Dashboard",
    version: APP_VERSION,
    license: "MIT",
    repository: APP_REPOSITORY,
    text: include_str!("../../../../../LICENSE"),
};

/// UI state for the license modal.
#[derive(Default)]
pub struct LicenseModalState {
    /// Whether the modal is currently shown.
    pub open: bool,
    /// The filter string for crate name / license kind.
    pub search: String,
}

/// Renders the license modal. Only shown when `state.open` is true; closes on Esc /
/// background click / the Close button.
pub fn show(ctx: &egui::Context, state: &mut LicenseModalState) {
    if !state.open {
        return;
    }

    let outcome = ModalScaffold::new("oss_license_modal", 560.0)
        .max_width(720.0)
        .heading("Open Source Licenses")
        .show(ctx, |ui| {
            ui.label(
                RichText::new(format!(
                    "Tunny Dashboard itself is MIT licensed, and bundles {} third-party crates.",
                    LICENSES.len()
                ))
                .color(crate::theme::TEXT_SECONDARY()),
            );
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut state.search);
                if !state.search.is_empty() && ui.button("✖").clicked() {
                    state.search.clear();
                }
            });
            ui.separator();

            let needle = state.search.trim().to_lowercase();
            // The app's own entry leads the list; the dependencies follow in the
            // order build.rs collected them.
            let filtered: Vec<&LicenseEntry> = std::iter::once(&APP_ENTRY)
                .chain(LICENSES.iter())
                .filter(|e| matches_filter(e, &needle))
                .collect();

            // Cap the modal height relative to the viewport, and let a long list scroll.
            let max_h = ctx.content_rect().height() * 0.6;
            egui::ScrollArea::vertical()
                .max_height(max_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if filtered.is_empty() {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("No crates match the filter.")
                                .color(crate::theme::TEXT_SECONDARY()),
                        );
                        return;
                    }
                    for entry in filtered {
                        show_entry(ui, entry);
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    state.open = false;
                }
            });
        });

    if outcome.should_close {
        state.open = false;
    }
}

/// Whether the entry matches the filter (targets crate name / license kind; matches
/// everything if empty).
fn matches_filter(entry: &LicenseEntry, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    entry.name.to_lowercase().contains(needle) || entry.license.to_lowercase().contains(needle)
}

/// Renders one crate's entry as a collapsing header.
fn show_entry(ui: &mut egui::Ui, entry: &LicenseEntry) {
    let license = if entry.license.is_empty() {
        "(license not specified)"
    } else {
        entry.license
    };
    let header = format!("{} {}  —  {}", entry.name, entry.version, license);

    egui::CollapsingHeader::new(RichText::new(header).strong())
        .id_salt(("license_entry", entry.name, entry.version))
        .show(ui, |ui| {
            if !entry.repository.is_empty() {
                ui.hyperlink_to(entry.repository, entry.repository);
            }
            if entry.text.is_empty() {
                ui.label(
                    RichText::new(
                        "No license file was bundled with this crate. \
                         Refer to the SPDX identifier above and the repository.",
                    )
                    .color(crate::theme::TEXT_SECONDARY()),
                );
            } else {
                ui.add_space(4.0);
                // The full license text, monospace and selectable.
                ui.add(
                    egui::Label::new(RichText::new(entry.text).monospace().size(11.0))
                        .selectable(true),
                );
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> LicenseEntry {
        LicenseEntry {
            name: "serde",
            version: "1.0.0",
            license: "MIT OR Apache-2.0",
            repository: "https://github.com/serde-rs/serde",
            text: "MIT License...",
        }
    }

    #[test]
    fn empty_filter_matches_all() {
        assert!(matches_filter(&entry(), ""));
    }

    #[test]
    fn filter_matches_name() {
        assert!(matches_filter(&entry(), "serd"));
    }

    #[test]
    fn filter_matches_license() {
        assert!(matches_filter(&entry(), "apache"));
    }

    #[test]
    fn filter_rejects_non_match() {
        assert!(!matches_filter(&entry(), "zzz_nonexistent"));
    }

    #[test]
    fn app_entry_carries_the_embedded_mit_license() {
        assert_eq!(APP_ENTRY.license, "MIT");
        assert!(APP_ENTRY.text.contains("MIT License"));
        assert!(APP_ENTRY.text.contains("WITHOUT WARRANTY OF ANY KIND"));
    }

    #[test]
    fn default_state_is_closed() {
        let s = LicenseModalState::default();
        assert!(!s.open);
        assert!(s.search.is_empty());
    }
}
