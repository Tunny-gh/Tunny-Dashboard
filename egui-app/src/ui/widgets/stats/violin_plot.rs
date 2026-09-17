use std::ops::RangeInclusive;
use std::sync::Arc;

use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_BAR_ACCENT, COLOR_BAR_PRIMARY};
use crate::ui::widgets::common::axis_labels::{draw_plot_x_labels, plot_x_label_band};
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::stats::box_plot::normalize_minmax;
use tunny_core::statistics::{compute_violin, ViolinCurve};

/// Width assumed for the y axis on the very first frame, before the plot has reported
/// its actual frame. From the second frame on, the measured width is used instead.
const Y_AXIS_WIDTH_GUESS: f32 = 56.0;

/// Floor on the plot height, so a long rotated label band cannot collapse the violins.
const MIN_PLOT_HEIGHT: f32 = 80.0;

/// Number of KDE grid points per violin. Shared with the CSV export so both agree.
pub(crate) const GRID_POINTS: usize = 128;

/// Half-width of a violin at its widest point (in x-axis units). Every violin is
/// scaled to this same maximum width.
const HALF_WIDTH: f64 = 0.4;

/// Half-width of the median marker segment (in x-axis units).
const MEDIAN_HALF_WIDTH: f64 = 0.2;

/// The target column group for the violin plot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ViolinSource {
    #[default]
    Objectives,
    Parameters,
}

impl ViolinSource {
    fn label(self) -> &'static str {
        match self {
            ViolinSource::Objectives => "Objectives",
            ViolinSource::Parameters => "Parameters",
        }
    }

    fn disc(self) -> u8 {
        match self {
            ViolinSource::Objectives => 0,
            ViolinSource::Parameters => 1,
        }
    }
}

/// (study_name, source_disc, normalize, selected_numeric, category_or_empty, row_count)
type ViolinCacheKey = (String, u8, bool, String, String, usize);

/// The curves built for a [`ViolinCacheKey`] and the number of groups they were derived
/// from (used only to report how many groups were skipped).
type ViolinCacheEntry = (ViolinCacheKey, Vec<(String, ViolinCurve)>, usize);

/// A widget that displays violin plots (kernel density estimates) for multiple columns
/// or for the levels of a categorical parameter.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ViolinPlotChart {
    pub source: ViolinSource,
    /// Whether to min-max normalize the selected column for display ([0,1]; the
    /// density shape is computed after normalization).
    pub normalize: bool,
    /// The categorical parameter to split by, or `None` for the side-by-side mode.
    pub category: Option<String>,
    /// The numeric column used when `category` is set.
    pub selected_numeric: String,
    /// Curves and the number of groups they were derived from, both keyed by
    /// [`ViolinCacheKey`]. Caching the attempted-group count keeps the per-frame render
    /// path from rescanning the whole category column just to size the skipped note.
    #[serde(skip)]
    cache: Option<ViolinCacheEntry>,
}

impl ViolinPlotChart {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        study_name: &str,
    ) {
        // The Source ComboBox mutates `self.source` in place, so the column lists must be
        // derived *after* it has been drawn. Computing them first would feed the previous
        // source's columns to `build_curves` while the cache key already uses the newly
        // selected source, poisoning the cache with the wrong source's columns.
        let numeric = ui
            .horizontal(|ui| {
                egui::ComboBox::from_id_salt("violin_plot_source_combo")
                    .selected_text(self.source.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.source,
                            ViolinSource::Objectives,
                            ViolinSource::Objectives.label(),
                        );
                        ui.selectable_value(
                            &mut self.source,
                            ViolinSource::Parameters,
                            ViolinSource::Parameters.label(),
                        );
                    });

                ui.toggle_value(&mut self.normalize, "Normalize [0,1]")
                    .on_hover_text("Min-max normalize the selected column for display");

                // Derive the candidates from the source selected just above.
                let names: &[String] = match self.source {
                    ViolinSource::Objectives => obj_names,
                    ViolinSource::Parameters => param_names,
                };
                // Numeric candidates exclude categorical (string) columns.
                let numeric: Vec<String> = names
                    .iter()
                    .filter(|n| view.numeric_column(n).is_some())
                    .cloned()
                    .collect();
                // Category candidates are the categorical parameters only (string columns).
                let categorical: Vec<String> = param_names
                    .iter()
                    .filter(|n| view.string_column(n).is_some())
                    .cloned()
                    .collect();

                // Validate the selections against the fresh candidates before the cache
                // check, so a column/category from a previous source can never reach the
                // computation.
                if !numeric.iter().any(|n| n == &self.selected_numeric) {
                    self.selected_numeric = numeric.first().cloned().unwrap_or_default();
                }
                if let Some(cat) = &self.category {
                    if !categorical.iter().any(|c| c == cat) {
                        self.category = None;
                    }
                }

                egui::ComboBox::from_id_salt("violin_plot_category_combo")
                    .selected_text(self.category.clone().unwrap_or_else(|| "None".to_string()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.category, None, "None");
                        for name in &categorical {
                            ui.selectable_value(
                                &mut self.category,
                                Some(name.clone()),
                                name.as_str(),
                            );
                        }
                    });

                if self.category.is_some() {
                    egui::ComboBox::from_id_salt("violin_plot_numeric_combo")
                        .selected_text(self.selected_numeric.clone())
                        .show_ui(ui, |ui| {
                            for name in &numeric {
                                ui.selectable_value(
                                    &mut self.selected_numeric,
                                    name.clone(),
                                    name.as_str(),
                                );
                            }
                        });
                }

                numeric
            })
            .inner;

        if numeric.is_empty() {
            not_enough_data(ui);
            return;
        }

        let key: ViolinCacheKey = (
            study_name.to_string(),
            self.source.disc(),
            self.normalize,
            self.selected_numeric.clone(),
            self.category.clone().unwrap_or_default(),
            view.row_count(),
        );
        if self.cache.as_ref().map(|(k, _, _)| k) != Some(&key) {
            let (curves, attempted) = build_curves(
                view,
                &numeric,
                self.category.as_deref(),
                &self.selected_numeric,
                self.normalize,
            );
            self.cache = Some((key, curves, attempted));
        }

        let (curves, attempted) = {
            let (_, curves, attempted) = self.cache.as_ref().expect("cache populated above");
            (curves, *attempted)
        };
        if curves.is_empty() {
            not_enough_data(ui);
            return;
        }

        let skipped = attempted.saturating_sub(curves.len());
        if skipped > 0 {
            ui.label(
                egui::RichText::new(format!(
                    "{skipped} group(s) skipped (fewer than 2 values or constant)."
                ))
                .small()
                .weak(),
            );
        }

        let labels: Vec<String> = curves.iter().map(|(name, _)| name.clone()).collect();
        let y_label = match &self.category {
            Some(_) => self.selected_numeric.clone(),
            None => "Value".to_string(),
        };

        // A category level (or a column index in the side-by-side mode) is placed at its
        // integer index; reserve a band below the plot for the rotated/slanted names,
        // exactly like the box plot does.
        let avail = ui.available_size();
        let width_memo_id = ui.id().with("violin_plot_x_label_band_width");
        let plot_width = ui
            .data(|d| d.get_temp::<f32>(width_memo_id))
            .filter(|w| *w > 0.0)
            .unwrap_or_else(|| (avail.x - Y_AXIS_WIDTH_GUESS).max(1.0));
        let plan = plot_x_label_band(ui, &labels, plot_width);
        let plot_height = (avail.y - plan.height).max(MIN_PLOT_HEIGHT);

        let mut plot = egui_plot::Plot::new("violin_plot_plot")
            .unified_nav()
            .legend(egui_plot::Legend::default())
            .show_axes([false, true])
            .height(plot_height)
            .y_axis_label(y_label);
        if let Some(cat) = &self.category {
            plot = plot.x_axis_label(cat.clone());
        }

        let fill = COLOR_BAR_PRIMARY();
        let median_color = COLOR_BAR_ACCENT();
        let resp = plot.show(ui, |plot_ui| {
            apply_wheel_zoom(plot_ui);
            plot_ui.add(ViolinItem::new(curves, fill, median_color));
        });

        let measured_width = resp.transform.frame().width();
        ui.data_mut(|d| d.insert_temp(width_memo_id, measured_width));
        let (band, _) =
            ui.allocate_exact_size(egui::vec2(avail.x, plan.height), egui::Sense::hover());
        draw_plot_x_labels(ui, band, &resp.transform, &labels, &plan);
    }
}

/// Builds one `(label, curve)` per violin for the current mode, along with the number
/// of groups the mode attempted to draw (used only to report how many were skipped).
///
/// Without a category, one violin is drawn per numeric column of the source (each
/// normalized independently), and the attempted count is the number of numeric columns.
/// With a category, the selected numeric column is normalized as a whole first, then
/// split into one violin per category level (sorted by label); empty-string level labels
/// are skipped. An empty-string label still counts as one attempted group, matching the
/// skipped-count note.
fn build_curves(
    view: &StudyView,
    numeric: &[String],
    category: Option<&str>,
    selected_numeric: &str,
    normalize: bool,
) -> (Vec<(String, ViolinCurve)>, usize) {
    let Some(category) = category else {
        let curves = numeric
            .iter()
            .filter_map(|name| {
                let raw = view.numeric_column(name)?;
                let values = if normalize {
                    normalize_minmax(raw)
                } else {
                    raw.to_vec()
                };
                compute_violin(&values, GRID_POINTS).map(|c| (name.clone(), c))
            })
            .collect();
        return (curves, numeric.len());
    };

    let Some(cat_col) = view.string_column(category) else {
        return (Vec::new(), 0);
    };
    let Some(raw) = view.numeric_column(selected_numeric) else {
        return (Vec::new(), 0);
    };
    // Normalize the whole column before splitting into groups.
    let values = if normalize {
        normalize_minmax(raw)
    } else {
        raw.to_vec()
    };

    // BTreeMap keeps the levels sorted by label, matching the drawn order.
    let mut groups: std::collections::BTreeMap<&str, Vec<f64>> = std::collections::BTreeMap::new();
    let mut has_empty = false;
    for (i, label) in cat_col.iter().enumerate() {
        if label.is_empty() {
            has_empty = true;
            continue;
        }
        if let Some(&v) = values.get(i) {
            groups.entry(label.as_str()).or_default().push(v);
        }
    }
    let attempted = groups.len() + usize::from(has_empty);
    let curves = groups
        .into_iter()
        .filter_map(|(label, vals)| {
            compute_violin(&vals, GRID_POINTS).map(|c| (label.to_string(), c))
        })
        .collect();
    (curves, attempted)
}

fn not_enough_data(ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.label(
            egui::RichText::new(
                "Need at least 2 finite, non-identical values to estimate a distribution.",
            )
            .weak(),
        );
    });
}

/// A custom [`egui_plot::PlotItem`] that draws the violin fills as triangle-strip
/// meshes. `egui_plot::Polygon`/`FilledArea` are not usable here: `Polygon` fills via
/// `Shape::convex_polygon` (which breaks on the non-convex, multi-modal outline) and
/// `FilledArea` only fills between two x-indexed lines, not between the left/right
/// edges of a violin.
struct ViolinItem<'a> {
    base: egui_plot::PlotItemBase,
    violins: &'a [(String, ViolinCurve)],
    fill: egui::Color32,
    median_color: egui::Color32,
    bounds: egui_plot::PlotBounds,
}

impl<'a> ViolinItem<'a> {
    fn new(
        violins: &'a [(String, ViolinCurve)],
        fill: egui::Color32,
        median_color: egui::Color32,
    ) -> Self {
        let mut bounds = egui_plot::PlotBounds::NOTHING;
        for (i, (_, curve)) in violins.iter().enumerate() {
            let center = i as f64;
            let scale = density_scale(curve);
            for (&g, &d) in curve.grid.iter().zip(curve.density.iter()) {
                let w = d * scale;
                bounds.extend_with(&egui_plot::PlotPoint::new(center + w, g));
                bounds.extend_with(&egui_plot::PlotPoint::new(center - w, g));
            }
            bounds.extend_with(&egui_plot::PlotPoint::new(
                center - MEDIAN_HALF_WIDTH,
                curve.median,
            ));
            bounds.extend_with(&egui_plot::PlotPoint::new(
                center + MEDIAN_HALF_WIDTH,
                curve.median,
            ));
        }
        Self {
            base: egui_plot::PlotItemBase::new("Violin".to_string()),
            violins,
            fill,
            median_color,
            bounds,
        }
    }
}

/// Scales a curve's density so its peak reaches [`HALF_WIDTH`]. Returns 0 for a
/// degenerate (all-zero / non-finite) density.
fn density_scale(curve: &ViolinCurve) -> f64 {
    let max_density = curve
        .density
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .fold(0.0_f64, f64::max);
    if max_density > 0.0 {
        HALF_WIDTH / max_density
    } else {
        0.0
    }
}

impl egui_plot::PlotItem for ViolinItem<'_> {
    fn shapes(
        &self,
        _ui: &egui::Ui,
        transform: &egui_plot::PlotTransform,
        shapes: &mut Vec<egui::Shape>,
    ) {
        for (i, (_, curve)) in self.violins.iter().enumerate() {
            let n = curve.grid.len().min(curve.density.len());
            if n < 2 {
                continue;
            }
            let center = i as f64;
            let scale = density_scale(curve);
            if scale <= 0.0 {
                continue;
            }

            let right: Vec<egui::Pos2> = (0..n)
                .map(|j| {
                    transform.position_from_point(&egui_plot::PlotPoint::new(
                        center + curve.density[j] * scale,
                        curve.grid[j],
                    ))
                })
                .collect();
            let left: Vec<egui::Pos2> = (0..n)
                .map(|j| {
                    transform.position_from_point(&egui_plot::PlotPoint::new(
                        center - curve.density[j] * scale,
                        curve.grid[j],
                    ))
                })
                .collect();

            // Fill the region between the right and left edges. Vertices are laid out as
            // [right..., left...] and each consecutive pair forms a quad; this is the same
            // triangle-strip pattern `egui_plot`'s built-in FilledArea uses, and it handles
            // the non-convex (multi-modal) outline that `Shape::convex_polygon` cannot.
            let mut mesh = egui::Mesh::default();
            mesh.reserve_vertices(n * 2);
            mesh.reserve_triangles((n - 1) * 2);
            for &p in &right {
                mesh.colored_vertex(p, self.fill);
            }
            for &p in &left {
                mesh.colored_vertex(p, self.fill);
            }
            for j in 0..n - 1 {
                mesh.add_triangle(j as u32, (n + j) as u32, (j + 1) as u32);
                mesh.add_triangle((n + j) as u32, (n + j + 1) as u32, (j + 1) as u32);
            }
            shapes.push(egui::Shape::Mesh(Arc::new(mesh)));

            // Outline the violin: right edge top-to-bottom, then left edge bottom-to-top.
            let mut outline = right;
            outline.extend(left.iter().rev().copied());
            shapes.push(egui::Shape::closed_line(
                outline,
                egui::Stroke::new(1.0, self.fill),
            ));

            // Median marker: a short horizontal segment centered on the group.
            let y = curve.median;
            let a = transform
                .position_from_point(&egui_plot::PlotPoint::new(center - MEDIAN_HALF_WIDTH, y));
            let b = transform
                .position_from_point(&egui_plot::PlotPoint::new(center + MEDIAN_HALF_WIDTH, y));
            shapes.push(egui::Shape::line_segment(
                [a, b],
                egui::Stroke::new(2.0, self.median_color),
            ));
        }
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn color(&self) -> egui::Color32 {
        self.fill
    }

    fn geometry(&self) -> egui_plot::PlotGeometry<'_> {
        egui_plot::PlotGeometry::None
    }

    fn bounds(&self) -> egui_plot::PlotBounds {
        self.bounds
    }

    fn base(&self) -> &egui_plot::PlotItemBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut egui_plot::PlotItemBase {
        &mut self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violin_plot_chart_default_values() {
        let chart = ViolinPlotChart::default();
        assert_eq!(chart.source, ViolinSource::Objectives);
        assert!(!chart.normalize);
        assert!(chart.category.is_none());
        assert!(chart.selected_numeric.is_empty());
        assert!(chart.cache.is_none());
    }

    #[test]
    fn cache_key_changes_with_category() {
        let key_a: ViolinCacheKey = ("s".into(), 0, false, "x".into(), String::new(), 10);
        let key_b: ViolinCacheKey = ("s".into(), 0, false, "x".into(), "cat".into(), 10);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_changes_with_selected_numeric() {
        let key_a: ViolinCacheKey = ("s".into(), 0, false, "x".into(), "cat".into(), 10);
        let key_b: ViolinCacheKey = ("s".into(), 0, false, "y".into(), "cat".into(), 10);
        assert_ne!(key_a, key_b);
    }

    #[test]
    fn cache_key_changes_with_source_normalize_and_rows() {
        let base: ViolinCacheKey = ("s".into(), 0, false, "x".into(), String::new(), 10);
        assert_ne!(base, ("s".into(), 1, false, "x".into(), String::new(), 10));
        assert_ne!(base, ("s".into(), 0, true, "x".into(), String::new(), 10));
        assert_ne!(base, ("s".into(), 0, false, "x".into(), String::new(), 11));
    }

    /// Builds a `StudyView` with numeric parameter `x` = (0, 2, 10, 12) and
    /// categorical parameter `cat` = (a, a, b, b).
    fn view_with_category() -> StudyView {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let param_names = vec!["x".to_string(), "cat".to_string()];
        let rows: Vec<CoreRow> = vec![
            (0u32, 0.0, "a"),
            (1, 2.0, "a"),
            (2, 10.0, "b"),
            (3, 12.0, "b"),
        ]
        .into_iter()
        .map(|(id, x, cat)| CoreRow {
            trial_id: id,
            trial_number: id,
            param_display: HashMap::from([("x".to_string(), x)]),
            param_category_label: HashMap::from([("cat".to_string(), cat.to_string())]),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
        let df = DataFrame::from_trials(&rows, &param_names, &[], &[], &[], 0);
        StudyView::new(Arc::new(df), vec![])
    }

    /// Builds a `StudyView` with numeric parameter `x` and categorical parameter `cat`
    /// containing: level "a" (2 values), level "b" (2 values), level "c" (1 value — too
    /// few to estimate), and an empty-string level (2 values). The attempted-group count
    /// is therefore 4 while only 2 curves can be built.
    fn view_with_sparse_category() -> StudyView {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let param_names = vec!["x".to_string(), "cat".to_string()];
        let rows: Vec<CoreRow> = vec![
            (0u32, 0.0, "a"),
            (1, 2.0, "a"),
            (2, 10.0, "b"),
            (3, 12.0, "b"),
            (4, 5.0, "c"),
            (5, 7.0, ""),
            (6, 9.0, ""),
        ]
        .into_iter()
        .map(|(id, x, cat)| CoreRow {
            trial_id: id,
            trial_number: id,
            param_display: HashMap::from([("x".to_string(), x)]),
            param_category_label: HashMap::from([("cat".to_string(), cat.to_string())]),
            objective_values: vec![],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
        let df = DataFrame::from_trials(&rows, &param_names, &[], &[], &[], 0);
        StudyView::new(Arc::new(df), vec![])
    }

    /// Builds a `StudyView` whose only objective column `x` is constant (`5.0`). The
    /// column is numeric (so it is a candidate), but `compute_violin` returns `None`,
    /// so the widget must fall back to the empty state rather than draw a violin.
    fn view_with_constant_objective() -> StudyView {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let obj_names = vec!["x".to_string()];
        let rows: Vec<CoreRow> = (0u32..3)
            .map(|id| CoreRow {
                trial_id: id,
                trial_number: id,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: vec![5.0],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&rows, &[], &obj_names, &[], &[], 0);
        StudyView::new(Arc::new(df), vec![])
    }

    #[test]
    fn build_curves_category_mode_normalizes_whole_column_before_splitting() {
        let view = view_with_category();
        let (curves, _) = build_curves(&view, &["x".to_string()], Some("cat"), "x", true);

        // One curve per category level, in sorted label order.
        let labels: Vec<&str> = curves.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, vec!["a", "b"]);

        // The global column min/max (0 / 12) are used, so level "a" spans
        // [0, 1/6] and level "b" spans [5/6, 1] — not [0, 1] each, which per-group
        // normalization would produce.
        let (_, a) = &curves[0];
        assert!((a.data_min - 0.0).abs() < 1e-12);
        assert!((a.data_max - 2.0 / 12.0).abs() < 1e-12);
        let (_, b) = &curves[1];
        assert!((b.data_min - 10.0 / 12.0).abs() < 1e-12);
        assert!((b.data_max - 1.0).abs() < 1e-12);
    }

    /// The attempted count returned by `build_curves` must count every distinct
    /// non-empty level (even ones too small to estimate) plus one for the empty label,
    /// so that `attempted - curves.len()` reports the same skipped count as before.
    #[test]
    fn build_curves_reports_attempted_groups_including_sparse_levels() {
        let view = view_with_sparse_category();
        let (curves, attempted) = build_curves(&view, &["x".to_string()], Some("cat"), "x", false);

        // Levels a, b and c are attempted; the empty label counts as one more group.
        assert_eq!(attempted, 4);
        // Only a and b have >= 2 finite values, so c and the empty label are skipped.
        let labels: Vec<&str> = curves.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, vec!["a", "b"]);
        assert_eq!(attempted - curves.len(), 2);
    }

    /// A constant numeric column is a valid candidate, so the mode attempts one group,
    /// but no curve can be built. Pinning `attempted == 1` with no curves keeps the
    /// skip/message accounting correct: the widget falls through to the empty state
    /// (not the "1 group(s) skipped" note, which needs a non-empty `curves`).
    #[test]
    fn build_curves_constant_column_yields_no_curves_but_counts_attempt() {
        let view = view_with_constant_objective();
        let (curves, attempted) = build_curves(&view, &["x".to_string()], None, "x", false);
        assert!(curves.is_empty());
        assert_eq!(attempted, 1);
    }

    /// The attempted count is stored alongside the curves in the cache, so the render
    /// path never rescans the category column. It must match the previous note behavior
    /// in category mode, including a level with fewer than 2 finite values.
    #[test]
    fn cached_attempted_count_matches_category_levels_with_a_sparse_level() {
        let view = view_with_sparse_category();
        let param_names = vec!["x".to_string(), "cat".to_string()];
        let obj_names: Vec<String> = vec![];
        let mut chart = ViolinPlotChart {
            source: ViolinSource::Parameters,
            category: Some("cat".to_string()),
            selected_numeric: "x".to_string(),
            ..Default::default()
        };

        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                chart.show(ui, &view, &param_names, &obj_names, "study");
            },
        );

        let (_, curves, attempted) = chart.cache.as_ref().expect("cache should be populated");
        assert_eq!(*attempted, 4);
        assert_eq!(curves.len(), 2);
        assert_eq!(*attempted - curves.len(), 2);
    }

    #[test]
    fn density_scale_gives_every_curve_the_same_max_width() {
        let narrow = compute_violin(&[0.0, 1.0, 2.0, 3.0, 4.0], GRID_POINTS).unwrap();
        let wide = compute_violin(&[0.0, 0.1, 0.2, 5.0, 10.0], GRID_POINTS).unwrap();
        for curve in [&narrow, &wide] {
            let max_density = curve.density.iter().copied().fold(0.0_f64, f64::max);
            let max_width = max_density * density_scale(curve);
            assert!((max_width - HALF_WIDTH).abs() < 1e-12);
        }
    }

    /// Builds a `StudyView` with objective `x` and numeric parameters `x` and `y`. The
    /// name `x` existing in both sources is the overlap that used to poison the cache:
    /// the objective list `["x"]` was cached under the `Parameters` key, where the list
    /// is `["x", "y"]`.
    fn view_with_overlapping_name() -> StudyView {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};

        let param_names = vec!["x".to_string(), "y".to_string()];
        let obj_names = vec!["x".to_string()];
        let rows: Vec<CoreRow> = vec![
            (0u32, 0.0, 1.0),
            (1, 2.0, 4.0),
            (2, 10.0, 2.0),
            (3, 12.0, 8.0),
        ]
        .into_iter()
        .map(|(id, x, y)| CoreRow {
            trial_id: id,
            trial_number: id,
            param_display: HashMap::from([("x".to_string(), x), ("y".to_string(), y)]),
            param_category_label: HashMap::new(),
            objective_values: vec![x],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
        let df = DataFrame::from_trials(&rows, &param_names, &obj_names, &[], &[], 0);
        StudyView::new(Arc::new(df), vec![])
    }

    /// Finds the first AccessKit node matching `role`, and `label`/`value` when those
    /// are given. Returns the node id and its on-screen rect.
    fn accesskit_node(
        output: &egui::FullOutput,
        role: egui::accesskit::Role,
        label: Option<&str>,
        value: Option<&str>,
    ) -> Option<(egui::accesskit::NodeId, egui::Rect)> {
        let update = output.platform_output.accesskit_update.as_ref()?;
        update.nodes.iter().find_map(|(id, node)| {
            if node.role() != role {
                return None;
            }
            if label.is_some_and(|l| node.label() != Some(l)) {
                return None;
            }
            if value.is_some_and(|v| node.value() != Some(v)) {
                return None;
            }
            let bounds = node.bounds()?;
            Some((
                *id,
                egui::Rect::from_min_max(
                    egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                    egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                ),
            ))
        })
    }

    /// A frame that sends an AccessKit `Click` action to `node`, which egui translates
    /// into a normal widget click.
    fn accesskit_click_frame(screen: egui::Rect, node: egui::accesskit::NodeId) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    action: egui::accesskit::Action::Click,
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: node,
                    data: None,
                },
            )],
            ..Default::default()
        }
    }

    /// Regression test for the source-switch cache poisoning: switching the Source combo
    /// from Objectives to Parameters must derive the columns *after* the combo has been
    /// drawn, so the cache holds the freshly selected source's columns.
    #[test]
    fn switching_source_rebuilds_cache_with_fresh_columns() {
        let view = view_with_overlapping_name();
        let param_names = vec!["x".to_string(), "y".to_string()];
        let obj_names = vec!["x".to_string()];
        let mut chart = ViolinPlotChart {
            source: ViolinSource::Objectives,
            selected_numeric: "x".to_string(),
            ..Default::default()
        };

        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let run = |chart: &mut ViolinPlotChart, input: egui::RawInput| {
            ctx.run_ui(input, |ui| {
                chart.show(ui, &view, &param_names, &obj_names, "study");
            })
        };
        let idle = || egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };

        // Frame 1: the Source combo is drawn closed; locate its button.
        let out = run(&mut chart, idle());
        let (source_combo, _) = accesskit_node(
            &out,
            egui::accesskit::Role::ComboBox,
            None,
            Some("Objectives"),
        )
        .expect("source combo should be visible");

        // Frame 2: click the Source combo to open its menu. The menu items are drawn in
        // the same frame, so they are available from this frame's output.
        let out = run(&mut chart, accesskit_click_frame(screen, source_combo));
        assert_eq!(
            chart.source,
            ViolinSource::Objectives,
            "opening the combo must not change the source"
        );
        let (parameters_item, _) = accesskit_node(
            &out,
            egui::accesskit::Role::Button,
            Some("Parameters"),
            None,
        )
        .expect("Parameters menu item should be visible while the menu is open");

        // Frame 3: click "Parameters". The columns used to build the cache must be the
        // Parameters columns derived in this same frame (["x", "y"]), not the Objectives
        // column (["x"]) computed before the combo was drawn.
        let _ = run(&mut chart, accesskit_click_frame(screen, parameters_item));

        assert_eq!(chart.source, ViolinSource::Parameters);
        let (key, curves, _) = chart.cache.as_ref().expect("cache should be populated");
        assert_eq!(key.1, ViolinSource::Parameters.disc());
        let labels: Vec<&str> = curves.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            labels,
            vec!["x", "y"],
            "the cache must hold the currently selected source's columns"
        );
    }

    /// Regression test for the empty-state message: a constant column has plenty of
    /// values but no variance, so `compute_violin` returns `None` and the widget must
    /// show the empty state. The exact wording is asserted through AccessKit so the
    /// test fails if the message regresses (e.g. back to the old "need at least 2
    /// values" text, which was false for this input).
    #[test]
    fn constant_column_renders_empty_state_message() {
        const MESSAGE: &str =
            "Need at least 2 finite, non-identical values to estimate a distribution.";

        let view = view_with_constant_objective();
        let param_names: Vec<String> = vec![];
        let obj_names = vec!["x".to_string()];
        let mut chart = ViolinPlotChart {
            source: ViolinSource::Objectives,
            selected_numeric: "x".to_string(),
            ..Default::default()
        };

        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                chart.show(ui, &view, &param_names, &obj_names, "study");
            },
        );

        // The cache must hold the attempt with no curves, so the empty state is what
        // was actually rendered (not, say, the plot frame).
        let (_, curves, attempted) = chart.cache.as_ref().expect("cache should be populated");
        assert!(curves.is_empty());
        assert_eq!(*attempted, 1);

        // egui exposes a `RichText` label as a Label node whose text is in `value`
        // (the `label` accessor is left unset). Asserting the exact wording makes the
        // test fail if the message regresses.
        assert!(
            accesskit_node(&out, egui::accesskit::Role::Label, None, Some(MESSAGE)).is_some(),
            "the empty-state label must be exposed via AccessKit with the exact wording"
        );
    }
}
