//! Card/image rendering.
//!
//! Groups together the appearance of a single artifact card (image or
//! fallback icon + title + objective values) and the pure drawing logic that
//! wraps cards into a grid layout.

use std::collections::HashMap;

use crate::io::artifacts::{ArtifactEntry, ArtifactFileType};
use crate::state::app_state::AppState;
use crate::theme::chart_colors::COLOR_LINK;
use crate::ui::widgets::trial_detail_modal::TrialDetailTarget;

/// Width of a single card (thumbnail + border, inner margin, card spacing).
const CARD_PADDING: f32 = 24.0;

/// Action requested by clicking a single card.
#[derive(Default)]
struct CardClick {
    /// Title clicked -> highlight the trial.
    highlight: bool,
    /// Image clicked -> open the trial detail modal.
    detail: bool,
}

/// Returns the number of card columns that fit in `content_w` (minimum 1).
fn card_columns(content_w: f32, thumb: f32) -> usize {
    let col_w = thumb + CARD_PADDING;
    if col_w <= 0.0 {
        return 1;
    }
    ((content_w / col_w).floor() as usize).max(1)
}

/// Draws the cards wrapped into a column count that fits `content_w`.
/// Inside a canvas Area, `horizontal_wrapped` doesn't wrap, so the column
/// count is computed explicitly and each row is laid out with `horizontal`.
/// Returns the trial_id of the clicked card.
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

/// Builds the target for the detail modal shared with the scatter plots, from a `trial_id`.
/// Looks up the row index in `StudyView`, and if the card's badge (cluster
/// number / MCDM rank, etc.) is present, attaches it as Chart Info.
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

/// Returns the file:// URI of an image artifact.
/// Returns `None` if it's not an image, or if the path is non-UTF-8 (cannot
/// be represented as a URI). Collapsing a non-UTF-8 path via `to_string_lossy`
/// would produce a URI for a path that doesn't actually exist, silently
/// breaking the image, so it's rejected here to let the caller fall back instead.
pub(super) fn image_uri(entry: &ArtifactEntry) -> Option<String> {
    if !matches!(entry.file_type(), ArtifactFileType::Image) {
        return None;
    }
    entry.path.to_str().map(|s| format!("file://{s}"))
}

/// Draws a single artifact card.
/// An image click requests an enlarged preview; a title click requests a trial highlight.
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
                        // A non-UTF-8 path can't be represented as a file:// URI.
                        // Fall back to an icon + Open button instead of a broken image.
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
                // Objective values (basis for judging good/bad). Wraps to fit the card width.
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
        // thumb=140 -> column width 164.
        assert_eq!(card_columns(700.0, 140.0), 4); // 700/164 = 4.26 -> 4
        assert_eq!(card_columns(164.0, 140.0), 1);
        assert_eq!(card_columns(10.0, 140.0), 1); // minimum 1 column
    }
}
