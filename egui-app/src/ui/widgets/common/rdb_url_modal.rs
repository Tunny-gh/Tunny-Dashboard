//! "Open URL…" dialog: opens storage by directly entering a PostgreSQL/MySQL
//! connection URL.
//!
//! `rfd::FileDialog` (the "Open" button), which only handles local files, cannot
//! select an RDB connection URL, so a dedicated single-line-input modal is provided
//! separately. The parsing and TLS-precondition-check logic is factored out of the UI
//! as `classify_input`, so it can be tested without an egui context.
//!
//! Since the backend doesn't support TLS (plaintext connections only), rather than
//! finding out via an error after connecting, the following is reported while typing:
//! - A URL without an explicit `sslmode=disable` -> a warning that "this connection
//!   will not be encrypted" (cases where the connection itself is still possible,
//!   e.g. loopback)
//! - A non-local host without opt-in, or `sslmode=require`, etc. -> the connection
//!   will always be rejected, so the reason is shown and Open is disabled

use egui::RichText;

use tunny_core::rdb::{check_tls_precondition, has_explicit_plaintext_optin, RdbUrl};

use crate::ui::widgets::common::modal::ModalScaffold;

/// The dialog's action result.
pub enum RdbUrlDialogAction {
    /// Open with the parsed and normalized URL string.
    Open(String),
    /// Close the dialog (open nothing).
    Cancel,
}

/// The classification result for the input URL (determines the displayed message and
/// whether Open is enabled).
#[derive(Debug, PartialEq, Eq)]
pub enum RdbUrlCheck {
    /// Cannot be interpreted as an RDB URL (invalid scheme, empty input, etc.).
    Invalid,
    /// Parses fine, but the TLS precondition check means the connection will always
    /// be rejected (plaintext to a non-local host without opt-in, `sslmode=require`,
    /// etc.). `reason` is the same explanation text returned at connection time.
    Blocked { reason: String },
    /// Connectable, but a plaintext connection without an explicit
    /// `sslmode=disable` (e.g. an unspecified loopback connection). Reports that it
    /// will not be encrypted.
    PlaintextImplicit { url: String },
    /// A plaintext connection with `sslmode=disable` explicitly set. Not reported
    /// since the user has already acknowledged it.
    PlaintextExplicit { url: String },
}

/// A pure function that parses and normalizes the input string, classifying it
/// including the TLS precondition check.
pub fn classify_input(input: &str) -> RdbUrlCheck {
    let Some(url) = RdbUrl::parse(input.trim()) else {
        return RdbUrlCheck::Invalid;
    };
    if let Err(reason) = check_tls_precondition(&url.url) {
        return RdbUrlCheck::Blocked { reason };
    }
    if has_explicit_plaintext_optin(&url.url) {
        RdbUrlCheck::PlaintextExplicit { url: url.url }
    } else {
        RdbUrlCheck::PlaintextImplicit { url: url.url }
    }
}

/// Draws the "Open URL…" modal.
///
/// Keep the dialog open while the return value is `None` (call again next frame,
/// keeping `input` around). Once `Some(Open(..))` / `Some(Cancel)` is returned, the
/// dialog may be closed.
pub fn show(ctx: &egui::Context, input: &mut String) -> Option<RdbUrlDialogAction> {
    let mut open_url: Option<String> = None;
    let mut cancel_clicked = false;

    let outcome = ModalScaffold::new("rdb_url_dialog", 440.0)
        .heading("Open Database URL")
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Connect directly to an Optuna RDBStorage (PostgreSQL/MySQL).")
                    .color(crate::theme::TEXT_SECONDARY()),
            );
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(input)
                    .hint_text("postgresql://user:pass@host:5432/dbname")
                    .desired_width(ui.available_width()),
            );

            let check = classify_input(input);
            let openable_url: Option<&str> = match &check {
                RdbUrlCheck::Invalid => {
                    if !input.trim().is_empty() {
                        ui.add_space(4.0);
                        ui.colored_label(
                            crate::theme::ERROR_COLOR(),
                            "Unsupported URL. Use postgresql:// or mysql://",
                        );
                    }
                    None
                }
                RdbUrlCheck::Blocked { reason } => {
                    ui.add_space(4.0);
                    ui.colored_label(crate::theme::ERROR_COLOR(), reason);
                    None
                }
                RdbUrlCheck::PlaintextImplicit { url } => {
                    ui.add_space(4.0);
                    ui.colored_label(
                        crate::theme::WARNING_COLOR(),
                        "⚠ This connection will not be encrypted (TLS is not supported). \
                         Add sslmode=disable to acknowledge and hide this notice.",
                    );
                    Some(url.as_str())
                }
                RdbUrlCheck::PlaintextExplicit { url } => Some(url.as_str()),
            };

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(openable_url.is_some(), egui::Button::new("Open"))
                    .clicked()
                {
                    open_url = openable_url.map(str::to_owned);
                }
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
            });
        });

    if let Some(url) = open_url {
        Some(RdbUrlDialogAction::Open(url))
    } else if cancel_clicked || outcome.should_close {
        Some(RdbUrlDialogAction::Cancel)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_url(check: RdbUrlCheck) -> String {
        match check {
            RdbUrlCheck::PlaintextImplicit { url } | RdbUrlCheck::PlaintextExplicit { url } => url,
            other => panic!("expected openable URL, got {other:?}"),
        }
    }

    #[test]
    fn classify_accepts_postgresql_and_mysql_loopback() {
        assert_eq!(
            expect_url(classify_input("postgresql://user:pass@localhost:5432/db")),
            "postgresql://user:pass@localhost:5432/db"
        );
        assert_eq!(
            expect_url(classify_input("mysql://user:pass@localhost:3306/db")),
            "mysql://user:pass@localhost:3306/db"
        );
    }

    #[test]
    fn classify_normalizes_short_scheme_and_driver_suffix() {
        assert_eq!(
            expect_url(classify_input("postgres://u:p@localhost/db")),
            "postgresql://u:p@localhost/db"
        );
        assert_eq!(
            expect_url(classify_input("postgresql+psycopg2://u:p@localhost/db")),
            "postgresql://u:p@localhost/db"
        );
    }

    #[test]
    fn classify_trims_whitespace() {
        assert_eq!(
            expect_url(classify_input("  postgresql://u:p@localhost/db  ")),
            "postgresql://u:p@localhost/db"
        );
    }

    #[test]
    fn classify_rejects_unsupported_scheme() {
        assert_eq!(classify_input("sqlite:///a.db"), RdbUrlCheck::Invalid);
        assert_eq!(classify_input(""), RdbUrlCheck::Invalid);
        assert_eq!(
            classify_input("/local/path/study.log"),
            RdbUrlCheck::Invalid
        );
    }

    #[test]
    fn classify_notifies_when_sslmode_disable_is_not_explicit() {
        // Loopback + unspecified is connectable but subject to the "not encrypted" notice.
        assert!(matches!(
            classify_input("postgresql://u:p@localhost/db"),
            RdbUrlCheck::PlaintextImplicit { .. }
        ));
        assert!(matches!(
            classify_input("mysql://u:p@127.0.0.1:3306/db"),
            RdbUrlCheck::PlaintextImplicit { .. }
        ));
    }

    #[test]
    fn classify_skips_notice_when_sslmode_disable_is_explicit() {
        assert!(matches!(
            classify_input("postgresql://u:p@localhost/db?sslmode=disable"),
            RdbUrlCheck::PlaintextExplicit { .. }
        ));
        // A non-local host is also connectable with no notice once opt-in is explicit.
        assert!(matches!(
            classify_input("mysql://u:p@db.example.com/db?ssl-mode=disable"),
            RdbUrlCheck::PlaintextExplicit { .. }
        ));
    }

    #[test]
    fn classify_blocks_urls_rejected_by_tls_precondition() {
        // Plaintext to a non-local host without opt-in is always rejected at connect time, so Blocked.
        let RdbUrlCheck::Blocked { reason } = classify_input("postgresql://u:p@db.example.com/db")
        else {
            panic!("remote without sslmode must be Blocked");
        };
        assert!(reason.contains("sslmode=disable"), "reason: {reason}");

        // sslmode=require is Blocked since TLS is unsupported.
        assert!(matches!(
            classify_input("postgresql://u:p@localhost/db?sslmode=require"),
            RdbUrlCheck::Blocked { .. }
        ));
    }
}
