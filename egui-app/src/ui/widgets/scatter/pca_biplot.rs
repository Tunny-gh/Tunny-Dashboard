//! PCA biplot — a widget that projects trials onto the 1st and 2nd principal
//! components of standardized PCA, overlaying the original variables' contributions
//! (loadings) as arrows.
//!
//! The PCA computation itself is handled by `tunny_core::clustering::run_pca_standardized`,
//! which reads the currently active Study's DataFrame directly (via `with_active_df`).
//! As a sync widget it is called every time in the draw path, and the result is
//! cached, keyed by (df's Arc identity, Study name, row count, target space), holding
//! just one entry. Draw artifacts such as colored point groups and loadings arrows
//! additionally depend on the colormap and coloring objective, so they are cached
//! together under a separate key.
//! See `theory/{en,ja}/clustering/pca-biplot.md` for details.

use std::collections::BTreeMap;

use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_EMPTY_STATE, COLOR_SCATTER_DOT};
use crate::theme::color_compute::{key_to_color32, rgba_key};
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::common::range_math::finite_value_range;
use tunny_core::clustering::PcaResult;

/// The target space for PCA. Since `tunny_core::clustering::PcaSpace` doesn't
/// implement serde, this mirror enum is provided for UI-state persistence and
/// converted via `to_core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum PcaSpaceOption {
    #[default]
    Param,
    Objective,
    All,
}

impl PcaSpaceOption {
    pub fn label(self) -> &'static str {
        match self {
            PcaSpaceOption::Param => "Parameters",
            PcaSpaceOption::Objective => "Objectives",
            PcaSpaceOption::All => "All",
        }
    }

    pub fn to_core(self) -> tunny_core::clustering::PcaSpace {
        match self {
            PcaSpaceOption::Param => tunny_core::clustering::PcaSpace::Param,
            PcaSpaceOption::Objective => tunny_core::clustering::PcaSpace::Objective,
            PcaSpaceOption::All => tunny_core::clustering::PcaSpace::All,
        }
    }

    /// Discriminant used for the cache key.
    fn disc(self) -> u8 {
        match self {
            PcaSpaceOption::Param => 0,
            PcaSpaceOption::Objective => 1,
            PcaSpaceOption::All => 2,
        }
    }
}

/// UI state for the PCA biplot widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PcaBiplotChart {
    pub space: PcaSpaceOption,
    pub show_loadings: bool,
    /// The objective function name used for continuous coloring. `None` means a
    /// single color.
    pub color_objective: Option<String>,
    /// Cache of the PCA computation and its draw artifacts.
    #[serde(skip)]
    cache: Option<PcaBiplotCache>,
}

/// Caches the PCA computation (`PcaResult`) together with its draw artifacts (color
/// groups, loadings arrows).
///
/// The PCA computation is only recomputed when the df's identity, Study, row count,
/// or target space changes. Draw artifacts additionally depend on the colormap and
/// coloring objective, so when those change, only the draw artifacts are rebuilt
/// while keeping the PCA computation (M-17).
struct PcaBiplotCache {
    /// Key for the PCA computation: (df identity, Study name, row count, target space
    /// discriminant).
    pca_key: (usize, String, usize, u8),
    result: PcaResult,
    /// Key for the draw artifacts: (colormap fingerprint, coloring objective).
    draw_key: (u64, Option<String>),
    draw: PcaDraw,
}

/// Draw artifacts for the PCA biplot (cached to avoid rebuilding every frame, M-17).
struct PcaDraw {
    /// Colored point groups (color -> list of projected coordinates).
    color_groups: BTreeMap<[u8; 4], Vec<[f64; 2]>>,
    /// Start points (always the origin) and end points of the loadings arrows.
    loading_origins: Vec<[f64; 2]>,
    loading_tips: Vec<[f64; 2]>,
    /// Axis labels including the explained variance ratio.
    x_label: String,
    y_label: String,
}

impl Default for PcaBiplotChart {
    fn default() -> Self {
        Self {
            space: PcaSpaceOption::default(),
            show_loadings: true,
            color_objective: None,
            cache: None,
        }
    }
}

impl PcaBiplotChart {
    /// Returns the cached PCA result, for CSV export.
    pub fn cached_result(&self) -> Option<&PcaResult> {
        self.cache.as_ref().map(|c| &c.result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        obj_names: &[String],
        cmap: &ColorMap,
        study_name: &str,
    ) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("pca_biplot_space")
                .selected_text(self.space.label())
                .show_ui(ui, |ui| {
                    for space in [
                        PcaSpaceOption::Param,
                        PcaSpaceOption::Objective,
                        PcaSpaceOption::All,
                    ] {
                        ui.selectable_value(&mut self.space, space, space.label());
                    }
                });
            ui.checkbox(&mut self.show_loadings, "Show loadings");

            ui.label("Color by:");
            let color_label = self.color_objective.as_deref().unwrap_or("None");
            egui::ComboBox::from_id_salt("pca_biplot_color")
                .selected_text(color_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.color_objective, None, "None");
                    for name in obj_names {
                        ui.selectable_value(
                            &mut self.color_objective,
                            Some(name.clone()),
                            name.as_str(),
                        );
                    }
                });
        });

        // Key for the PCA computation: df's Arc identity + Study name + row count +
        // target space (low-priority finding).
        // Including identity avoids mixing up different data even when switching to
        // another dataset with the same (Study name, row count, space).
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;
        let pca_key = (
            df_ptr,
            study_name.to_string(),
            view.row_count(),
            self.space.disc(),
        );
        // Key for the draw artifacts: colormap + coloring objective (M-17).
        let draw_key = (
            super::rank_plot::cmap_fingerprint(cmap),
            self.color_objective.clone(),
        );

        // The PCA computation is only recomputed when pca_key changes. Draw
        // artifacts are rebuilt, keeping the PCA computation, only when draw_key
        // changes.
        let pca_valid = self.cache.as_ref().is_some_and(|c| c.pca_key == pca_key);
        if !pca_valid {
            self.cache = tunny_core::clustering::run_pca_standardized(2, self.space.to_core()).map(
                |result| {
                    let draw = compute_pca_draw(&result, &self.color_objective, view, cmap);
                    PcaBiplotCache {
                        pca_key,
                        result,
                        draw_key,
                        draw,
                    }
                },
            );
        } else if self.cache.as_ref().is_some_and(|c| c.draw_key != draw_key) {
            if let Some(c) = self.cache.as_mut() {
                c.draw = compute_pca_draw(&c.result, &self.color_objective, view, cmap);
                c.draw_key = draw_key;
            }
        }

        let Some(cache) = self.cache.as_ref() else {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "PCA needs at least 2 numeric columns and 2 trials.",
                );
            });
            return;
        };
        let result = &cache.result;
        if result.projections.is_empty() || result.loadings.len() < 2 {
            ui.vertical_centered(|ui| {
                ui.colored_label(
                    COLOR_EMPTY_STATE(),
                    "PCA needs at least 2 numeric columns and 2 trials.",
                );
            });
            return;
        }

        let draw = &cache.draw;
        let show_loadings = self.show_loadings;
        egui_plot::Plot::new("pca_biplot")
            .unified_nav()
            .x_axis_label(&draw.x_label)
            .y_axis_label(&draw.y_label)
            .data_aspect(1.0)
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                for (&key, pts) in &draw.color_groups {
                    let color = key_to_color32(key);
                    plot_ui.points(
                        egui_plot::Points::new("Trials", pts.clone())
                            .color(color)
                            .radius(3.0),
                    );
                }

                if show_loadings {
                    plot_ui.arrows(
                        egui_plot::Arrows::new(
                            "Loadings",
                            draw.loading_origins.clone(),
                            draw.loading_tips.clone(),
                        )
                        .color(crate::theme::chart_colors::COLOR_CONTOUR()),
                    );
                    for (j, name) in result.feature_names.iter().enumerate() {
                        if let Some(&[tx, ty]) = draw.loading_tips.get(j) {
                            plot_ui.text(egui_plot::Text::new(
                                "",
                                egui_plot::PlotPoint::new(tx, ty),
                                name.as_str(),
                            ));
                        }
                    }
                }
            });
    }
}

/// Builds the draw artifacts (colored point groups, loadings arrows, axis labels)
/// from the PCA computation (`result`) (M-17).
/// Separates out the part that depends only on colormap/coloring objective, and
/// rebuilds only when those change.
fn compute_pca_draw(
    result: &PcaResult,
    color_objective: &Option<String>,
    view: &StudyView,
    cmap: &ColorMap,
) -> PcaDraw {
    let color_col = color_objective
        .as_deref()
        .and_then(|name| view.numeric_column(name));
    let (color_min, color_max) = color_range(color_col);

    let mut color_groups: BTreeMap<[u8; 4], Vec<[f64; 2]>> = BTreeMap::new();
    for (i, row) in result.projections.iter().enumerate() {
        let x = row.first().copied().unwrap_or(0.0);
        let y = row.get(1).copied().unwrap_or(0.0);
        let color = match color_col {
            Some(col) => {
                let v = col.get(i).copied().unwrap_or(f64::NAN);
                if v.is_finite() && color_max > color_min {
                    let t = ((v - color_min) / (color_max - color_min)) as f32;
                    cmap.interpolate(t)
                } else {
                    COLOR_SCATTER_DOT()
                }
            }
            None => COLOR_SCATTER_DOT(),
        };
        let key = rgba_key(color);
        color_groups.entry(key).or_default().push([x, y]);
    }

    let max_abs_score = result
        .projections
        .iter()
        .flat_map(|row| row.iter())
        .fold(0.0f64, |acc, &v| acc.max(v.abs()))
        .max(1e-9);
    let loading_scale = 0.8 * max_abs_score;

    // Don't draw arrows in the degenerate case where loadings has fewer than 2
    // principal components (avoids an index panic).
    let (loading_origins, loading_tips) = if result.loadings.len() >= 2 {
        let origins = vec![[0.0, 0.0]; result.feature_names.len()];
        let tips: Vec<[f64; 2]> = (0..result.feature_names.len())
            .map(|j| {
                let lx = result.loadings[0].get(j).copied().unwrap_or(0.0);
                let ly = result.loadings[1].get(j).copied().unwrap_or(0.0);
                [lx * loading_scale, ly * loading_scale]
            })
            .collect();
        (origins, tips)
    } else {
        (Vec::new(), Vec::new())
    };

    let x_label = format!(
        "PC1 ({:.1}%)",
        result.explained_ratio.first().unwrap_or(&0.0) * 100.0
    );
    let y_label = format!(
        "PC2 ({:.1}%)",
        result.explained_ratio.get(1).unwrap_or(&0.0) * 100.0
    );

    PcaDraw {
        color_groups,
        loading_origins,
        loading_tips,
        x_label,
        y_label,
    }
}

/// Returns (min, max) over only the finite values of the continuous-coloring column.
/// Returns (0.0, 0.0) if there is no column.
fn color_range(col: Option<&[f64]>) -> (f64, f64) {
    let Some(col) = col else {
        return (0.0, 0.0);
    };
    finite_value_range(col.iter().copied()).unwrap_or((0.0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_param_space_with_loadings_and_no_color() {
        let chart = PcaBiplotChart::default();
        assert_eq!(chart.space, PcaSpaceOption::Param);
        assert!(chart.show_loadings);
        assert!(chart.color_objective.is_none());
        assert!(chart.cache.is_none());
    }

    #[test]
    fn space_disc_is_stable_and_distinct() {
        assert_eq!(PcaSpaceOption::Param.disc(), 0);
        assert_eq!(PcaSpaceOption::Objective.disc(), 1);
        assert_eq!(PcaSpaceOption::All.disc(), 2);
    }

    #[test]
    fn to_core_maps_each_variant() {
        assert_eq!(
            PcaSpaceOption::Param.to_core(),
            tunny_core::clustering::PcaSpace::Param
        );
        assert_eq!(
            PcaSpaceOption::Objective.to_core(),
            tunny_core::clustering::PcaSpace::Objective
        );
        assert_eq!(
            PcaSpaceOption::All.to_core(),
            tunny_core::clustering::PcaSpace::All
        );
    }

    #[test]
    fn color_range_ignores_non_finite() {
        let col = [1.0, f64::NAN, 5.0, f64::INFINITY, -2.0];
        assert_eq!(color_range(Some(&col)), (-2.0, 5.0));
    }

    #[test]
    fn color_range_none_column_is_zero_zero() {
        assert_eq!(color_range(None), (0.0, 0.0));
    }

    #[test]
    fn color_range_all_non_finite_is_zero_zero() {
        let col = [f64::NAN, f64::INFINITY];
        assert_eq!(color_range(Some(&col)), (0.0, 0.0));
    }
}
