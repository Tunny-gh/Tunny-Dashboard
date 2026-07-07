//! カード/画像描画。
//!
//! 1 枚の artifact カードの見た目（画像 or フォールバックアイコン + タイトル + 目的関数値）と、
//! カードをグリッド状に折り返して並べる純粋な描画ロジックをまとめる。

use std::collections::HashMap;

use crate::io::artifacts::{ArtifactEntry, ArtifactFileType};
use crate::state::app_state::AppState;
use crate::theme::chart_colors::COLOR_LINK;
use crate::ui::widgets::trial_detail_modal::TrialDetailTarget;

/// 1 枚のカード幅（サムネ + 枠線・内側余白・カード間隔）。
const CARD_PADDING: f32 = 24.0;

/// 1 枚のカードクリックで要求されたアクション。
#[derive(Default)]
struct CardClick {
    /// タイトルクリック → trial をハイライト。
    highlight: bool,
    /// 画像クリック → トライアル詳細モーダルを開く。
    detail: bool,
}

/// `content_w` に収まるカード列数を返す（最低 1 列）。
fn card_columns(content_w: f32, thumb: f32) -> usize {
    let col_w = thumb + CARD_PADDING;
    if col_w <= 0.0 {
        return 1;
    }
    ((content_w / col_w).floor() as usize).max(1)
}

/// カード群を `content_w` に収まる列数で折り返して描画する。
/// キャンバスの Area 内では `horizontal_wrapped` が折り返さないため、列数を明示計算して
/// 行ごとに `horizontal` で並べる。クリックされたカードの trial_id を返す。
pub(super) fn render_card_grid(
    ui: &mut egui::Ui,
    app_state: &AppState,
    content_w: f32,
    thumb: f32,
    cards: &[(u32, String, &ArtifactEntry)],
    obj_by_trial: &HashMap<u32, String>,
) -> (Option<u32>, Option<TrialDetailTarget>) {
    let columns = card_columns(content_w, thumb);
    let mut highlight: Option<u32> = None;
    let mut detail: Option<TrialDetailTarget> = None;
    for row in cards.chunks(columns) {
        ui.horizontal_top(|ui| {
            for (trial_id, badge, entry) in row {
                let obj_text = obj_by_trial.get(trial_id).map(String::as_str).unwrap_or("");
                let click =
                    show_artifact_card(ui, app_state, *trial_id, entry, badge, obj_text, thumb);
                if click.highlight {
                    highlight = Some(*trial_id);
                }
                if click.detail {
                    if let Some(target) = detail_target_for(app_state, *trial_id, badge) {
                        detail = Some(target);
                    }
                }
            }
        });
    }
    (highlight, detail)
}

/// `trial_id` から散布図共有の詳細モーダル用ターゲットを組み立てる。
/// `StudyView` 上の行 index を逆引きし、カードのバッジ（クラスタ番号 / MCDM ランク等）が
/// あれば Chart Info として付加する。
fn detail_target_for(
    app_state: &AppState,
    trial_id: u32,
    badge: &str,
) -> Option<TrialDetailTarget> {
    let ctx = app_state.current_study.as_ref()?;
    let row_index = ctx.view.trial_ids.iter().position(|&id| id == trial_id)?;
    let context = if badge.is_empty() {
        Vec::new()
    } else {
        vec![("Group".to_string(), badge.to_string())]
    };
    Some(TrialDetailTarget {
        trial_id,
        row_index,
        context,
    })
}

/// 画像アーティファクトの file:// URI を返す。
/// 画像でない、または非 UTF-8 パス（URI で表現できない）の場合は `None`。
/// 非 UTF-8 パスを `to_string_lossy` で潰すと実在しないパスの URI になり画像が
/// 無言で壊れるため、ここで弾いて呼び出し側にフォールバックさせる。
pub(super) fn image_uri(entry: &ArtifactEntry) -> Option<String> {
    if !matches!(entry.file_type(), ArtifactFileType::Image) {
        return None;
    }
    entry.path.to_str().map(|s| format!("file://{s}"))
}

/// 1 枚の artifact カードを描画する。
/// 画像クリックで拡大プレビュー、タイトルクリックで trial ハイライトを要求する。
#[allow(clippy::too_many_arguments)]
fn show_artifact_card(
    ui: &mut egui::Ui,
    app_state: &AppState,
    trial_id: u32,
    entry: &ArtifactEntry,
    badge: &str,
    obj_text: &str,
    thumb: f32,
) -> CardClick {
    let mut click = CardClick::default();
    let is_highlighted = app_state.highlighted_trial == Some(trial_id);
    let stroke = if is_highlighted {
        egui::Stroke::new(2.0, COLOR_LINK())
    } else {
        egui::Stroke::new(1.0, crate::theme::BORDER_COLOR())
    };

    egui::Frame::group(ui.style())
        .stroke(stroke)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.set_width(thumb);
            ui.vertical(|ui| {
                match (entry.file_type(), image_uri(entry)) {
                    (ArtifactFileType::Image, Some(uri)) => {
                        let resp = ui
                            .add(
                                egui::Image::from_uri(uri)
                                    .fit_to_exact_size(egui::vec2(thumb, thumb)),
                            )
                            .interact(egui::Sense::click())
                            .on_hover_text("Click for trial details");
                        if resp.clicked() {
                            click.detail = true;
                        }
                    }
                    (ArtifactFileType::Image, None) => {
                        // 非 UTF-8 パスは file:// URI で表現できない。壊れた画像の代わりに
                        // アイコン + Open ボタンでフォールバックする。
                        ui.vertical_centered(|ui| {
                            ui.add_space(thumb * 0.2);
                            ui.label(egui::RichText::new("🖼").size(thumb * 0.4));
                            ui.add_space(thumb * 0.2);
                            if ui.small_button("Open").clicked() {
                                let _ = open::that(&entry.path);
                            }
                        });
                    }
                    (other, _) => {
                        let icon = if matches!(other, ArtifactFileType::Csv) {
                            "📊"
                        } else {
                            "📦"
                        };
                        ui.vertical_centered(|ui| {
                            ui.add_space(thumb * 0.2);
                            ui.label(egui::RichText::new(icon).size(thumb * 0.4));
                            ui.add_space(thumb * 0.2);
                            if ui.small_button("Open").clicked() {
                                let _ = open::that(&entry.path);
                            }
                        });
                    }
                }
                let fname = entry.filename.clone();
                let header = if badge.is_empty() {
                    format!("Trial {trial_id}")
                } else {
                    format!("Trial {trial_id} · {badge}")
                };
                let label_resp = ui.add(
                    egui::Label::new(egui::RichText::new(header).small().strong())
                        .truncate()
                        .sense(egui::Sense::click()),
                );
                if label_resp.clicked() {
                    click.highlight = true;
                }
                ui.add(egui::Label::new(egui::RichText::new(fname).small().weak()).truncate());
                // 目的関数値（良し悪し判断の材料）。カード幅に合わせて折り返す。
                if !obj_text.is_empty() {
                    ui.add(egui::Label::new(
                        egui::RichText::new(obj_text)
                            .small()
                            .color(crate::theme::TEXT_SECONDARY()),
                    ));
                }
            });
        });
    click
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_columns_fits_width_and_min_one() {
        // thumb=140 → 列幅 164。
        assert_eq!(card_columns(700.0, 140.0), 4); // 700/164 = 4.26 → 4
        assert_eq!(card_columns(164.0, 140.0), 1);
        assert_eq!(card_columns(10.0, 140.0), 1); // 最低 1 列
    }
}
