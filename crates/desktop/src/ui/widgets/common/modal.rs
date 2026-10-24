//! Common scaffold for modal dialogs (D-4).
//!
//! Each modal (CSV import / license / RDB URL / report / trial detail) used to
//! duplicate `egui::Modal::new(Id::new(..)).show(ctx, |ui| { set min width; heading;
//! body })` along with a trailing `should_close()` check. This scaffold centralizes
//! that boilerplate in one place; each modal only needs to pass the ID, title, min
//! width (plus optional max width / min height), and a body closure.
//!
//! The final "close" condition combined with `should_close()` (validation, suppression
//! during generation, etc.) differs per modal, so the scaffold only returns
//! `should_close` as a [`ModalOutcome`].

/// The scaffold's draw result. Each modal records confirm actions (Load / Export /
/// Cancel, etc.) into mutable variables captured inside the body closure, so the
/// scaffold only needs to return `should_close`.
pub struct ModalOutcome {
    /// Whether the modal is being closed (background click, Esc, etc.).
    pub should_close: bool,
}

/// Builder for the modal scaffold. Give it an ID and min width via `new`, optionally
/// add a max width / min height / auto heading, then draw with `show`.
pub struct ModalScaffold<'a> {
    id: &'a str,
    min_width: f32,
    max_width: Option<f32>,
    min_height: Option<f32>,
    heading: Option<String>,
}

impl<'a> ModalScaffold<'a> {
    /// Creates the scaffold with the given ID and min width.
    pub fn new(id: &'a str, min_width: f32) -> Self {
        Self {
            id,
            min_width,
            max_width: None,
            min_height: None,
            heading: None,
        }
    }

    /// Sets the max width.
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = Some(w);
        self
    }

    /// Sets the min height.
    pub fn min_height(mut self, h: f32) -> Self {
        self.min_height = Some(h);
        self
    }

    /// Sets a heading to be drawn automatically at the top of the body. Omit this for
    /// modals with a custom header (e.g. trial detail, which places a heading + Close
    /// button on the same line) and draw it in the body instead.
    pub fn heading(mut self, title: impl Into<String>) -> Self {
        self.heading = Some(title.into());
        self
    }

    /// Draws the scaffold. Applies the configured min width, max width, min height,
    /// and heading, then calls `body` and returns `should_close`.
    pub fn show(self, ctx: &egui::Context, body: impl FnOnce(&mut egui::Ui)) -> ModalOutcome {
        let modal = egui::Modal::new(egui::Id::new(self.id)).show(ctx, |ui| {
            ui.set_min_width(self.min_width);
            if let Some(w) = self.max_width {
                ui.set_max_width(w);
            }
            if let Some(h) = self.min_height {
                ui.set_min_height(h);
            }
            if let Some(title) = &self.heading {
                ui.heading(title);
            }
            body(ui);
        });
        ModalOutcome {
            should_close: modal.should_close(),
        }
    }
}
