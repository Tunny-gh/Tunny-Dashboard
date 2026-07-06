//! オープンソースライセンス表示モーダル。
//!
//! 配布バイナリに含まれる依存クレートのライセンス（SPDX とライセンス全文）を
//! 一覧表示する。データは `build.rs` が収集した [`crate::licenses::LICENSES`] を使う。

use egui::RichText;

use crate::licenses::{LicenseEntry, LICENSES};

/// ライセンスモーダルの UI 状態。
#[derive(Default)]
pub struct LicenseModalState {
    /// モーダルを表示中か。
    pub open: bool,
    /// クレート名・ライセンス種別での絞り込み文字列。
    pub search: String,
}

/// ライセンスモーダルを描画する。`state.open` が true のときのみ表示し、
/// Esc / 背景クリック / Close ボタンで閉じる。
pub fn show(ctx: &egui::Context, state: &mut LicenseModalState) {
    if !state.open {
        return;
    }

    let modal = egui::Modal::new(egui::Id::new("oss_license_modal")).show(ctx, |ui| {
        ui.set_min_width(560.0);
        ui.set_max_width(720.0);

        ui.heading("Open Source Licenses");
        ui.label(
            RichText::new(format!(
                "This application bundles {} third-party crates.",
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
        let filtered: Vec<&LicenseEntry> = LICENSES
            .iter()
            .filter(|e| matches_filter(e, &needle))
            .collect();

        // モーダル高をビューポートに対して抑え、長大なリストはスクロールさせる。
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

    if modal.should_close() {
        state.open = false;
    }
}

/// 絞り込み条件にマッチするか（クレート名 / ライセンス種別を対象、空なら全件）。
fn matches_filter(entry: &LicenseEntry, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    entry.name.to_lowercase().contains(needle) || entry.license.to_lowercase().contains(needle)
}

/// 1 クレート分のエントリを折りたたみ見出しで描画する。
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
                // 等幅・選択可能なライセンス全文。
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
    fn default_state_is_closed() {
        let s = LicenseModalState::default();
        assert!(!s.open);
        assert!(s.search.is_empty());
    }
}
