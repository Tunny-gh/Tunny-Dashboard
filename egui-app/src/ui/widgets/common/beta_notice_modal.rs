//! The startup notice telling the user this build is a beta.
//!
//! Shown once per version: dismissing it with "Don't show this again" records the
//! current [`crate::licenses::APP_VERSION`] in eframe storage, and the notice stays
//! hidden until the app is upgraded. The wording lives here as
//! [`BETA_NOTICE_HEADING`] / [`BETA_NOTICE_BODY`] / [`LICENSE_NOTICE`] so the About
//! modal ([`crate::ui::widgets::about_modal`]) can present exactly the same text.

use egui::RichText;

use crate::licenses::APP_VERSION;
use crate::ui::widgets::common::modal::ModalScaffold;

/// Heading of the beta notice.
pub const BETA_NOTICE_HEADING: &str = "Tunny Dashboard is in beta";

/// The main body of the beta notice.
pub const BETA_NOTICE_BODY: &str = "This version is still under active development. \
     Features, layouts, and analysis results may change or contain errors without notice.";

/// The license and warranty sentence. The detailed disclaimer is the MIT License
/// itself, which is readable in full from the Licenses modal.
pub const LICENSE_NOTICE: &str = "Tunny Dashboard is open source software released under \
     the MIT License and is provided \"AS IS\", without warranty of any kind. Please verify \
     important results independently and keep backups of your Optuna storage.";

/// UI state for the beta notice modal.
#[derive(Default)]
pub struct BetaNoticeState {
    /// Whether the modal is currently shown.
    pub open: bool,
    /// The state of the "Don't show this again" checkbox. Read when the modal closes,
    /// no matter which way it was closed.
    pub dont_show_again: bool,
}

/// How the user left the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BetaNoticeDismissal {
    /// The checkbox was ticked: record this version and stop showing the notice.
    Suppress,
    /// Closed without ticking the checkbox: show it again on the next launch.
    ShowAgain,
}

/// Whether the notice should be shown on startup.
///
/// `ack` is the version recorded the last time the user ticked "Don't show this again".
/// The comparison is an exact match, so downgrading to an older build shows the notice
/// again — which is the intent, since that build's caveats differ.
pub fn should_show(ack: Option<&str>, current: &str) -> bool {
    ack != Some(current)
}

/// Draws the notice.
///
/// Returns `None` while it is still open. Esc and a background click close it just
/// like every other modal; whichever way it closes, the checkbox state decides the
/// returned dismissal.
pub fn show(ctx: &egui::Context, state: &mut BetaNoticeState) -> Option<BetaNoticeDismissal> {
    if !state.open {
        return None;
    }

    let mut continue_clicked = false;
    let outcome = ModalScaffold::new("beta_notice_modal", 440.0)
        .max_width(520.0)
        .heading(BETA_NOTICE_HEADING)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.label(BETA_NOTICE_BODY);
            ui.add_space(8.0);
            ui.label(RichText::new(LICENSE_NOTICE).color(crate::theme::TEXT_SECONDARY()));
            ui.add_space(12.0);
            ui.checkbox(
                &mut state.dont_show_again,
                format!("Don't show this again for v{APP_VERSION}"),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Continue").clicked() {
                    continue_clicked = true;
                }
            });
        });

    if !continue_clicked && !outcome.should_close {
        return None;
    }
    state.open = false;
    Some(if state.dont_show_again {
        BetaNoticeDismissal::Suppress
    } else {
        BetaNoticeDismissal::ShowAgain
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_when_nothing_acknowledged() {
        assert!(should_show(None, "0.1.0"));
    }

    #[test]
    fn hides_when_current_version_acknowledged() {
        assert!(!should_show(Some("0.1.0"), "0.1.0"));
    }

    #[test]
    fn shows_again_after_version_change() {
        assert!(should_show(Some("0.1.0"), "0.2.0"));
        // Downgrades count as a change too.
        assert!(should_show(Some("0.2.0"), "0.1.0"));
    }

    #[test]
    fn default_state_is_closed_and_unchecked() {
        let s = BetaNoticeState::default();
        assert!(!s.open);
        assert!(!s.dont_show_again);
    }
}
