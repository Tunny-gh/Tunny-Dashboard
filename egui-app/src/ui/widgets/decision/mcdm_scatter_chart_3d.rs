use crate::state::results::{McdmMethod, McdmResult};
use crate::state::types::{ColormapName, StudyView};
use crate::theme::chart_colors::{COLOR_EMPTY_STATE, COLOR_INFEASIBLE, COLOR_MCDM_NONE};
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::mcdm_chart::McdmControls;
use crate::ui::widgets::mcdm_scatter_chart::{extract_axis_values, get_axis_options};
use crate::ui::widgets::scatter_3d::{
    draw_3d_axes, draw_3d_grid, normalize_to_clip, setup_3d_canvas, ArcballCamera,
};
use egui::Color32;

// ── キャッシュ ────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Eq)]
struct CacheKey {
    trial_count: usize,
    x_axis: String,
    y_axis: String,
    z_axis: String,
    colormap_name: ColormapName,
    top_n: usize,
    result_method: McdmMethod,
    /// ranked_indices の FNV ライクなハッシュ。
    /// 旧実装は scores[0] を使っていたが、パレートフロント外の試行は
    /// expand_scores により常に 0.0 になり、ウェイト変更を検知できなかった。
    ranked_indices_hash: u64,
}

/// clip 空間座標 [-1,1] と色に変換済みのポイントキャッシュ
struct PointsCache {
    clip_pts: Vec<([f32; 3], Color32)>,
    infeasible_clip_pts: Vec<[f32; 3]>,
    x_range: (f64, f64),
    y_range: (f64, f64),
    z_range: (f64, f64),
}

// ── ウィジェット ──────────────────────────────────────────────────

/// MCDM 3D 散布図ウィジェット
pub struct McdmScatterChart3D {
    /// MCDM 設定・実行状態（手法 / 重み / Run など）
    pub controls: McdmControls,
    pub x_axis: String,
    pub y_axis: String,
    pub z_axis: String,
    pub camera: ArcballCamera,
    /// 実行不可能解を表示するか（制約あり Study でのみ有効）
    pub show_infeasible: bool,
    cache: Option<PointsCache>,
    cache_key: Option<CacheKey>,
}

impl Default for McdmScatterChart3D {
    fn default() -> Self {
        Self {
            controls: McdmControls::default(),
            x_axis: "Objective0".to_string(),
            y_axis: "Objective1".to_string(),
            z_axis: "Objective2".to_string(),
            camera: ArcballCamera {
                rotation: [-0.2391, 0.3696, 0.0990, 0.8924],
                ..Default::default()
            },
            show_infeasible: true,
            cache: None,
            cache_key: None,
        }
    }
}

fn val_range(vals: &[f64]) -> (f64, f64) {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for &v in vals {
        if v.is_finite() {
            if v < mn {
                mn = v;
            }
            if v > mx {
                mx = v;
            }
        }
    }
    if !mn.is_finite() || !mx.is_finite() {
        (-1.0, 1.0)
    } else if (mx - mn).abs() < f64::EPSILON {
        (mn - 1.0, mx + 1.0)
    } else {
        (mn, mx)
    }
}

impl McdmScatterChart3D {
    /// グローバル widget の MCDM 実行状態を取り込む（キャンバスの各アイテム用）。
    pub fn adopt_compute_state(&mut self, src: &Self) {
        self.controls.adopt_compute_state(&src.controls);
    }

    fn ranked_hash(result: &McdmResult) -> u64 {
        result.ranked_indices().iter().fold(0u64, |acc, &x| {
            acc.wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(x as u64 + 1)
        })
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

        // ranked_indices → rank_map
        let ranked = result.ranked_indices();
        let mut rank_map = vec![usize::MAX; n_trials];
        for (rank, &idx) in ranked.iter().enumerate() {
            let i = idx as usize;
            if i < n_trials {
                rank_map[i] = rank;
            }
        }
        let colored_range = top_n.max(1);

        let is_feasible_col = view.numeric_column("is_feasible");
        let mut clip_pts: Vec<([f32; 3], Color32)> = Vec::with_capacity(n_trials);
        let mut infeasible_clip_pts: Vec<[f32; 3]> = Vec::new();

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

            let feasible = is_feasible_col
                .and_then(|c| c.get(i))
                .map(|&v| v > 0.5)
                .unwrap_or(true);

            if !feasible {
                infeasible_clip_pts.push([cx, cy, cz]);
                continue;
            }

            let color = if rank == usize::MAX || rank >= colored_range {
                COLOR_MCDM_NONE
            } else {
                let t = if colored_range > 1 {
                    1.0 - rank as f32 / (colored_range - 1) as f32
                } else {
                    1.0
                };
                colormap.interpolate(t)
            };
            clip_pts.push(([cx, cy, cz], color));
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
        obj_names: &[String],
        colormap: &ColorMap,
        colormap_name: &ColormapName,
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
                ui.colored_label(COLOR_EMPTY_STATE, "Press Run to compute the MCDM ranking");
            });
            return;
        };

        let options = get_axis_options(result, obj_names);

        // デフォルト軸のリセット（無効なIDを選択中の場合）
        if !options.iter().any(|o| o.id == self.x_axis) {
            self.x_axis = options.first().map(|o| o.id.clone()).unwrap_or_default();
            self.cache_key = None;
        }
        if !options.iter().any(|o| o.id == self.y_axis) {
            self.y_axis = options
                .get(1)
                .or_else(|| options.first())
                .map(|o| o.id.clone())
                .unwrap_or_default();
            self.cache_key = None;
        }
        if !options.iter().any(|o| o.id == self.z_axis) {
            self.z_axis = options
                .get(2)
                .or_else(|| options.first())
                .map(|o| o.id.clone())
                .unwrap_or_default();
            self.cache_key = None;
        }

        // 軸セレクタ
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
        let has_constraints = view.numeric_column("is_feasible").is_some();

        // キャッシュ再構築
        if self.is_cache_stale(n_trials, result, colormap_name, top_n) {
            if let Err(e) =
                self.rebuild_cache(result, view, obj_names, colormap, colormap_name, top_n)
            {
                ui.colored_label(ERROR_COLOR, e);
                return;
            }
        }

        if has_constraints {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            });
        }

        // カメラ操作はキャッシュ借用前に完了させる
        let (painter, rect, project) = setup_3d_canvas(ui, &mut self.camera);

        let Some(pc) = &self.cache else {
            return;
        };
        let (x_min, x_max) = pc.x_range;
        let (y_min, y_max) = pc.y_range;
        let (z_min, z_max) = pc.z_range;
        let show_infeasible = self.show_infeasible;

        draw_3d_grid(&painter, &project);
        draw_3d_axes(
            &painter,
            &project,
            [&self.x_axis, &self.y_axis, &self.z_axis],
            [(x_min, x_max), (y_min, y_max), (z_min, z_max)],
        );

        // 実行不可能解を最背面に描画
        if show_infeasible && !pc.infeasible_clip_pts.is_empty() {
            let mut inf_pts: Vec<(egui::Pos2, f32)> = pc
                .infeasible_clip_pts
                .iter()
                .map(|&clip| project(clip))
                .collect();
            inf_pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            for (pos, _) in &inf_pts {
                painter.circle_filled(*pos, 3.0, COLOR_INFEASIBLE);
            }
        }

        // 実行可能解を奥から手前の順（ペインターズアルゴリズム）
        let mut pts: Vec<(egui::Pos2, f32, Color32)> = pc
            .clip_pts
            .iter()
            .map(|&(clip, color)| {
                let (pos, depth) = project(clip);
                (pos, depth, color)
            })
            .collect();
        pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _, color) in &pts {
            painter.circle_filled(*pos, 3.5, *color);
        }

        // ── 右上カラーバー判例 ────────────────────────────────────
        draw_colorbar_legend(
            &painter,
            rect,
            colormap,
            top_n,
            show_infeasible && !pc.infeasible_clip_pts.is_empty(),
        );
    }
}

/// 3D キャンバスの右上にカラーバー判例を描画する
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
    const N_SEGS: i32 = 24;

    // 判例全体の高さ（カラーバー＋Others＋オプションのInfeasible）
    let row_h = 16.0_f32;
    let extra_rows = 1 + if has_infeasible { 1 } else { 0 };
    let legend_h = BAR_H + row_h * extra_rows as f32 + PADDING * 2.0;
    let legend_w = 100.0_f32;

    let origin = egui::pos2(rect.right() - legend_w - PADDING, rect.top() + PADDING);

    // 半透明背景
    painter.rect_filled(
        egui::Rect::from_min_size(origin, egui::vec2(legend_w, legend_h)),
        4.0,
        egui::Color32::from_rgba_unmultiplied(20, 20, 20, 160),
    );

    let bar_x = origin.x + PADDING;
    let bar_y = origin.y + PADDING;

    // カラーバー（上 = Rank 1 = t=1.0、下 = Rank top_n = t=0.0）
    for seg in 0..N_SEGS {
        let t = 1.0 - seg as f32 / (N_SEGS - 1) as f32;
        let color = colormap.interpolate(t);
        let seg_h = BAR_H / N_SEGS as f32;
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(bar_x, bar_y + seg as f32 * seg_h),
                egui::vec2(BAR_W, seg_h + 0.5),
            ),
            0.0,
            color,
        );
    }

    let text_color = egui::Color32::from_rgb(220, 220, 220);
    let font = egui::FontId::proportional(FONT_SZ);

    // 上ラベル（最良）
    painter.text(
        egui::pos2(bar_x + TEXT_X, bar_y),
        egui::Align2::LEFT_TOP,
        "Rank 1 (Best)",
        font.clone(),
        text_color,
    );
    // 下ラベル（最下位着色）
    painter.text(
        egui::pos2(bar_x + TEXT_X, bar_y + BAR_H),
        egui::Align2::LEFT_BOTTOM,
        format!("Rank {top_n}"),
        font.clone(),
        text_color,
    );

    // Others 行
    let others_y = bar_y + BAR_H + 4.0;
    painter.circle_filled(
        egui::pos2(bar_x + BAR_W * 0.5, others_y + row_h * 0.5),
        4.0,
        COLOR_MCDM_NONE,
    );
    painter.text(
        egui::pos2(bar_x + TEXT_X, others_y + row_h * 0.5),
        egui::Align2::LEFT_CENTER,
        "Others",
        font.clone(),
        text_color,
    );

    // Infeasible 行
    if has_infeasible {
        let inf_y = others_y + row_h;
        painter.circle_filled(
            egui::pos2(bar_x + BAR_W * 0.5, inf_y + row_h * 0.5),
            4.0,
            crate::theme::chart_colors::COLOR_INFEASIBLE,
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
            positive_ideal: vec![],
            negative_ideal: vec![],
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
        // 擬似的に cache_key をセットして "旧ランキング" 状態を作る
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
        // ランキングが変わった新結果
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
