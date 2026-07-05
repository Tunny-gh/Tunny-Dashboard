//! 「Open URL…」ダイアログ: PostgreSQL/MySQL 接続 URL を直接入力してストレージを開く。
//!
//! ローカルファイルのみを想定する `rfd::FileDialog`（"Open" ボタン）では RDB 接続 URL を
//! 選べないため、専用の 1 行入力モーダルを別途用意する。パース結果（成否・正規化済み
//! 文字列）を計算する部分は `resolve_open_url` として UI から切り離してあり、
//! egui コンテキスト無しでテストできる。

use egui::RichText;

use tunny_core::rdb::RdbUrl;

/// ダイアログの操作結果。
pub enum RdbUrlDialogAction {
    /// パース済み・正規化済みの URL 文字列で開く。
    Open(String),
    /// ダイアログを閉じる（何も開かない）。
    Cancel,
}

/// 入力文字列をパースし、"Open" 可能かどうかと正規化済み URL 文字列を返す純関数。
/// 空文字列や `postgresql://`/`mysql://` 以外のスキームは `Err` になる。
pub fn resolve_open_url(input: &str) -> Result<String, &'static str> {
    RdbUrl::parse(input.trim())
        .map(|url| url.url)
        .ok_or("Unsupported URL. Use postgresql:// or mysql://")
}

/// 「Open URL…」モーダルを描画する。
///
/// 戻り値が `None` の間はダイアログを開いたままにする（次フレームも `input` を保持して
/// 呼び直すこと）。`Some(Open(..))` / `Some(Cancel)` が返ったらダイアログを閉じてよい。
pub fn show(ctx: &egui::Context, input: &mut String) -> Option<RdbUrlDialogAction> {
    let mut open_clicked = false;
    let mut cancel_clicked = false;

    let modal = egui::Modal::new(egui::Id::new("rdb_url_dialog")).show(ctx, |ui| {
        ui.set_min_width(440.0);
        ui.heading("Open Database URL");
        ui.add_space(4.0);
        ui.label(
            RichText::new("Connect directly to an Optuna RDBStorage (PostgreSQL/MySQL).")
                .color(crate::theme::TEXT_SECONDARY),
        );
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(input)
                .hint_text("postgresql://user:pass@host:5432/dbname")
                .desired_width(ui.available_width()),
        );

        let parsed = resolve_open_url(input);
        if let Err(msg) = &parsed {
            if !input.trim().is_empty() {
                ui.add_space(4.0);
                ui.colored_label(crate::theme::ERROR_COLOR, *msg);
            }
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(parsed.is_ok(), egui::Button::new("Open"))
                .clicked()
            {
                open_clicked = true;
            }
            if ui.button("Cancel").clicked() {
                cancel_clicked = true;
            }
        });
    });

    if open_clicked {
        // ボタンは parsed.is_ok() のときのみ有効なので、ここでの再パース失敗は起こらない想定。
        resolve_open_url(input).ok().map(RdbUrlDialogAction::Open)
    } else if cancel_clicked || modal.should_close() {
        Some(RdbUrlDialogAction::Cancel)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_open_url_accepts_postgresql_and_mysql() {
        assert_eq!(
            resolve_open_url("postgresql://user:pass@localhost:5432/db"),
            Ok("postgresql://user:pass@localhost:5432/db".to_string())
        );
        assert_eq!(
            resolve_open_url("mysql://user:pass@localhost:3306/db"),
            Ok("mysql://user:pass@localhost:3306/db".to_string())
        );
    }

    #[test]
    fn resolve_open_url_normalizes_short_scheme_and_driver_suffix() {
        assert_eq!(
            resolve_open_url("postgres://u:p@h/db"),
            Ok("postgresql://u:p@h/db".to_string())
        );
        assert_eq!(
            resolve_open_url("postgresql+psycopg2://u:p@h/db"),
            Ok("postgresql://u:p@h/db".to_string())
        );
    }

    #[test]
    fn resolve_open_url_trims_whitespace() {
        assert_eq!(
            resolve_open_url("  postgresql://u:p@h/db  "),
            Ok("postgresql://u:p@h/db".to_string())
        );
    }

    #[test]
    fn resolve_open_url_rejects_unsupported_scheme() {
        assert_eq!(
            resolve_open_url("sqlite:///a.db"),
            Err("Unsupported URL. Use postgresql:// or mysql://")
        );
        assert_eq!(
            resolve_open_url(""),
            Err("Unsupported URL. Use postgresql:// or mysql://")
        );
        assert_eq!(
            resolve_open_url("/local/path/study.log"),
            Err("Unsupported URL. Use postgresql:// or mysql://")
        );
    }
}
