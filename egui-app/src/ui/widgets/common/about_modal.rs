//! The About modal: version, the beta notice, and the way into the licenses.
//!
//! This is where the startup notice stays readable after the user has ticked
//! "Don't show this again" — the wording is shared with
//! [`crate::ui::widgets::beta_notice_modal`] rather than duplicated. It replaces the
//! former toolbar "Licenses" button, which is now reached from here.

use egui::RichText;

use crate::licenses::{APP_REPOSITORY, APP_VERSION};
use crate::ui::widgets::common::beta_notice_modal::{
    BETA_NOTICE_BODY, BETA_NOTICE_HEADING, LICENSE_NOTICE,
};
use crate::ui::widgets::common::modal::ModalScaffold;

/// UI state for the About modal.
#[derive(Default)]
pub struct AboutModalState {
    /// Whether the modal is currently shown.
    pub open: bool,
}

/// Renders the About modal. Returns true when the user asked for the open-source
/// license list, which the caller opens.
pub fn show(ctx: &egui::Context, state: &mut AboutModalState) -> bool {
    if !state.open {
        return false;
    }

    let mut open_licenses = false;
    let outcome = ModalScaffold::new("about_modal", 420.0)
        .max_width(520.0)
        .heading("About Tunny Dashboard")
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Version {APP_VERSION}")).strong());
            ui.hyperlink_to(APP_REPOSITORY, APP_REPOSITORY);
            ui.separator();

            ui.label(RichText::new(BETA_NOTICE_HEADING).strong());
            ui.add_space(4.0);
            ui.label(BETA_NOTICE_BODY);
            ui.add_space(8.0);
            ui.label(RichText::new(LICENSE_NOTICE).color(crate::theme::TEXT_SECONDARY()));
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui.button("📄 Open Source Licenses").clicked() {
                    open_licenses = true;
                }
                if ui.button("Close").clicked() {
                    state.open = false;
                }
            });
        });

    if open_licenses || outcome.should_close {
        state.open = false;
    }
    open_licenses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_closed() {
        assert!(!AboutModalState::default().open);
    }
}
