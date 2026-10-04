//! "Trial detail" modal opened by clicking a point in a scatter plot.
//!
//! Shared by the Pareto 2D / Cluster 2D / MCDM scatter plots. Each scatter plot detects the
//! trial for the clicked point and passes the target to [`TrialDetailModal::open`], then
//! calls [`TrialDetailModal::show`] every frame to render it. In addition to chart-specific
//! information (Pareto rank / cluster number / MCDM rank, etc.), the modal shows objective
//! values, parameter values, user attributes, and artifacts (thumbnail + filename).

use std::collections::HashMap;

use tunny_core::dataframe::Feasibility;

use crate::io::artifacts::{file_image_uri, ArtifactEntry, ArtifactFileType};
use crate::state::types::{format_user_attribute, StudyView};

use super::modal::ModalScaffold;
use super::radar_chart;

/// Thumbnail side length (px).
const THUMB_SIZE: f32 = 220.0;

/// Threshold for point-click hit testing (screen-space distance from click position to
/// point, in px).
pub const HIT_THRESHOLD: f32 = 12.0;

/// The target point resolved by hit testing (`trial_id`, `row_index`).
pub type TrialHit = (u32, usize);

/// The target trial shown by the modal, plus chart-specific supplementary info.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialDetailTarget {
    /// Global ID of the target trial (used for artifact lookup; not displayed).
    pub trial_id: u32,
    /// Row index in the `StudyView`. Used both to look up objective/parameter values and,
    /// as the 0-based number within the Study, for the header display.
    pub row_index: usize,
    /// Chart-specific info (e.g. `[("Pareto Rank", "0")]`). Displayed in array order.
    pub context: Vec<(String, String)>,
}

/// Trial detail modal shared across scatter plots.
#[derive(Default)]
pub struct TrialDetailModal {
    /// The currently displayed target. `None` means it's closed.
    open: Option<TrialDetailTarget>,
    /// Show indexed rows for array user attributes in the current trial.
    expand_user_attr_lists: bool,
}

impl TrialDetailModal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the target trial and opens the modal.
    pub fn open(&mut self, target: TrialDetailTarget) {
        self.open = Some(target);
    }

    /// Whether the modal is open.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Renders the modal. Does nothing if it's closed.
    /// Closes on background click / Esc / the Close button.
    pub fn show(
        &mut self,
        ui: &egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        let Some(target) = self.open.clone() else {
            return;
        };
        let egui_ctx = ui.ctx().clone();
        let screen = egui_ctx.content_rect();
        // Sized to take up most of the screen, matching the artifact preview modal.
        let max_w = (screen.width() * 0.95).max(320.0);
        let max_h = (screen.height() * 0.95).max(240.0);
        // Height of the scrollable body area, excluding the header/separator/margins.
        let body_max_h = (max_h - 80.0).max(160.0);
        // Three columns: left = text info / center = radar chart / right = artifacts.
        // Left and center are fixed width; the rest goes to the right (artifacts).
        let left_w = (max_w * 0.26).clamp(280.0, 460.0);
        let radar_w = (max_w * 0.3).clamp(300.0, 500.0);

        let mut close = false;
        // The heading sits on the same row as the Close button, so we skip the scaffold's
        // automatic heading and draw it ourselves in the body.
        // Keep the modal large regardless of image aspect ratio (min=max=max_w).
        let outcome = ModalScaffold::new("trial_detail_modal", max_w)
            .max_width(max_w)
            .min_height(max_h)
            .show(&egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    // The header shows Optuna's `trial.number` (the 0-based creation-order
                    // number within the Study). `trial_id` is a global ID spanning storage
                    // and shifts by the number of pruned/failed trials and trials from
                    // other studies, so it's not used for display (it's still used for
                    // artifact lookup).
                    let trial_number = view
                        .df
                        .get_trial_number(target.row_index)
                        .unwrap_or(target.row_index as u32);
                    ui.heading(format!("Trial {trial_number}"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("× Close").clicked() {
                            close = true;
                        }
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(body_max_h)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        // Three columns: left = text info / center = radar chart / right =
                        // artifacts.
                        ui.horizontal_top(|ui| {
                            // Left: text info (Chart Info / Objectives / Variables / User Attributes).
                            ui.allocate_ui_with_layout(
                                egui::vec2(left_w, body_max_h),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    let mut first_section = true;
                                    if view.feasibility().has_constraints()
                                        && !target
                                            .context
                                            .iter()
                                            .any(|(key, _)| key == "Feasibility")
                                    {
                                        detail_section_label(ui, "Constraints", &mut first_section);
                                        let mut rows = Vec::new();
                                        push_feasible_row(
                                            &mut rows,
                                            view.feasibility(),
                                            target.row_index,
                                        );
                                        kv_rows(ui, &rows, left_w, |_| false);
                                    }
                                    // Chart-specific info (rank, cluster number, etc.).
                                    if !target.context.is_empty() {
                                        detail_section_label(ui, "Chart Info", &mut first_section);
                                        kv_rows(ui, &target.context, left_w, |i| {
                                            target.context[i].1.parse::<f64>().is_ok()
                                        });
                                    }

                                    // Objective values.
                                    if !obj_names.is_empty() {
                                        detail_section_label(ui, "Objectives", &mut first_section);
                                        let rows = value_rows(view, obj_names, target.row_index);
                                        kv_rows(ui, &rows, left_w, |_| true);
                                    }

                                    // Parameter values.
                                    if !param_names.is_empty() {
                                        detail_section_label(ui, "Variables", &mut first_section);
                                        let rows = value_rows(view, param_names, target.row_index);
                                        kv_rows(ui, &rows, left_w, |i| {
                                            view.numeric_column(&param_names[i]).is_some()
                                        });
                                    }

                                    if !first_section {
                                        ui.add_space(8.0);
                                        ui.separator();
                                        ui.add_space(4.0);
                                    }
                                    ui.horizontal(|ui| {
                                        section_label(ui, "User Attributes");
                                        if view.user_attribute_names().iter().any(|name| {
                                            view.df
                                                .user_attr_value(name, target.row_index)
                                                .and_then(|value| value.as_array())
                                                .is_some_and(|items| !items.is_empty())
                                        }) {
                                            ui.checkbox(
                                                &mut self.expand_user_attr_lists,
                                                "Expand lists",
                                            );
                                        }
                                    });
                                    let rows = user_attribute_rows(
                                        view,
                                        target.row_index,
                                        self.expand_user_attr_lists,
                                    );
                                    if rows.is_empty() {
                                        ui.label(egui::RichText::new("No user attributes.").weak());
                                    } else {
                                        for (index, row) in rows.iter().enumerate() {
                                            kv_row(
                                                ui,
                                                left_w,
                                                index,
                                                &row.key,
                                                &row.value,
                                                row.numeric,
                                                row.kind,
                                            );
                                        }
                                    }
                                },
                            );

                            ui.separator();

                            // Center: radar chart (objectives + variables). Overlays every
                            // Pareto-front individual as a thin line, with the outer
                            // envelope = the front's maxima; the selected trial is overlaid
                            // in red.
                            ui.allocate_ui_with_layout(
                                egui::vec2(radar_w, body_max_h),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    let radar_data = radar_chart::build(
                                        view,
                                        obj_names,
                                        param_names,
                                        target.row_index,
                                    );
                                    if radar_data.axes.len() >= 3 {
                                        section_label(ui, "Comparison (Radar)");
                                        radar_chart::show(ui, &radar_data);
                                    } else {
                                        ui.label(
                                            egui::RichText::new("Radar chart unavailable.").weak(),
                                        );
                                    }
                                },
                            );

                            ui.separator();

                            // Right: artifacts (thumbnail + filename).
                            ui.vertical(|ui| {
                                section_label(ui, "Artifacts");
                                match artifact_map.get(&target.trial_id) {
                                    Some(entries) if !entries.is_empty() => {
                                        render_artifacts(ui, entries)
                                    }
                                    _ => {
                                        ui.label(
                                            egui::RichText::new("No artifacts for this trial.")
                                                .weak(),
                                        );
                                    }
                                }
                            });
                        });
                    });
            });

        if close || outcome.should_close {
            self.open = None;
        }
    }
}

/// Section heading.
fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).strong());
}

fn detail_section_label(ui: &mut egui::Ui, text: &str, first: &mut bool) {
    if !*first {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
    }
    *first = false;
    section_label(ui, text);
}

/// Builds (name, formatted value) pairs from a column-name -> value-slice mapping.
fn value_rows(view: &StudyView, names: &[String], row_index: usize) -> Vec<(String, String)> {
    let cols = view.numeric_columns(names);
    names
        .iter()
        .zip(cols.iter())
        .map(|(name, col)| axis_row(name, *col, row_index))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeRowKind {
    Plain,
    ArrayParent,
    ArrayChild,
}

#[derive(Debug, PartialEq)]
struct UserAttributeRow {
    key: String,
    value: String,
    kind: AttributeRowKind,
    numeric: bool,
}

fn user_attribute_rows(
    view: &StudyView,
    row_index: usize,
    expand_lists: bool,
) -> Vec<UserAttributeRow> {
    view.user_attribute_columns(expand_lists)
        .into_iter()
        .filter_map(|column| {
            let value = column.value(&view.df, row_index);
            if column.array_index.is_some() && value.is_none() {
                return None;
            }
            let kind = if column.array_index.is_some() {
                AttributeRowKind::ArrayChild
            } else if expand_lists && value.is_some_and(serde_json::Value::is_array) {
                AttributeRowKind::ArrayParent
            } else {
                AttributeRowKind::Plain
            };
            let display = match (kind, value) {
                (AttributeRowKind::ArrayParent, Some(serde_json::Value::Array(items))) => {
                    format!("Array [{}]", items.len())
                }
                _ => format_user_attribute(value),
            };
            Some(UserAttributeRow {
                key: column.label(),
                value: display,
                kind,
                numeric: value.is_some_and(serde_json::Value::is_number),
            })
        })
        .collect()
}

fn kv_rows(
    ui: &mut egui::Ui,
    rows: &[(String, String)],
    width: f32,
    is_numeric: impl Fn(usize) -> bool,
) {
    for (index, (key, value)) in rows.iter().enumerate() {
        kv_row(
            ui,
            width,
            index,
            key,
            value,
            is_numeric(index),
            AttributeRowKind::Plain,
        );
    }
}

/// Draws a bounded key/value row with subtle stripes and a bottom separator.
fn kv_row(
    ui: &mut egui::Ui,
    width: f32,
    index: usize,
    key: &str,
    value: &str,
    numeric: bool,
    kind: AttributeRowKind,
) {
    let background = if index % 2 == 1 || kind == AttributeRowKind::ArrayParent {
        ui.visuals().faint_bg_color
    } else {
        egui::Color32::TRANSPARENT
    };
    let row = egui::Frame::new()
        .fill(background)
        .inner_margin(egui::Margin::symmetric(4, 3))
        .show(ui, |ui| {
            let available = (width - 8.0).max(120.0);
            ui.set_width(available);
            let key_width = available * 0.36;
            let value_width = (available - key_width - ui.spacing().item_spacing.x).max(40.0);
            ui.horizontal_top(|ui| {
                key_label(ui, key, kind, key_width);
                ui.add_sized(
                    [value_width, 0.0],
                    egui::Label::new(value).wrap().halign(if numeric {
                        egui::Align::RIGHT
                    } else {
                        egui::Align::LEFT
                    }),
                );
            });
        });
    ui.painter().hline(
        row.response.rect.x_range(),
        row.response.rect.bottom(),
        egui::Stroke::new(0.5, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
}

fn key_label(ui: &mut egui::Ui, key: &str, kind: AttributeRowKind, width: f32) -> egui::Response {
    let key_text = egui::RichText::new(key).color(crate::theme::TEXT_SECONDARY());
    let key_text = if kind == AttributeRowKind::ArrayParent {
        key_text.strong()
    } else {
        key_text
    };
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::left_to_right(egui::Align::Min),
        |ui| {
            if kind == AttributeRowKind::ArrayChild {
                ui.add_space(18.0);
            }
            ui.add(egui::Label::new(key_text).wrap())
        },
    )
    .inner
}

/// Formats an `Option<f64>` to 4 decimal places (`None` becomes an em dash).
/// Shared by the scatter plots' hover/click detail rows.
pub fn fmt_opt(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{x:.4}"),
        None => "—".to_string(),
    }
}

/// Builds a single `(axis name, formatted value)` row from a column slice. Shared by the
/// scatter plots' x/y/z axis value rows.
/// `col` is the numeric column for that axis (`None` for a missing column), `row` is the
/// row index in the `StudyView`.
pub fn axis_row(name: &str, col: Option<&[f64]>, row: usize) -> (String, String) {
    (
        name.to_string(),
        fmt_opt(col.and_then(|c| c.get(row)).copied()),
    )
}

/// Appends the explicit feasibility state, only for a constrained study.
/// Does nothing if there are no constraints (`has_constraints() == false`). Shared by both
/// the hover detail rows and the click detail context.
pub fn push_feasible_row(rows: &mut Vec<(String, String)>, feas: Feasibility, row: usize) {
    if feas.has_constraints() {
        rows.push((
            "Feasibility".to_string(),
            feas.state(row).label().to_string(),
        ));
    }
}

/// Renders artifacts side by side as a thumbnail (image) + filename.
fn render_artifacts(ui: &mut egui::Ui, entries: &[ArtifactEntry]) {
    ui.horizontal_wrapped(|ui| {
        for entry in entries {
            ui.allocate_ui(egui::vec2(THUMB_SIZE, THUMB_SIZE + 24.0), |ui| {
                ui.vertical(|ui| {
                    match (entry.file_type(), file_image_uri(&entry.path)) {
                        // Collapsing a non-UTF-8 path with `to_string_lossy` produces a URI
                        // for a path that doesn't actually exist, silently breaking the
                        // image. Reject it with `to_str()` and fall back the same way as
                        // for non-image files.
                        (ArtifactFileType::Image, Some(uri)) => {
                            ui.add(
                                egui::Image::from_uri(uri)
                                    .fit_to_exact_size(egui::vec2(THUMB_SIZE, THUMB_SIZE)),
                            );
                        }
                        (other, _) => {
                            let icon = if matches!(other, ArtifactFileType::Csv) {
                                "📊"
                            } else {
                                "📦"
                            };
                            ui.vertical_centered(|ui| {
                                ui.add_space(THUMB_SIZE * 0.25);
                                ui.label(egui::RichText::new(icon).size(THUMB_SIZE * 0.4));
                                ui.add_space(THUMB_SIZE * 0.25);
                                if ui.small_button("Open").clicked() {
                                    let _ = open::that(&entry.path);
                                }
                            });
                        }
                    }
                    ui.add(
                        egui::Label::new(egui::RichText::new(&entry.filename).small()).truncate(),
                    );
                });
            });
        }
    });
}

/// Returns the index of the candidate point closest to the click coordinate (screen
/// coordinates, within the threshold in px).
///
/// Call this after computing the points' screen coordinates inside the `egui_plot` closure.
/// It only takes candidate screen coordinates and the click coordinate so it stays a pure,
/// testable function.
pub fn nearest_within(
    screen_points: &[egui::Pos2],
    click: egui::Pos2,
    threshold: f32,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (i, &p) in screen_points.iter().enumerate() {
        let d = p.distance(click);
        if d <= threshold && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, i));
        }
    }
    best.map(|(_, i)| i)
}

/// Common helper that shows a tooltip at the pointer position summarizing the hovered
/// trial.
///
/// Shared by the scatter-style charts (Scatter 2D / History / Slice). Uses `trial_number`
/// as the heading and lays out `rows` (label, value) in a two-column grid. `id_salt` should
/// be a string unique per chart to avoid `Id` collisions between the tooltip and the grid.
pub fn show_hover_tooltip(
    ui: &egui::Ui,
    id_salt: &str,
    trial_number: u32,
    rows: &[(String, String)],
) {
    // egui 0.35: show_tooltip_at_pointer was removed. Replaced with
    // Tooltip::always_open + PopupAnchor::Pointer.
    egui::Tooltip::always_open(
        ui.ctx().clone(),
        ui.layer_id(),
        egui::Id::new(id_salt),
        egui::PopupAnchor::Pointer,
    )
    .show(|ui| {
        ui.strong(format!("Trial {trial_number}"));
        egui::Grid::new(format!("{id_salt}_grid"))
            .num_columns(2)
            .spacing([12.0, 2.0])
            .show(ui, |ui| {
                for (k, v) in rows {
                    ui.label(egui::RichText::new(k).color(crate::theme::TEXT_SECONDARY()));
                    ui.label(v);
                    ui.end_row();
                }
            });
    });
}

/// From the candidate points (trial_id, row_index, plot coordinates), returns the
/// `(trial_id, row_index)` of the point closest to the click position (screen coordinates)
/// and within `threshold` px.
pub fn hit_test_nearest(
    plot_ui: &egui_plot::PlotUi,
    candidates: &[(u32, usize, [f64; 2])],
    click: egui::Pos2,
    threshold: f32,
) -> Option<(u32, usize)> {
    let screen_points: Vec<egui::Pos2> = candidates
        .iter()
        .map(|&(_, _, [x, y])| plot_ui.screen_from_plot(egui_plot::PlotPoint::new(x, y)))
        .collect();
    nearest_within(&screen_points, click, threshold).map(|i| (candidates[i].0, candidates[i].1))
}

/// Consolidates click/hover resolution inside a 2D `egui_plot`. Call it from within the
/// `plot.show` closure, passing `plot_ui`. Returns `(click, hover)`, each being the closest
/// candidate point within [`HIT_THRESHOLD`] px as `(trial_id, row_index)` (or `None` if
/// there is no match).
///
/// - Clicks are resolved from `interact_pointer_pos` only on the frame the left button is
///   pressed.
/// - Hover is resolved on any frame where `hover_pos` is available.
pub fn resolve_click_hover(
    plot_ui: &egui_plot::PlotUi,
    candidates: &[(u32, usize, [f64; 2])],
) -> (Option<TrialHit>, Option<TrialHit>) {
    let resp = plot_ui.response();
    let clicked = if resp.clicked_by(egui::PointerButton::Primary) {
        resp.interact_pointer_pos()
            .and_then(|pos| hit_test_nearest(plot_ui, candidates, pos, HIT_THRESHOLD))
    } else {
        None
    };
    let hovered = resp
        .hover_pos()
        .and_then(|pos| hit_test_nearest(plot_ui, candidates, pos, HIT_THRESHOLD));
    (clicked, hovered)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_pairs(rows: Vec<UserAttributeRow>) -> Vec<(String, String)> {
        rows.into_iter().map(|row| (row.key, row.value)).collect()
    }

    #[test]
    fn expanded_array_child_key_is_indented() {
        let ctx = egui::Context::default();
        let mut key_positions = None;
        let _ = ctx.run_ui(Default::default(), |ui| {
            let parent = key_label(ui, "Attr1", AttributeRowKind::ArrayParent, 100.0);
            let child = key_label(ui, "Attr1[0]", AttributeRowKind::ArrayChild, 100.0);
            key_positions = Some((parent.rect.left(), child.rect.left()));
        });
        let (parent_x, child_x) = key_positions.unwrap();
        assert!(child_x >= parent_x + 18.0);
    }

    #[test]
    fn nearest_within_returns_closest_in_threshold() {
        let pts = vec![
            egui::pos2(0.0, 0.0),
            egui::pos2(10.0, 0.0),
            egui::pos2(100.0, 100.0),
        ];
        // The closest point to click (11, 0) is index 1 (distance 1).
        assert_eq!(nearest_within(&pts, egui::pos2(11.0, 0.0), 12.0), Some(1));
    }

    #[test]
    fn nearest_within_none_outside_threshold() {
        let pts = vec![egui::pos2(0.0, 0.0)];
        assert_eq!(nearest_within(&pts, egui::pos2(50.0, 50.0), 12.0), None);
    }

    #[test]
    fn nearest_within_empty_is_none() {
        assert_eq!(nearest_within(&[], egui::pos2(0.0, 0.0), 12.0), None);
    }

    #[test]
    fn nearest_within_picks_strictly_closest() {
        let pts = vec![egui::pos2(5.0, 0.0), egui::pos2(3.0, 0.0)];
        // Both are within the threshold, but the closer index 1 is chosen.
        assert_eq!(nearest_within(&pts, egui::pos2(0.0, 0.0), 12.0), Some(1));
    }

    #[test]
    fn fmt_opt_formats_and_handles_none() {
        assert_eq!(fmt_opt(Some(1.23456)), "1.2346");
        assert_eq!(fmt_opt(None), "—");
    }

    #[test]
    fn user_attribute_rows_preserve_types_missing_values_and_order() {
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow};

        let new_row = || TrialRow {
            trial_id: 0,
            trial_number: 0,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        };
        let mut first = new_row();
        first
            .user_attrs_numeric
            .insert("mixed".to_string(), 1.23456);
        first
            .user_attrs_string
            .insert("note".to_string(), "a,b\n\"c\"".to_string());
        let mut second = new_row();
        second
            .user_attrs_string
            .insert("mixed".to_string(), "text".to_string());
        let df = DataFrame::from_trials(
            &[first, second],
            &[],
            &[],
            &["mixed".to_string()],
            &["mixed".to_string(), "note".to_string()],
            0,
        );
        let view = StudyView::new(Arc::new(df), vec![]);
        assert_eq!(
            row_pairs(user_attribute_rows(&view, 0, false)),
            vec![
                ("mixed".to_string(), "1.2346".to_string()),
                ("note".to_string(), "a,b\n\"c\"".to_string()),
            ]
        );
        assert_eq!(user_attribute_rows(&view, 1, false)[0].value, "text");
        assert_eq!(user_attribute_rows(&view, 1, false)[1].value, "—");
    }

    #[test]
    fn user_attribute_rows_expand_only_present_array_elements() {
        use serde_json::json;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow};

        let make_row = |id, value| TrialRow {
            trial_id: id,
            trial_number: id,
            param_display: HashMap::new(),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::from([("items".to_string(), value)]),
            constraint_values: vec![],
        };
        let df = DataFrame::from_trials(
            &[
                make_row(0, json!([1, null, {"ok": true}])),
                make_row(1, json!(["short"])),
                make_row(2, json!("scalar")),
            ],
            &[],
            &[],
            &[],
            &[],
            0,
        );
        let view = StudyView::new(Arc::new(df), vec![]);
        assert_eq!(
            row_pairs(user_attribute_rows(&view, 0, false)),
            vec![("items".into(), "[1,null,{\"ok\":true}]".into())]
        );
        assert_eq!(
            row_pairs(user_attribute_rows(&view, 0, true)),
            vec![
                ("items".into(), "Array [3]".into()),
                ("items[0]".into(), "1.0000".into()),
                ("items[1]".into(), "null".into()),
                ("items[2]".into(), "{\"ok\":true}".into()),
            ]
        );
        let rows = user_attribute_rows(&view, 0, true);
        assert_eq!(rows[0].kind, AttributeRowKind::ArrayParent);
        assert_eq!(rows[1].kind, AttributeRowKind::ArrayChild);
        assert!(rows[1].numeric);
        assert!(!rows[2].numeric);
        assert_eq!(user_attribute_rows(&view, 1, true).len(), 2);
        assert_eq!(
            row_pairs(user_attribute_rows(&view, 2, true)),
            vec![("items".into(), "scalar".into())]
        );
    }
}
