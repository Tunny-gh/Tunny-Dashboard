//! 「Report…」ダイアログ: 自己完結型レポート（HTML / Markdown / JSON）の出力設定。
//!
//! フォーマット選択・言語・Top-N をここで確定し、Export を押すと呼び出し側
//! （`app.rs`）が rfd の保存ダイアログでベースパスを選ばせ、バックグラウンド
//! スレッドで `tunny_core::report::build_study_report` を実行する
//! （`crate::io::report_export::spawn_report_export`）。生成中/完了/失敗の状態は
//! この `ReportDialogState` 自体が持つため、egui コンテキスト無しでも純粋な
//! ロジック（検証・ファイル名導出）をテストできる。

use std::path::{Path, PathBuf};

use egui::RichText;
use tunny_core::report::ReportLang;

/// エクスポート可能なレポート形式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportFormat {
    Html,
    Markdown,
    Json,
}

impl ReportFormat {
    /// ファイル拡張子（先頭ドット無し）。
    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Html => "html",
            ReportFormat::Markdown => "md",
            ReportFormat::Json => "json",
        }
    }

    /// 表示ラベル。
    pub fn label(self) -> &'static str {
        match self {
            ReportFormat::Html => "HTML",
            ReportFormat::Markdown => "Markdown",
            ReportFormat::Json => "JSON",
        }
    }
}

/// 「Report…」モーダルの編集状態。`AppState.report_dialog` が `Some` の間表示する。
#[derive(Debug, Clone)]
pub struct ReportDialogState {
    pub html: bool,
    pub markdown: bool,
    pub json: bool,
    pub lang: ReportLang,
    /// 上位表の件数（`ReportOptions::top_n` にそのまま渡す）。
    pub top_n: usize,
    /// フォーマット未選択などのバリデーションエラー、または生成失敗時のエラー。
    pub error: Option<String>,
    /// バックグラウンドで生成中か（true の間は入力を無効化する）。
    pub generating: bool,
    /// 生成完了後に書き出したファイルパス一覧。`Some` の間は完了表示に切り替える。
    pub success_paths: Option<Vec<PathBuf>>,
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
        }
    }
}

impl ReportDialogState {
    /// チェック済みフォーマットの一覧を返す。1 件も選択されていなければ `Err`。
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

/// 既定のファイル名（`report_{study_name}.html`）。study 名はファイル名に
/// 使えない文字を `_` に置換する。
pub fn default_file_name(study_name: &str) -> String {
    format!("report_{}.html", sanitize_file_stem(study_name))
}

/// ファイル名として安全な文字（英数字・`-`・`_`）以外を `_` に置換する。
/// 全文字が置換対象だった場合は `"study"` を返す。
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

/// ユーザーが選んだベースパスから、選択フォーマットぶんの拡張子違いパスを導出する。
/// 例: base=`report_x.html`, formats=[Html, Json] → `report_x.html`, `report_x.json`。
pub fn export_paths(base_path: &Path, formats: &[ReportFormat]) -> Vec<(ReportFormat, PathBuf)> {
    formats
        .iter()
        .map(|&fmt| (fmt, base_path.with_extension(fmt.extension())))
        .collect()
}

/// ダイアログの操作結果。
pub enum ReportModalAction {
    /// Export ボタン押下（形式検証は呼び出し側 `app.rs` が行う）。
    Export,
    /// Cancel / Close ボタン押下、またはモーダル外クリック。
    Close,
}

/// 「Report…」モーダルを描画する。
///
/// 戻り値が `None` の間はダイアログを開いたままにする。`generating` 中は
/// 入力・Export ボタンを無効化し、`success_paths` が `Some` になったら
/// 完了メッセージ＋Close ボタンに切り替える。
pub fn show(
    ctx: &egui::Context,
    state: &mut ReportDialogState,
    study_name: Option<&str>,
) -> Option<ReportModalAction> {
    let mut export_clicked = false;
    let mut close_clicked = false;

    let modal = egui::Modal::new(egui::Id::new("report_export_dialog")).show(ctx, |ui| {
        ui.set_min_width(380.0);
        ui.heading("Export Report");
        if let Some(name) = study_name {
            ui.label(RichText::new(format!("Study: {name}")).color(crate::theme::TEXT_SECONDARY));
        }
        ui.add_space(4.0);

        if let Some(paths) = &state.success_paths {
            for path in paths {
                ui.colored_label(
                    crate::theme::TEXT_SECONDARY,
                    format!("Saved: {}", path.display()),
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
            ui.colored_label(crate::theme::ERROR_COLOR, err);
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
    } else if close_clicked || (modal.should_close() && !state.generating) {
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
        assert_eq!(default_file_name("my study/01"), "report_my_study_01.html");
        assert_eq!(default_file_name("safe-name_2"), "report_safe-name_2.html");
    }

    #[test]
    fn default_file_name_falls_back_when_fully_sanitized() {
        assert_eq!(default_file_name("///"), "report_study.html");
        assert_eq!(default_file_name(""), "report_study.html");
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
