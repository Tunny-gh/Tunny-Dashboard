//! Confirmation shown before File > New discards the current work.
//!
//! `New` has no undo, and the canvas layout it throws away is only recoverable
//! from a saved session file, so the reset is confirmed whenever there is
//! something to lose. The caller decides whether that is the case
//! (`app::has_discardable_state`); this modal only asks.

use crate::ui::widgets::common::modal::ModalScaffold;

/// What the user chose in the confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewConfirmAction {
    /// Confirmed: reset to the empty state.
    Confirm,
    /// Dismissed (Cancel button, Esc, or a click outside the modal).
    Cancel,
}

/// Draws the confirmation modal.
///
/// Returns `None` while the user hasn't decided yet — keep calling it on the next
/// frame in that case.
pub fn show(ctx: &egui::Context) -> Option<NewConfirmAction> {
    let mut confirmed = false;
    let mut cancelled = false;

    let outcome = ModalScaffold::new("new_confirm_dialog", 380.0)
        .heading("New")
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label("Close the current file and start from an empty state?");
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "The canvas layout and widget settings are discarded as well. \
                     Save a session first if you want to keep them.",
                )
                .color(crate::theme::TEXT_SECONDARY()),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    confirmed = true;
                }
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
            });
        });

    if confirmed {
        Some(NewConfirmAction::Confirm)
    } else if cancelled || outcome.should_close {
        Some(NewConfirmAction::Cancel)
    } else {
        None
    }
}
