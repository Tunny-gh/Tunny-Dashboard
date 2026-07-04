//! 応答曲面 3D ビューア。
//!
//! `robustness.rs` と同じ 2 段階構成: サロゲートのフィットは非同期（poll_chart 経由）、
//! フィット済みモデルからのスライス評価（`tunny_core::surrogate_opt::surface_slice_at`）は
//! ミリ秒オーダーのためレンダーパスで同期実行しキャッシュする。アンカー点
//! （Best trial / pin 留めした trial）を通る 2 パラメータ平面のスライスを 3D メッシュで
//! 表示する（PDP のような周辺化はせず、他パラメータをアンカー値に固定した「生の断面」）。
//! アンカー点の選択は `robustness.rs` と共通の `anchor::CenterChoice` を使う。
//!
//! 描画は `pdp_2d.rs` のサーフェスメッシュ共有描画（`draw_surface_mesh` ほか）を再利用する。

use std::sync::Arc;

use tunny_core::surrogate_opt::{
    SurfaceSlice, SurrogateModelKind, TrainedSurrogate, MIN_TRIALS_FOR_SURROGATE_OPT,
};

use super::anchor::{center_label, resolve_center, CenterChoice};
use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::COLOR_CONTOUR;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::pdp_2d::{
    band_grids, draw_surface_mesh, extract_observed_3d, value_range_of,
};
use crate::ui::widgets::scatter_3d::{
    axis_segments_3d, draw_3d_axis_labels, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    ArcballCamera,
};

/// モデル選択肢（コンボ表示順）。`robustness.rs` と揃える。
const MODEL_CHOICES: [SurrogateModelKind; 5] = [
    SurrogateModelKind::Ridge,
    SurrogateModelKind::GpFitc,
    SurrogateModelKind::GpVfe,
    SurrogateModelKind::GpMoe,
    SurrogateModelKind::Lgbm,
];

/// グリッド解像度の選択肢。
const GRID_CHOICES: [usize; 3] = [20, 30, 50];

/// フィット段階の計算リクエスト。poll_chart が消費する。
pub struct ResponseSurfaceFitRequest {
    pub objective_index: usize,
    pub model: SurrogateModelKind,
}

/// キャッシュキー: (学習済みモデルのポインタ恒等性, x_idx, y_idx, アンカーのビット表現, n_grid)。
type SliceCacheKey = (usize, usize, usize, Vec<u64>, usize);

/// 応答曲面 3D ウィジェットの UI 状態。
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ResponseSurfaceChart {
    pub selected_objective: usize,
    pub model: SurrogateModelKind,
    pub anchor: CenterChoice,
    pub param_x: String,
    pub param_y: String,
    pub n_grid: usize,
    /// GP 系のみ有効な ±1.96σ 帯表示トグル。
    pub show_uncertainty: bool,
    pub show_observed: bool,
    pub camera: ArcballCamera,

    #[serde(skip)]
    pub trained: Option<Arc<TrainedSurrogate>>,
    #[serde(skip)]
    pub fitting: bool,
    #[serde(skip)]
    pub fit_error: Option<String>,
    #[serde(skip)]
    pub pending_fit: Option<ResponseSurfaceFitRequest>,
    #[serde(skip)]
    cache: Option<(SliceCacheKey, SurfaceSlice)>,
}

impl Default for ResponseSurfaceChart {
    fn default() -> Self {
        Self {
            selected_objective: 0,
            model: SurrogateModelKind::GpFitc,
            anchor: CenterChoice::default(),
            param_x: String::new(),
            param_y: String::new(),
            n_grid: 30,
            show_uncertainty: true,
            show_observed: true,
            camera: ArcballCamera::isometric_default(),
            trained: None,
            fitting: false,
            fit_error: None,
            pending_fit: None,
            cache: None,
        }
    }
}

impl ResponseSurfaceChart {
    /// グローバル widget の計算実行状態・結果・エラーを取り込む
    /// （`ComputeSyncKind::ResponseSurfaceFit` から呼ぶ。`robustness.rs` と同じ規約）。
    pub fn adopt_compute_state(&mut self, global: &Self) {
        self.trained = global.trained.clone();
        self.fitting = global.fitting;
        self.fit_error = global.fit_error.clone();
    }

    /// 直近のスライス評価結果（キャッシュ）。CSV エクスポート等が参照する。
    pub fn cached_slice(&self) -> Option<&SurfaceSlice> {
        self.cache.as_ref().map(|(_, s)| s)
    }
}

fn cache_key(
    trained: &Arc<TrainedSurrogate>,
    x_idx: usize,
    y_idx: usize,
    anchor: &[f64],
    n_grid: usize,
) -> SliceCacheKey {
    (
        Arc::as_ptr(trained) as usize,
        x_idx,
        y_idx,
        anchor.iter().map(|v| v.to_bits()).collect(),
        n_grid,
    )
}

impl ResponseSurfaceChart {
    /// `obj_names` / `directions` は現在の Study の全目的（Best trial 解決用）。
    /// `param_names` は数値パラメータ一覧（X/Y コンボの候補）。
    /// `pinned_trials` は pin 留めした trial_id（Anchor コンボの候補）。
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        param_names: &[String],
        obj_names: &[String],
        directions: &[Direction],
        trial_count: usize,
        pinned_trials: &[u32],
        cmap: &ColorMap,
    ) {
        if obj_names.is_empty() {
            ui.label("No objectives available.");
            return;
        }
        if self.selected_objective >= obj_names.len() {
            self.selected_objective = 0;
        }

        ui.horizontal(|ui| {
            ui.label("Objective:");
            egui::ComboBox::from_id_salt("response_surface_obj")
                .selected_text(obj_names[self.selected_objective].as_str())
                .show_ui(ui, |ui| {
                    for (i, name) in obj_names.iter().enumerate() {
                        ui.selectable_value(&mut self.selected_objective, i, name);
                    }
                });

            ui.label("Model:");
            egui::ComboBox::from_id_salt("response_surface_model")
                .selected_text(super::surrogate_opt::model_label(self.model))
                .show_ui(ui, |ui| {
                    for kind in MODEL_CHOICES {
                        ui.selectable_value(
                            &mut self.model,
                            kind,
                            super::surrogate_opt::model_label(kind),
                        );
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("X:");
            egui::ComboBox::from_id_salt("response_surface_x")
                .selected_text(self.param_x.as_str())
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.param_x, name.clone(), name);
                    }
                });
            ui.label("Y:");
            egui::ComboBox::from_id_salt("response_surface_y")
                .selected_text(self.param_y.as_str())
                .show_ui(ui, |ui| {
                    for name in param_names {
                        ui.selectable_value(&mut self.param_y, name.clone(), name);
                    }
                });
            ui.label("Grid:");
            egui::ComboBox::from_id_salt("response_surface_grid")
                .selected_text(self.n_grid.to_string())
                .show_ui(ui, |ui| {
                    for n in GRID_CHOICES {
                        ui.selectable_value(&mut self.n_grid, n, n.to_string());
                    }
                });
        });

        if !self.param_x.is_empty() && self.param_x == self.param_y {
            ui.colored_label(COLOR_CONTOUR, "Warning: X and Y must differ");
        }

        ui.horizontal(|ui| {
            ui.label("Anchor:");
            let anchor_text = center_label(self.anchor, view);
            egui::ComboBox::from_id_salt("response_surface_anchor")
                .selected_text(anchor_text)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.anchor, CenterChoice::BestTrial, "Best trial");
                    for &trial_id in pinned_trials {
                        let Some(row) = view.trial_ids.iter().position(|&t| t == trial_id) else {
                            continue;
                        };
                        let number = view.df.get_trial_number(row).unwrap_or(trial_id);
                        ui.selectable_value(
                            &mut self.anchor,
                            CenterChoice::Pinned(trial_id),
                            format!("Trial #{number}"),
                        );
                    }
                });
            ui.toggle_value(&mut self.show_observed, "Show data");
        });

        if trial_count < MIN_TRIALS_FOR_SURROGATE_OPT {
            ui.label(
                egui::RichText::new(format!(
                    "At least {} trials required (current: {})",
                    MIN_TRIALS_FOR_SURROGATE_OPT, trial_count
                ))
                .weak(),
            );
            return;
        }

        let can_fit = !self.fitting && self.pending_fit.is_none();
        if ui
            .add_enabled(can_fit, egui::Button::new("Fit Surrogate"))
            .clicked()
        {
            self.fit_error = None;
            self.fitting = true;
            self.pending_fit = Some(ResponseSurfaceFitRequest {
                objective_index: self.selected_objective,
                model: self.model,
            });
        }
        if self.fitting {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Fitting surrogate...");
            });
        }
        if let Some(err) = self.fit_error.clone() {
            ui.colored_label(egui::Color32::RED, err);
        }

        let Some(trained) = self.trained.clone() else {
            return;
        };

        if self.param_x.is_empty() || self.param_y.is_empty() || self.param_x == self.param_y {
            return;
        }
        let Some(x_idx) = trained.param_names.iter().position(|p| p == &self.param_x) else {
            ui.colored_label(
                egui::Color32::RED,
                "Selected X parameter is not part of the trained model.",
            );
            return;
        };
        let Some(y_idx) = trained.param_names.iter().position(|p| p == &self.param_y) else {
            ui.colored_label(
                egui::Color32::RED,
                "Selected Y parameter is not part of the trained model.",
            );
            return;
        };

        let Some(anchor) = resolve_center(&trained, self.anchor, view, obj_names, directions)
        else {
            ui.colored_label(
                egui::Color32::RED,
                "Could not resolve the anchor point for the trained parameters.",
            );
            return;
        };

        let key = cache_key(&trained, x_idx, y_idx, &anchor, self.n_grid);
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            self.cache = tunny_core::surrogate_opt::surface_slice_at(
                &trained,
                &anchor,
                x_idx,
                y_idx,
                self.n_grid,
            )
            .map(|s| (key, s));
        }

        if self.cache.is_none() {
            ui.colored_label(egui::Color32::RED, "Response surface evaluation failed.");
            return;
        }

        // 不確実性バンド表示トグル（ガウス過程系のみ。cache の不変借用前に self を可変借用する）。
        let has_uncertainty = self.cache.as_ref().is_some_and(|(_, s)| s.z_std.is_some());
        if has_uncertainty {
            ui.checkbox(&mut self.show_uncertainty, "95% CI (±1.96σ)");
        }

        let anchor_text = center_label(self.anchor, view);
        let show_uncertainty = self.show_uncertainty;
        let show_observed = self.show_observed;
        let param_x = self.param_x.clone();
        let param_y = self.param_y.clone();
        let objective_name = obj_names[self.selected_objective].clone();
        let camera = &mut self.camera;
        // `camera`（self.camera への可変借用）と `slice`（self.cache への不変借用）は
        // 互いに素なフィールドなので同時に借用できる（pdp_2d.rs と同じパターン）。
        let (_, slice) = self.cache.as_ref().expect("checked non-empty above");

        let (c_min, c_max) = value_range_of(&slice.z_values);
        let mut v_min = c_min;
        let mut v_max = c_max;

        let bands = if show_uncertainty {
            slice
                .z_std
                .as_ref()
                .map(|std_grid| band_grids(&slice.z_values, std_grid))
        } else {
            None
        };
        if let Some((lower, upper)) = &bands {
            let (l_min, _) = value_range_of(lower);
            let (_, u_max) = value_range_of(upper);
            v_min = v_min.min(l_min);
            v_max = v_max.max(u_max);
        }

        let observed = if show_observed {
            extract_observed_3d(
                view,
                &param_x,
                &param_y,
                &objective_name,
                &[],
                pinned_trials,
            )
        } else {
            vec![]
        };
        for (p, _) in &observed {
            v_min = v_min.min(p[2]);
            v_max = v_max.max(p[2]);
        }

        let (x_min, x_max) = value_range_of(std::slice::from_ref(&slice.x_values));
        let (y_min, y_max) = value_range_of(std::slice::from_ref(&slice.y_values));

        let observed_clip: Vec<([f32; 3], egui::Color32)> = observed
            .iter()
            .map(|&([px, py, ov], kind)| {
                (
                    [
                        normalize_to_clip(px, x_min, x_max),
                        normalize_to_clip(ov, v_min, v_max),
                        normalize_to_clip(py, y_min, y_max),
                    ],
                    kind.color(),
                )
            })
            .collect();

        let avail = ui.available_size();
        let canvas_size = egui::vec2((avail.x - 16.0).max(120.0), avail.y.max(160.0));
        ui.allocate_ui(canvas_size, |ui| {
            ui.set_min_size(canvas_size);
            let (painter, _rect, project, _click_pos, _hover_pos) = setup_3d_canvas(ui, camera);
            draw_3d_grid(&painter, &project);
            draw_surface_mesh(
                &painter,
                &project,
                &slice.z_values,
                (v_min, v_max),
                (c_min, c_max),
                cmap,
                bands.as_ref().map(|(lower, upper)| (lower, upper)),
                &observed_clip,
                &axis_segments_3d(24),
            );
            draw_3d_axis_labels(
                &painter,
                &project,
                [&param_x, &objective_name, &param_y],
                [(x_min, x_max), (v_min, v_max), (y_min, y_max)],
            );
        });

        ui.label(
            egui::RichText::new(format!(
                "Slice through {anchor_text} (other parameters fixed)"
            ))
            .weak(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_surface_chart_default_values() {
        let s = ResponseSurfaceChart::default();
        assert_eq!(s.selected_objective, 0);
        assert_eq!(s.anchor, CenterChoice::BestTrial);
        assert_eq!(s.n_grid, 30);
        assert!(s.show_uncertainty);
        assert!(s.show_observed);
        assert!(s.trained.is_none());
        assert!(!s.fitting);
        assert!(s.pending_fit.is_none());
        assert!(s.cached_slice().is_none());
        assert!(!s.camera.is_identity_rotation());
    }

    #[test]
    fn adopt_compute_state_propagates_and_keeps_selection() {
        let src = ResponseSurfaceChart {
            fitting: false,
            fit_error: Some("err".into()),
            ..Default::default()
        };
        let mut dst = ResponseSurfaceChart {
            fitting: true,
            selected_objective: 2,
            param_x: "x1".to_string(),
            ..Default::default()
        };
        dst.adopt_compute_state(&src);
        assert!(!dst.fitting);
        assert_eq!(dst.fit_error.as_deref(), Some("err"));
        // UI 選択は維持される。
        assert_eq!(dst.selected_objective, 2);
        assert_eq!(dst.param_x, "x1");
    }
}
