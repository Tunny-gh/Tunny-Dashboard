//! SOM (Self-Organizing Map) widget.
//!
//! Trains a batch SOM on the standardized feature space
//! (`tunny_core::clustering::train_som`) and switches between displaying the
//! U-matrix, component planes, and hit counts. Training takes on the order of
//! milliseconds to tens of milliseconds, so this is a SYNC widget (computed directly
//! and cached in the render pass, without going through poll_chart). See
//! theory/{en,ja}/clustering/som.md for the theoretical background.
//!
//! The grid is interactive rather than a flat color map: each node reports its value
//! and the trials that landed on it, either as a number painted into the cell or, when
//! the cells are too small for that, on hover. Clicking a node pushes its trials into
//! the shared selection via [`SomMapChart::pending_selection`], the same handoff the
//! parallel-coordinates brush uses.

use crate::state::types::StudyView;
use crate::theme::chart_colors::COLOR_SELECTION_HIGHLIGHT;
use crate::theme::color_compute::contrasting_text_color;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::heatmap::draw_colorbar_simple;
use crate::ui::widgets::common::range_math::{expand_degenerate, normalize01, value_range};
use tunny_core::clustering::{train_som, SomResult, SomSpec};

/// Feature space used for training.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SomSpace {
    #[default]
    Params,
    ParamsAndObjectives,
}

impl SomSpace {
    fn label(self) -> &'static str {
        match self {
            SomSpace::Params => "Parameters",
            SomSpace::ParamsAndObjectives => "Parameters + Objectives",
        }
    }

    fn disc(self) -> u8 {
        match self {
            SomSpace::Params => 0,
            SomSpace::ParamsAndObjectives => 1,
        }
    }
}

/// Display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SomViewMode {
    #[default]
    UMatrix,
    ComponentPlane,
    Hits,
}

impl SomViewMode {
    fn label(self) -> &'static str {
        match self {
            SomViewMode::UMatrix => "U-Matrix",
            SomViewMode::ComponentPlane => "Component Plane",
            SomViewMode::Hits => "Hit Counts",
        }
    }
}

/// Number of trial numbers spelled out in a node's tooltip before the rest are
/// summarized as a count.
const MAX_TOOLTIP_TRIALS: usize = 10;

/// Width reserved to the right of the grid for the color bar plus its ticks and title
/// (`draw_colorbar_simple` documents ~80px for ticks and title, plus the 12px gap).
const COLORBAR_RESERVE: f32 = 96.0;

/// Smallest square the grid is drawn at, even in a cramped widget.
const MIN_GRID_SIDE: f32 = 120.0;

/// (study_name, row_count, grid_size, n_epochs, space disc)
type SomCacheKey = (String, usize, usize, usize, u8);

/// A trained map plus the mapping back to trials.
struct SomCache {
    key: SomCacheKey,
    result: SomResult,
    /// `StudyView` row indices that landed on each node, indexed by node.
    node_rows: Vec<Vec<usize>>,
}

/// What a click on the grid asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridAction {
    None,
    /// Select only this node.
    Replace(usize),
    /// Add this node to the selection, or drop it if it was already in.
    Toggle(usize),
    /// Drop the whole selection.
    Clear,
}

/// UI state for the SOM widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SomMapChart {
    /// Number of nodes per grid side (square grid, shared by both axes).
    pub grid_size: usize,
    pub n_epochs: usize,
    pub view_mode: SomViewMode,
    /// Feature name displayed in ComponentPlane mode.
    pub selected_feature: String,
    pub space: SomSpace,
    /// Nodes whose trials are currently selected. Node indices only mean anything for
    /// one trained map, so this is cleared whenever the map is retrained.
    #[serde(skip)]
    selected_nodes: Vec<usize>,
    /// Selection to hand over to `AppState::selected_indices`, taken by the caller
    /// (same convention as the parallel-coordinates brush).
    #[serde(skip)]
    pub pending_selection: Option<Vec<u32>>,
    #[serde(skip)]
    cache: Option<SomCache>,
}

impl Default for SomMapChart {
    fn default() -> Self {
        Self {
            grid_size: 8,
            n_epochs: 20,
            view_mode: SomViewMode::default(),
            selected_feature: String::new(),
            space: SomSpace::default(),
            selected_nodes: Vec::new(),
            pending_selection: None,
            cache: None,
        }
    }
}

/// The list of feature names to train on for a given `space` (whether to include objectives).
fn feature_names(param_names: &[String], obj_names: &[String], space: SomSpace) -> Vec<String> {
    match space {
        SomSpace::Params => param_names.to_vec(),
        SomSpace::ParamsAndObjectives => param_names
            .iter()
            .chain(obj_names.iter())
            .cloned()
            .collect(),
    }
}

/// Buckets the training rows by the node each one was assigned to, translating back to
/// `StudyView` row indices via `rows` (training row `i` came from `rows[i]`).
fn group_rows_by_node(result: &SomResult, rows: &[usize]) -> Vec<Vec<usize>> {
    let mut node_rows = vec![Vec::new(); result.grid_w * result.grid_h];
    for (i, &node) in result.bmu.iter().enumerate() {
        if let (Some(bucket), Some(&row)) = (node_rows.get_mut(node), rows.get(i)) {
            bucket.push(row);
        }
    }
    node_rows
}

/// Formats a node value for the tooltip and the in-cell overlay. Hit counts are whole
/// numbers; everything else is trimmed to three significant digits so it has a chance
/// of fitting inside a cell.
fn format_node_value(v: f64, integral: bool) -> String {
    if integral {
        return format!("{}", v.round() as i64);
    }
    let magnitude = v.abs();
    if magnitude != 0.0 && !(0.01..100_000.0).contains(&magnitude) {
        format!("{v:.2e}")
    } else {
        let decimals = if magnitude >= 100.0 {
            0
        } else if magnitude >= 10.0 {
            1
        } else {
            2
        };
        format!("{v:.decimals$}")
    }
}

impl SomMapChart {
    fn cache_key(&self, study_name: &str, row_count: usize) -> SomCacheKey {
        (
            study_name.to_string(),
            row_count,
            self.grid_size,
            self.n_epochs,
            self.space.disc(),
        )
    }

    /// Returns the node-value grid (row-major `grid_h * grid_w`) and axis label for
    /// the current display mode, for CSV export.
    pub fn current_grid(
        &self,
        param_names: &[String],
        obj_names: &[String],
    ) -> Option<(usize, usize, Vec<f64>, String)> {
        let cache = self.cache.as_ref()?;
        let features = feature_names(param_names, obj_names, self.space);
        let (values, label) = match self.view_mode {
            SomViewMode::UMatrix => (cache.result.u_matrix.clone(), "u_matrix".to_string()),
            SomViewMode::Hits => (
                cache.result.hits.iter().map(|&h| h as f64).collect(),
                "hits".to_string(),
            ),
            SomViewMode::ComponentPlane => {
                let idx = features.iter().position(|f| f == &self.selected_feature)?;
                (
                    cache.result.component_plane(idx),
                    self.selected_feature.clone(),
                )
            }
        };
        Some((cache.result.grid_w, cache.result.grid_h, values, label))
    }

    /// The trial IDs behind the currently selected nodes, in row order.
    fn selected_trials(&self, view: &StudyView) -> Vec<u32> {
        let Some(cache) = self.cache.as_ref() else {
            return Vec::new();
        };
        let mut rows: Vec<usize> = self
            .selected_nodes
            .iter()
            .filter_map(|&node| cache.node_rows.get(node))
            .flatten()
            .copied()
            .collect();
        rows.sort_unstable();
        rows.dedup();
        rows.iter()
            .filter_map(|&row| view.trial_ids.get(row).copied())
            .collect()
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        study_name: &str,
        cmap: &ColorMap,
    ) {
        ui.horizontal(|ui| {
            ui.label("Grid size:");
            ui.add(egui::Slider::new(&mut self.grid_size, 4..=16));
            ui.label("Epochs:");
            egui::ComboBox::from_id_salt("som_epochs")
                .selected_text(self.n_epochs.to_string())
                .show_ui(ui, |ui| {
                    for n in [10usize, 20, 50] {
                        ui.selectable_value(&mut self.n_epochs, n, n.to_string());
                    }
                });
            egui::ComboBox::from_id_salt("som_space")
                .selected_text(self.space.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.space,
                        SomSpace::Params,
                        SomSpace::Params.label(),
                    );
                    ui.selectable_value(
                        &mut self.space,
                        SomSpace::ParamsAndObjectives,
                        SomSpace::ParamsAndObjectives.label(),
                    );
                });
        });

        let features = feature_names(param_names, obj_names, self.space);

        ui.horizontal(|ui| {
            for mode in [
                SomViewMode::UMatrix,
                SomViewMode::ComponentPlane,
                SomViewMode::Hits,
            ] {
                if ui
                    .selectable_label(self.view_mode == mode, mode.label())
                    .clicked()
                {
                    self.view_mode = mode;
                }
            }
            if self.view_mode == SomViewMode::ComponentPlane {
                if self.selected_feature.is_empty() || !features.contains(&self.selected_feature) {
                    if let Some(f) = features.first() {
                        self.selected_feature = f.clone();
                    }
                }
                egui::ComboBox::from_id_salt("som_feature")
                    .selected_text(self.selected_feature.as_str())
                    .show_ui(ui, |ui| {
                        for f in &features {
                            ui.selectable_value(&mut self.selected_feature, f.clone(), f);
                        }
                    });
            }
        });

        if features.is_empty() {
            ui.colored_label(
                crate::theme::chart_colors::COLOR_EMPTY_STATE(),
                "No numeric columns available.",
            );
            return;
        }

        let key = self.cache_key(study_name, view.row_count());
        if self.cache.as_ref().map(|c| &c.key) != Some(&key) {
            // Every row is handed to `train_som`: it caps only the rows used by the
            // weight update (`MAX_SOM_TRAINING_ROWS`) and still assigns a BMU to all of
            // them, so hit counts and the click-to-select mapping cover every trial.
            let (rows, matrix) = super::feature_matrix_with_rows(view, &features);
            let spec = SomSpec {
                grid_w: self.grid_size,
                grid_h: self.grid_size,
                n_epochs: self.n_epochs,
            };
            self.cache = train_som(&matrix, &spec).map(|result| SomCache {
                key,
                node_rows: group_rows_by_node(&result, &rows),
                result,
            });
            // Node indices are meaningless against the new map, so the selection goes
            // too — including the copy the rest of the app is holding, which would
            // otherwise keep highlighting trials that no cell is marked with anymore.
            if !self.selected_nodes.is_empty() {
                self.selected_nodes.clear();
                self.pending_selection = Some(Vec::new());
            }
        }

        let Some(cache) = &self.cache else {
            ui.colored_label(
                crate::theme::chart_colors::COLOR_EMPTY_STATE(),
                "Not enough data to train a SOM (need >= 3 rows and a 2x2+ grid).",
            );
            return;
        };

        let (values, value_label): (Vec<f64>, String) = match self.view_mode {
            SomViewMode::UMatrix => (
                cache.result.u_matrix.clone(),
                "U-matrix distance".to_string(),
            ),
            SomViewMode::Hits => (
                cache.result.hits.iter().map(|&h| h as f64).collect(),
                "Hits".to_string(),
            ),
            SomViewMode::ComponentPlane => {
                let idx = features
                    .iter()
                    .position(|f| f == &self.selected_feature)
                    .unwrap_or(0);
                (
                    cache.result.component_plane(idx),
                    self.selected_feature.clone(),
                )
            }
        };

        let action = draw_grid(
            ui,
            view,
            cache,
            &values,
            &value_label,
            self.view_mode == SomViewMode::Hits,
            cmap,
            &self.selected_nodes,
        );

        match action {
            GridAction::None => return,
            GridAction::Replace(node) => self.selected_nodes = vec![node],
            GridAction::Toggle(node) => match self.selected_nodes.iter().position(|&n| n == node) {
                Some(at) => {
                    self.selected_nodes.remove(at);
                }
                None => self.selected_nodes.push(node),
            },
            GridAction::Clear => self.selected_nodes.clear(),
        }
        // An empty vector means "no selection filter", the same convention the
        // parallel-coordinates brush uses when its brushes are cleared.
        self.pending_selection = Some(self.selected_trials(view));
    }
}

/// Draws the node grid plus its color bar, and reports whatever the user clicked.
///
/// Cells carry their value as text whenever one fits; below that size the hover
/// tooltip is the only place the numbers appear.
#[allow(clippy::too_many_arguments)]
fn draw_grid(
    ui: &mut egui::Ui,
    view: &StudyView,
    cache: &SomCache,
    values: &[f64],
    value_label: &str,
    integral_values: bool,
    cmap: &ColorMap,
    selected_nodes: &[usize],
) -> GridAction {
    let (grid_w, grid_h) = (cache.result.grid_w, cache.result.grid_h);
    let (v_min, v_max) = value_range(values.iter().copied())
        .map(|(mn, mx)| expand_degenerate(mn, mx))
        .unwrap_or((0.0, 1.0));

    let avail = ui.available_size();
    let side = (avail.x - COLORBAR_RESERVE).min(avail.y).max(MIN_GRID_SIDE);
    let canvas_size = egui::vec2(side + COLORBAR_RESERVE, side);

    let mut action = GridAction::None;
    ui.allocate_ui(canvas_size, |ui| {
        ui.set_min_size(canvas_size);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
        let painter = ui.painter_at(rect);
        let cell = egui::vec2(rect.width() / grid_w as f32, rect.height() / grid_h as f32);
        let cell_rect = |node: usize| {
            egui::Rect::from_min_size(
                egui::pos2(
                    rect.left() + (node % grid_w) as f32 * cell.x,
                    rect.top() + (node / grid_w) as f32 * cell.y,
                ),
                cell,
            )
        };
        // Scale the overlay font to the cell, then let the fit test below decide
        // whether the text is drawn at all.
        let font = egui::FontId::proportional((cell.y * 0.34).clamp(7.0, 14.0));

        for node in 0..grid_w * grid_h {
            let Some(&v) = values.get(node) else {
                continue;
            };
            let color = cmap.interpolate(normalize01(v, v_min, v_max));
            let r = cell_rect(node);
            // Overdraw by a pixel so no seam shows between neighboring cells.
            painter.rect_filled(r.expand(0.5), 0.0, color);

            let text_color = contrasting_text_color(color);
            let galley = painter.layout_no_wrap(
                format_node_value(v, integral_values),
                font.clone(),
                text_color,
            );
            if galley.size().x <= cell.x - 4.0 && galley.size().y <= cell.y - 2.0 {
                painter.galley(r.center() - galley.size() * 0.5, galley, text_color);
            }
        }

        for &node in selected_nodes {
            painter.rect_stroke(
                cell_rect(node),
                0.0,
                egui::Stroke::new(2.0, COLOR_SELECTION_HIGHLIGHT()),
                egui::StrokeKind::Inside,
            );
        }

        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
            egui::StrokeKind::Inside,
        );

        let hovered = resp
            .hover_pos()
            .and_then(|pos| node_at(rect, grid_w, grid_h, pos));
        if let Some(node) = hovered {
            painter.rect_stroke(
                cell_rect(node),
                0.0,
                egui::Stroke::new(
                    1.5,
                    contrasting_text_color(cmap.interpolate(normalize01(
                        values.get(node).copied().unwrap_or(0.0),
                        v_min,
                        v_max,
                    ))),
                ),
                egui::StrokeKind::Inside,
            );
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let resp = resp.on_hover_ui_at_pointer(|ui| {
            let Some(node) = hovered else { return };
            node_tooltip(
                ui,
                view,
                cache,
                node,
                values.get(node).copied().unwrap_or(0.0),
                value_label,
                integral_values,
            );
        });

        if resp.secondary_clicked() || resp.double_clicked() {
            action = GridAction::Clear;
        } else if resp.clicked() {
            if let Some(node) = hovered {
                action = if ui.input(|i| i.modifiers.command) {
                    GridAction::Toggle(node)
                } else {
                    GridAction::Replace(node)
                };
            }
        }

        let bar_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() + 12.0, rect.top()),
            egui::vec2(14.0, rect.height()),
        );
        draw_colorbar_simple(ui, bar_rect, v_min, v_max, cmap.clone(), Some(value_label));
    });

    action
}

/// The node under `pos`, or `None` when the pointer is outside the grid.
fn node_at(rect: egui::Rect, grid_w: usize, grid_h: usize, pos: egui::Pos2) -> Option<usize> {
    if !rect.contains(pos) || grid_w == 0 || grid_h == 0 {
        return None;
    }
    let gx = (((pos.x - rect.left()) / rect.width()) * grid_w as f32) as usize;
    let gy = (((pos.y - rect.top()) / rect.height()) * grid_h as f32) as usize;
    Some(gy.min(grid_h - 1) * grid_w + gx.min(grid_w - 1))
}

/// Fills in the hover tooltip for one node.
fn node_tooltip(
    ui: &mut egui::Ui,
    view: &StudyView,
    cache: &SomCache,
    node: usize,
    value: f64,
    value_label: &str,
    integral_value: bool,
) {
    let rows = cache.node_rows.get(node).map(Vec::as_slice).unwrap_or(&[]);
    ui.label(format!(
        "Node ({}, {})",
        node % cache.result.grid_w,
        node / cache.result.grid_w
    ));
    ui.label(format!("Hits: {}", rows.len()));
    ui.label(format!(
        "{value_label}: {}",
        format_node_value(value, integral_value)
    ));

    if rows.is_empty() {
        return;
    }
    let shown: Vec<String> = rows
        .iter()
        .take(MAX_TOOLTIP_TRIALS)
        .map(|&row| {
            view.df
                .get_trial_number(row)
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".to_string())
        })
        .collect();
    let mut text = format!("Trials: {}", shown.join(", "));
    if rows.len() > shown.len() {
        text.push_str(&format!(" …and {} more", rows.len() - shown.len()));
    }
    ui.label(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn som_map_chart_default_values() {
        let s = SomMapChart::default();
        assert_eq!(s.grid_size, 8);
        assert_eq!(s.n_epochs, 20);
        assert_eq!(s.view_mode, SomViewMode::UMatrix);
        assert_eq!(s.space, SomSpace::Params);
        assert!(s.selected_feature.is_empty());
        assert!(s.selected_nodes.is_empty());
        assert!(s.pending_selection.is_none());
        assert!(s.cache.is_none());
    }

    #[test]
    fn som_space_disc_is_distinct() {
        assert_ne!(
            SomSpace::Params.disc(),
            SomSpace::ParamsAndObjectives.disc()
        );
    }

    #[test]
    fn feature_names_params_only() {
        let params = vec!["x".to_string(), "y".to_string()];
        let objs = vec!["obj".to_string()];
        let names = feature_names(&params, &objs, SomSpace::Params);
        assert_eq!(names, params);
    }

    #[test]
    fn feature_names_params_and_objectives() {
        let params = vec!["x".to_string()];
        let objs = vec!["obj".to_string()];
        let names = feature_names(&params, &objs, SomSpace::ParamsAndObjectives);
        assert_eq!(names, vec!["x".to_string(), "obj".to_string()]);
    }

    /// Builds a `StudyView` whose single parameter column contains `values`.
    fn view_with_param(values: &[f64]) -> (StudyView, Vec<String>) {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let core_rows: Vec<CoreRow> = values
            .iter()
            .enumerate()
            .map(|(i, &v)| CoreRow {
                trial_id: 100 + i as u32,
                trial_number: i as u32,
                param_display: [("x".to_string(), v)].into(),
                param_category_label: HashMap::new(),
                objective_values: vec![],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let param_names = vec!["x".to_string()];
        let df = DataFrame::from_trials(&core_rows, &param_names, &[], &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0; values.len()]);
        (view, param_names)
    }

    #[test]
    fn training_matrix_reports_the_source_row_of_every_kept_row() {
        // The NaN row is dropped, so the second kept row is view row 2, not row 1 —
        // this offset is what the node-to-trial mapping depends on.
        let (view, param_names) = view_with_param(&[1.0, f64::NAN, 3.0]);
        let (rows, matrix) = super::super::feature_matrix_with_rows(&view, &param_names);
        assert_eq!(rows, vec![0, 2]);
        assert_eq!(matrix, vec![vec![1.0], vec![3.0]]);
    }

    #[test]
    fn group_rows_by_node_translates_bmu_back_to_view_rows() {
        let result = SomResult {
            grid_w: 2,
            grid_h: 1,
            weights: vec![vec![0.0], vec![1.0]],
            u_matrix: vec![0.0, 0.0],
            hits: vec![1, 1],
            bmu: vec![0, 1],
            feature_means: vec![0.0],
            feature_stds: vec![1.0],
        };
        // Training row 1 came from view row 2 (row 1 held a NaN and was dropped).
        let node_rows = group_rows_by_node(&result, &[0, 2]);
        assert_eq!(node_rows, vec![vec![0], vec![2]]);
    }

    #[test]
    fn selected_trials_maps_nodes_to_trial_ids_without_duplicates() {
        let (view, _) = view_with_param(&[1.0, 2.0, 3.0]);
        let chart = SomMapChart {
            cache: Some(SomCache {
                key: ("s".to_string(), 3, 8, 20, 0),
                result: SomResult {
                    grid_w: 2,
                    grid_h: 1,
                    weights: vec![vec![0.0], vec![1.0]],
                    u_matrix: vec![0.0, 0.0],
                    hits: vec![2, 1],
                    bmu: vec![0, 0, 1],
                    feature_means: vec![0.0],
                    feature_stds: vec![1.0],
                },
                node_rows: vec![vec![0, 1], vec![2]],
            }),
            selected_nodes: vec![1, 0],
            ..Default::default()
        };
        // trial_ids start at 100 in the fixture, and the result is ordered by row.
        assert_eq!(chart.selected_trials(&view), vec![100, 101, 102]);
    }

    #[test]
    fn selected_trials_is_empty_without_a_trained_map() {
        let (view, _) = view_with_param(&[1.0, 2.0, 3.0]);
        let chart = SomMapChart::default();
        assert!(chart.selected_trials(&view).is_empty());
    }

    #[test]
    fn node_at_maps_corners_to_the_first_and_last_node() {
        let rect = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(80.0, 80.0));
        assert_eq!(node_at(rect, 4, 4, egui::pos2(1.0, 1.0)), Some(0));
        assert_eq!(node_at(rect, 4, 4, egui::pos2(79.0, 79.0)), Some(15));
        // The bottom-right corner is exactly on the edge; it must not spill past the
        // last node.
        assert_eq!(node_at(rect, 4, 4, egui::pos2(80.0, 80.0)), Some(15));
        assert_eq!(node_at(rect, 4, 4, egui::pos2(-1.0, 40.0)), None);
    }

    #[test]
    fn format_node_value_keeps_hit_counts_whole() {
        assert_eq!(format_node_value(139.0, true), "139");
        assert_eq!(format_node_value(0.0, true), "0");
    }

    #[test]
    fn format_node_value_trims_decimals_as_the_magnitude_grows() {
        assert_eq!(format_node_value(0.4234, false), "0.42");
        assert_eq!(format_node_value(12.345, false), "12.3");
        assert_eq!(format_node_value(432.1, false), "432");
    }

    #[test]
    fn format_node_value_falls_back_to_exponent_for_extreme_magnitudes() {
        assert_eq!(format_node_value(0.0, false), "0.00");
        assert!(format_node_value(1.2e-7, false).contains('e'));
        assert!(format_node_value(9.9e12, false).contains('e'));
    }
}
