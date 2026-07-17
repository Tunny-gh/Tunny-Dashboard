use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::results::{McdmMethod, McdmResult};
use crate::state::types::{ColormapName, StudyView};
use crate::theme::chart_colors::{COLOR_EMPTY_STATE, COLOR_INFEASIBLE, COLOR_MCDM_NONE};
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::common::heatmap::draw_gradient_bar;
use crate::ui::widgets::mcdm_chart::McdmControls;
use crate::ui::widgets::mcdm_scatter_chart::{
    build_rank_map, extract_axis_values, fallback_axis_id, get_axis_options, mcdm_rank_color,
    ranked_hash,
};
use crate::ui::widgets::scatter_3d::{
    draw_3d_axes, draw_3d_grid, draw_depth_sorted_points, normalize_to_clip, setup_3d_canvas,
    show_hover_and_click_detail, val_range, ArcballCamera, DepthPoint,
};
use crate::ui::widgets::trial_detail_modal::{fmt_opt, TrialDetailModal};
use egui::Color32;

// ── Cache ─────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Eq)]
struct CacheKey {
    trial_count: usize,
    x_axis: String,
    y_axis: String,
    z_axis: String,
    colormap_name: ColormapName,
    top_n: usize,
    result_method: McdmMethod,
    /// FNV-like hash of ranked_indices.
    /// The old implementation used scores[0], but for trials outside the
    /// Pareto front, `expand_scores` always sets it to 0.0, so weight changes
    /// could not be detected.
    ranked_indices_hash: u64,
}

/// Cache of points already converted to clip-space coordinates [-1,1], color,
/// and row index. The row index is kept to identify the trial when a point is
/// clicked.
struct PointsCache {
    clip_pts: Vec<([f32; 3], Color32, usize)>,
    infeasible_clip_pts: Vec<([f32; 3], usize)>,
    x_range: (f64, f64),
    y_range: (f64, f64),
    z_range: (f64, f64),
}

// ── Widget ────────────────────────────────────────────────────────

/// MCDM 3D scatter plot widget.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct McdmScatterChart3D {
    /// MCDM settings and execution state (method / weights / Run, etc.)
    pub controls: McdmControls,
    pub x_axis: String,
    pub y_axis: String,
    pub z_axis: String,
    pub camera: ArcballCamera,
    /// Whether to show infeasible solutions (only relevant for constrained studies)
    pub show_infeasible: bool,
    #[serde(skip)]
    cache: Option<PointsCache>,
    #[serde(skip)]
    cache_key: Option<CacheKey>,
    /// Trial detail modal opened by clicking a point
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
}

impl Default for McdmScatterChart3D {
    fn default() -> Self {
        Self {
            controls: McdmControls::default(),
            x_axis: "Objective0".to_string(),
            y_axis: "Objective1".to_string(),
            z_axis: "Objective2".to_string(),
            camera: ArcballCamera::isometric_default(),
            show_infeasible: true,
            cache: None,
            cache_key: None,
            detail_modal: TrialDetailModal::new(),
        }
    }
}

impl McdmScatterChart3D {
    /// Pulls in the global widget's MCDM execution state (for each canvas item).
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    /// `ranked_indices()` hash shared with the 2D version (unified 2D/3D in H-3).
    fn ranked_hash(result: &McdmResult) -> u64 {
        ranked_hash(result)
    }

    fn is_cache_stale(
        &self,
        trial_count: usize,
        result: &McdmResult,
        colormap_name: &ColormapName,
        top_n: usize,
    ) -> bool {
        match &self.cache_key {
            None => true,
            Some(k) => {
                k.trial_count != trial_count
                    || k.x_axis != self.x_axis
                    || k.y_axis != self.y_axis
                    || k.z_axis != self.z_axis
                    || k.colormap_name != *colormap_name
                    || k.top_n != top_n
                    || k.result_method != result.method()
                    || k.ranked_indices_hash != Self::ranked_hash(result)
            }
        }
    }

    fn rebuild_cache(
        &mut self,
        result: &McdmResult,
        view: &StudyView,
        obj_names: &[String],
        colormap: &ColorMap,
        colormap_name: &ColormapName,
        top_n: usize,
    ) -> Result<(), String> {
        let n_trials = view.row_count();
        let x_vals = extract_axis_values(&self.x_axis, result, view, obj_names)?;
        let y_vals = extract_axis_values(&self.y_axis, result, view, obj_names)?;
        let z_vals = extract_axis_values(&self.z_axis, result, view, obj_names)?;

        let x_range = val_range(&x_vals);
        let y_range = val_range(&y_vals);
        let z_range = val_range(&z_vals);

        // ranked_indices → rank_map (shared with 2D, D-6)
        let rank_map = build_rank_map(result.ranked_indices(), n_trials);
        let colored_range = top_n.max(1);

        let feas = view.feasibility();
        let mut clip_pts: Vec<([f32; 3], Color32, usize)> = Vec::with_capacity(n_trials);
        let mut infeasible_clip_pts: Vec<([f32; 3], usize)> = Vec::new();

        for (i, &rank) in rank_map.iter().enumerate() {
            let x = match x_vals.get(i).copied() {
                Some(v) if v.is_finite() => v,
                _ => continue,
            };
            let y = match y_vals.get(i).copied() {
                Some(v) if v.is_finite() => v,
                _ => continue,
            };
            let z = match z_vals.get(i).copied() {
                Some(v) if v.is_finite() => v,
                _ => continue,
            };
            let cx = normalize_to_clip(x, x_range.0, x_range.1);
            let cy = normalize_to_clip(y, y_range.0, y_range.1);
            let cz = normalize_to_clip(z, z_range.0, z_range.1);

            if !feas.is_feasible(i) {
                infeasible_clip_pts.push(([cx, cy, cz], i));
                continue;
            }

            // rank -> color (colormap within top_n, gray outside it; shared with 2D, D-6)
            let color = mcdm_rank_color(rank, colored_range, colormap);
            clip_pts.push(([cx, cy, cz], color, i));
        }

        self.cache = Some(PointsCache {
            clip_pts,
            infeasible_clip_pts,
            x_range,
            y_range,
            z_range,
        });
        self.cache_key = Some(CacheKey {
            trial_count: n_trials,
            x_axis: self.x_axis.clone(),
            y_axis: self.y_axis.clone(),
            z_axis: self.z_axis.clone(),
            colormap_name: colormap_name.clone(),
            top_n,
            result_method: result.method(),
            ranked_indices_hash: Self::ranked_hash(result),
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        mcdm_result: Option<&McdmResult>,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        colormap: &ColorMap,
        colormap_name: &ColormapName,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
    ) {
        if !self.controls.show_controls(ui, obj_names, "mcdm_scatter3d") {
            return;
        }
        if self.controls.computing {
            return;
        }
        let top_n = self.controls.top_n.value();

        let Some(result) = mcdm_result else {
            ui.centered_and_justified(|ui| {
                ui.colored_label(COLOR_EMPTY_STATE(), "Press Run to compute the MCDM ranking");
            });
            return;
        };

        let options = get_axis_options(result, obj_names);

        // Reset to default axis (when an invalid ID is currently selected)
        if !options.iter().any(|o| o.id == self.x_axis) {
            self.x_axis = fallback_axis_id(&options, 0);
            self.cache_key = None;
        }
        if !options.iter().any(|o| o.id == self.y_axis) {
            self.y_axis = fallback_axis_id(&options, 1);
            self.cache_key = None;
        }
        if !options.iter().any(|o| o.id == self.z_axis) {
            self.z_axis = fallback_axis_id(&options, 2);
            self.cache_key = None;
        }

        // Axis selectors
        ui.horizontal(|ui| {
            ui.label("X:");
            egui::ComboBox::from_id_salt("mcdm3d_x")
                .selected_text(&self.x_axis)
                .show_ui(ui, |ui| {
                    for opt in &options {
                        ui.selectable_value(&mut self.x_axis, opt.id.clone(), &opt.label);
                    }
                });
            ui.label("Y:");
            egui::ComboBox::from_id_salt("mcdm3d_y")
                .selected_text(&self.y_axis)
                .show_ui(ui, |ui| {
                    for opt in &options {
                        ui.selectable_value(&mut self.y_axis, opt.id.clone(), &opt.label);
                    }
                });
            ui.label("Z:");
            egui::ComboBox::from_id_salt("mcdm3d_z")
                .selected_text(&self.z_axis)
                .show_ui(ui, |ui| {
                    for opt in &options {
                        ui.selectable_value(&mut self.z_axis, opt.id.clone(), &opt.label);
                    }
                });
        });

        let n_trials = view.row_count();
        let has_constraints = view.feasibility().has_constraints();

        // Rebuild cache
        if self.is_cache_stale(n_trials, result, colormap_name, top_n) {
            if let Err(e) =
                self.rebuild_cache(result, view, obj_names, colormap, colormap_name, top_n)
            {
                ui.colored_label(ERROR_COLOR(), e);
                return;
            }
        }

        if has_constraints {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            });
        }

        // Finish camera manipulation before borrowing the cache
        let (painter, rect, project, click_pos, hover_pos) = setup_3d_canvas(ui, &mut self.camera);

        // For left-click point hit testing (trial_id, row, and screen coords of drawn points)
        let mut candidates: Vec<(u32, usize, egui::Pos2)> = Vec::new();
        let has_infeasible;
        {
            let Some(pc) = &self.cache else {
                return;
            };
            let (x_min, x_max) = pc.x_range;
            let (y_min, y_max) = pc.y_range;
            let (z_min, z_max) = pc.z_range;
            let show_infeasible = self.show_infeasible;
            has_infeasible = show_infeasible && !pc.infeasible_clip_pts.is_empty();

            draw_3d_grid(&painter, &project);
            draw_3d_axes(
                &painter,
                &project,
                [&self.x_axis, &self.y_axis, &self.z_axis],
                [(x_min, x_max), (y_min, y_max), (z_min, z_max)],
            );

            candidates.reserve(pc.clip_pts.len() + pc.infeasible_clip_pts.len());

            // Draw infeasible solutions at the very back
            if has_infeasible {
                let mut inf_pts: Vec<DepthPoint> = Vec::with_capacity(pc.infeasible_clip_pts.len());
                for &(clip, row) in &pc.infeasible_clip_pts {
                    let (pos, depth) = project(clip);
                    let trial_id = view.trial_ids.get(row).copied().unwrap_or(row as u32);
                    candidates.push((trial_id, row, pos));
                    inf_pts.push(DepthPoint {
                        pos,
                        depth,
                        color: COLOR_INFEASIBLE(),
                        radius: 3.0,
                    });
                }
                draw_depth_sorted_points(&painter, &mut inf_pts, None);
            }

            // Draw feasible solutions back-to-front (painter's algorithm)
            let mut pts: Vec<DepthPoint> = Vec::with_capacity(pc.clip_pts.len());
            for &(clip, color, row) in &pc.clip_pts {
                let (pos, depth) = project(clip);
                let trial_id = view.trial_ids.get(row).copied().unwrap_or(row as u32);
                candidates.push((trial_id, row, pos));
                pts.push(DepthPoint {
                    pos,
                    depth,
                    color,
                    radius: 3.5,
                });
            }
            draw_depth_sorted_points(&painter, &mut pts, None);
        }

        // ── Top-right colorbar legend ───────────────────────────────
        draw_colorbar_legend(&painter, rect, colormap, top_n, has_infeasible);

        // Build MCDM rank/score rows (hover and click show the same content).
        let rank_score_rows = |row: usize| -> Vec<(String, String)> {
            let rank = result
                .ranked_indices()
                .iter()
                .position(|&x| x as usize == row);
            let rank_str = rank
                .map(|r| (r + 1).to_string())
                .unwrap_or_else(|| "—".to_string());
            let score = result.primary_scores().get(row).copied();
            vec![
                ("MCDM Rank".to_string(), rank_str),
                ("Score".to_string(), fmt_opt(score)),
            ]
        };
        show_hover_and_click_detail(
            ui,
            view,
            &candidates,
            hover_pos,
            click_pos,
            "mcdm3d_hover_tooltip",
            &mut self.detail_modal,
            rank_score_rows,
            rank_score_rows,
        );

        // Draw the detail modal.
        if self.detail_modal.is_open() {
            self.detail_modal
                .show(ui, view, param_names, obj_names, artifact_map);
        }
    }
}

/// Draws the colorbar legend in the top-right of the 3D canvas.
fn draw_colorbar_legend(
    painter: &egui::Painter,
    rect: egui::Rect,
    colormap: &ColorMap,
    top_n: usize,
    has_infeasible: bool,
) {
    const BAR_W: f32 = 12.0;
    const BAR_H: f32 = 90.0;
    const PADDING: f32 = 8.0;
    const TEXT_X: f32 = BAR_W + 4.0;
    const FONT_SZ: f32 = 10.0;
    const N_SEGS: usize = 24;

    // Total legend height (colorbar + Others + optional Infeasible)
    let row_h = 16.0_f32;
    let extra_rows = 1 + if has_infeasible { 1 } else { 0 };
    let legend_h = BAR_H + row_h * extra_rows as f32 + PADDING * 2.0;
    let legend_w = 100.0_f32;

    let origin = egui::pos2(rect.right() - legend_w - PADDING, rect.top() + PADDING);

    // Semi-transparent background
    painter.rect_filled(
        egui::Rect::from_min_size(origin, egui::vec2(legend_w, legend_h)),
        4.0,
        egui::Color32::from_rgba_unmultiplied(20, 20, 20, 160),
    );

    let bar_x = origin.x + PADDING;
    let bar_y = origin.y + PADDING;

    // Colorbar (top = Rank 1 = t=1.0, bottom = Rank top_n = t=0.0). The bar body
    // rendering is shared via `common::heatmap::draw_gradient_bar` (D-10). No border.
    let bar_rect = egui::Rect::from_min_size(egui::pos2(bar_x, bar_y), egui::vec2(BAR_W, BAR_H));
    draw_gradient_bar(painter, bar_rect, colormap, N_SEGS);

    let text_color = egui::Color32::from_rgb(220, 220, 220);
    let font = egui::FontId::proportional(FONT_SZ);

    // Top label (best)
    painter.text(
        egui::pos2(bar_x + TEXT_X, bar_y),
        egui::Align2::LEFT_TOP,
        "Rank 1 (Best)",
        font.clone(),
        text_color,
    );
    // Bottom label (lowest colored rank)
    painter.text(
        egui::pos2(bar_x + TEXT_X, bar_y + BAR_H),
        egui::Align2::LEFT_BOTTOM,
        format!("Rank {top_n}"),
        font.clone(),
        text_color,
    );

    // Others row
    let others_y = bar_y + BAR_H + 4.0;
    painter.circle_filled(
        egui::pos2(bar_x + BAR_W * 0.5, others_y + row_h * 0.5),
        4.0,
        COLOR_MCDM_NONE(),
    );
    painter.text(
        egui::pos2(bar_x + TEXT_X, others_y + row_h * 0.5),
        egui::Align2::LEFT_CENTER,
        "Others",
        font.clone(),
        text_color,
    );

    // Infeasible row
    if has_infeasible {
        let inf_y = others_y + row_h;
        painter.circle_filled(
            egui::pos2(bar_x + BAR_W * 0.5, inf_y + row_h * 0.5),
            4.0,
            crate::theme::chart_colors::COLOR_INFEASIBLE(),
        );
        painter.text(
            egui::pos2(bar_x + TEXT_X, inf_y + row_h * 0.5),
            egui::Align2::LEFT_CENTER,
            "Infeasible",
            font,
            text_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::results::{McdmResult, TopsisResult};

    fn make_topsis(ranked: Vec<u32>) -> McdmResult {
        McdmResult::Topsis(TopsisResult {
            scores: vec![0.0; ranked.len().max(1)],
            ranked_indices: ranked,
            duration_ms: 0.0,
        })
    }

    #[test]
    fn ranked_hash_changes_when_order_changes() {
        let r1 = make_topsis(vec![5, 2, 8]);
        let r2 = make_topsis(vec![2, 5, 8]);
        assert_ne!(
            McdmScatterChart3D::ranked_hash(&r1),
            McdmScatterChart3D::ranked_hash(&r2),
            "ランキング順序が変わればハッシュが変わる"
        );
    }

    #[test]
    fn ranked_hash_stable_for_same_order() {
        let r1 = make_topsis(vec![5, 2, 8]);
        let r2 = make_topsis(vec![5, 2, 8]);
        assert_eq!(
            McdmScatterChart3D::ranked_hash(&r1),
            McdmScatterChart3D::ranked_hash(&r2)
        );
    }

    #[test]
    fn cache_stale_after_ranking_change() {
        let mut w = McdmScatterChart3D::default();
        // Artificially set cache_key to simulate the "old ranking" state
        w.cache_key = Some(CacheKey {
            trial_count: 10,
            x_axis: w.x_axis.clone(),
            y_axis: w.y_axis.clone(),
            z_axis: w.z_axis.clone(),
            colormap_name: crate::state::types::ColormapName::Viridis,
            top_n: 10,
            result_method: crate::state::results::McdmMethod::Topsis,
            ranked_indices_hash: McdmScatterChart3D::ranked_hash(&make_topsis(vec![5, 2, 8])),
        });
        // New result with a changed ranking
        let new_result = make_topsis(vec![2, 5, 8]);
        assert!(
            w.is_cache_stale(
                10,
                &new_result,
                &crate::state::types::ColormapName::Viridis,
                10
            ),
            "ランキング変更でキャッシュが無効化される"
        );
    }

    #[test]
    fn cache_not_stale_for_same_ranking() {
        let mut w = McdmScatterChart3D::default();
        let result = make_topsis(vec![5, 2, 8]);
        w.cache_key = Some(CacheKey {
            trial_count: 10,
            x_axis: w.x_axis.clone(),
            y_axis: w.y_axis.clone(),
            z_axis: w.z_axis.clone(),
            colormap_name: crate::state::types::ColormapName::Viridis,
            top_n: 10,
            result_method: crate::state::results::McdmMethod::Topsis,
            ranked_indices_hash: McdmScatterChart3D::ranked_hash(&result),
        });
        assert!(
            !w.is_cache_stale(10, &result, &crate::state::types::ColormapName::Viridis, 10),
            "同じランキングならキャッシュは有効"
        );
    }

    #[test]
    fn mcdm_scatter_3d_default_axes() {
        let w = McdmScatterChart3D::default();
        assert_eq!(w.x_axis, "Objective0");
        assert_eq!(w.y_axis, "Objective1");
        assert_eq!(w.z_axis, "Objective2");
        assert!(w.show_infeasible);
        assert!(w.cache.is_none());
        assert!(w.cache_key.is_none());
    }

    #[test]
    fn val_range_basic() {
        let (mn, mx) = val_range(&[1.0, 3.0, 2.0]);
        assert!((mn - 1.0).abs() < 1e-10);
        assert!((mx - 3.0).abs() < 1e-10);
    }

    #[test]
    fn val_range_empty_returns_fallback() {
        let (mn, mx) = val_range(&[]);
        assert!((mn - (-1.0)).abs() < 1e-10);
        assert!((mx - 1.0).abs() < 1e-10);
    }

    #[test]
    fn val_range_equal_values_expands() {
        let (mn, mx) = val_range(&[5.0, 5.0]);
        assert!(mn < 5.0);
        assert!(mx > 5.0);
    }
}
