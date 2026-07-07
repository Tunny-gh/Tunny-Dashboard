//! 「Open URL…」ダイアログ: PostgreSQL/MySQL 接続 URL を直接入力してストレージを開く。
//!
//! ローカルファイルのみを想定する `rfd::FileDialog`（"Open" ボタン）では RDB 接続 URL を
//! 選べないため、専用の 1 行入力モーダルを別途用意する。パース・TLS 事前チェックの
//! 判定部分は `classify_input` として UI から切り離してあり、egui コンテキスト無しで
//! テストできる。
//!
//! バックエンドは TLS 未対応（平文接続のみ）のため、接続してからエラーで知るのでは
//! なく、入力中に次を通知する:
//! - `sslmode=disable` の明示が無い URL → 「この接続は暗号化されない」旨の警告
//!   （ループバック等、接続自体は可能なケース）
//! - 非ローカルホストへ opt-in 無し、または `sslmode=require` 等 → 接続時に必ず
//!   拒否されるため、その理由を表示して Open を無効化

use egui::RichText;

use tunny_core::rdb::{check_tls_precondition, has_explicit_plaintext_optin, RdbUrl};

use crate::ui::widgets::common::modal::ModalScaffold;

/// ダイアログの操作結果。
pub enum RdbUrlDialogAction {
    /// パース済み・正規化済みの URL 文字列で開く。
    Open(String),
    /// ダイアログを閉じる（何も開かない）。
    Cancel,
}

/// 入力 URL の判定結果（表示メッセージと Open 可否を決める）。
#[derive(Debug, PartialEq, Eq)]
pub enum RdbUrlCheck {
    /// スキーム不正・空入力など、RDB URL として解釈できない。
    Invalid,
    /// パースは通るが、TLS 事前チェックにより接続時に必ず拒否される
    /// （非ローカルホストへの opt-in 無し平文、`sslmode=require` 等）。
    /// `reason` は接続時に返るものと同じ説明文。
    Blocked { reason: String },
    /// 接続可能だが `sslmode=disable` の明示が無い平文接続
    /// （ループバック接続の無指定など）。暗号化されない旨を通知する。
    PlaintextImplicit { url: String },
    /// `sslmode=disable` 明示済みの平文接続。ユーザーが了解済みのため通知しない。
    PlaintextExplicit { url: String },
}

/// 入力文字列をパース・正規化し、TLS 事前チェックまで含めて分類する純関数。
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

/// 「Open URL…」モーダルを描画する。
///
/// 戻り値が `None` の間はダイアログを開いたままにする（次フレームも `input` を保持して
/// 呼び直すこと）。`Some(Open(..))` / `Some(Cancel)` が返ったらダイアログを閉じてよい。
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
        // ループバック + 無指定は接続可能だが「暗号化されない」通知の対象になる。
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
        // 非ローカルホストも opt-in 明示済みなら接続可能・通知なし。
        assert!(matches!(
            classify_input("mysql://u:p@db.example.com/db?ssl-mode=disable"),
            RdbUrlCheck::PlaintextExplicit { .. }
        ));
    }

    #[test]
    fn classify_blocks_urls_rejected_by_tls_precondition() {
        // 非ローカルホストへの opt-in 無し平文は接続時に必ず拒否されるため Blocked。
        let RdbUrlCheck::Blocked { reason } = classify_input("postgresql://u:p@db.example.com/db")
        else {
            panic!("remote without sslmode must be Blocked");
        };
        assert!(reason.contains("sslmode=disable"), "reason: {reason}");

        // sslmode=require は TLS 未対応のため Blocked。
        assert!(matches!(
            classify_input("postgresql://u:p@localhost/db?sslmode=require"),
            RdbUrlCheck::Blocked { .. }
        ));
    }
}
