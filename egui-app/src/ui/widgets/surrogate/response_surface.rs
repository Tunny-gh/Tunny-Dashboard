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

use std::collections::HashMap;
use std::sync::Arc;

use tunny_core::surrogate_opt::{
    SurfaceSlice, SurrogateModelKind, TrainedSurrogate, MIN_TRIALS_FOR_SURROGATE_OPT,
};

use super::anchor::{center_label, resolve_center, CenterChoice};
use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyView};
use crate::theme::chart_colors::COLOR_CONTOUR;
use crate::theme::colormap::ColorMap;
use crate::ui::widgets::pdp_2d::{
    band_grids, draw_surface_mesh, extract_observed_3d, value_range_of,
};
use crate::ui::widgets::pdp_chart::classify_observed;
use crate::ui::widgets::scatter_3d::{
    axis_segments_3d, draw_3d_axis_labels, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    show_hover_and_click_detail, ArcballCamera,
};
use crate::ui::widgets::trial_detail_modal::TrialDetailModal;

// モデル選択肢（コンボ表示順）。3 ウィジェット共通の単一情報源（`super::MODEL_CHOICES`）を使う。
use super::MODEL_CHOICES;

/// グリッド解像度の選択肢。
const GRID_CHOICES: [usize; 3] = [20, 30, 50];

/// GP 系モデルのスライス評価はグリッド点数の二乗に比例して重い（50²=2500 点予測）。
/// 描画パス同期実行での UI ブロックを避けるため、GP 系ではグリッド解像度をこの値で頭打ちにする。
/// Ridge / LightGBM は安価なため制限しない。
const GP_GRID_CAP: usize = 30;

/// GP（ガウス過程）系モデルか。応答曲面スライスの計算コストが高いモデル群。
fn is_gp_kind(kind: SurrogateModelKind) -> bool {
    matches!(
        kind,
        SurrogateModelKind::GpFitc | SurrogateModelKind::GpVfe | SurrogateModelKind::GpMoe
    )
}

/// フィット段階の計算リクエスト。poll_chart が消費する。
pub struct ResponseSurfaceFitRequest {
    pub objective_index: usize,
    pub model: SurrogateModelKind,
}

/// キャッシュキー: (フィット世代 ID, x_idx, y_idx, アンカーのビット表現, n_grid)。
/// 先頭要素は以前 `Arc::as_ptr` だったが、解放後に同一アドレスが再利用されると
/// 別モデルの結果を誤表示しうる（ABA）。フィット採用時に単調増加する世代 ID
/// （`ResponseSurfaceChart::fit_generation`）へ置き換えて回避する。
type SliceCacheKey = (u64, usize, usize, Vec<u64>, usize);

/// アンカー解決結果のキャッシュキー: (フィット世代 ID, アンカー選択, DataFrame 恒等性)。
/// 中心点解決（`resolve_center`）は全 trial を走査する O(N) 処理のため、
/// 入力が変わらないフレームでは再走査を避ける。
type AnchorCacheKey = (u64, CenterChoice, usize);

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
    /// アンカー解決結果のキャッシュ（毎フレームの O(N) 走査回避）。
    #[serde(skip)]
    anchor_cache: Option<(AnchorCacheKey, Vec<f64>)>,
    /// フィット採用時に単調増加する世代 ID。キャッシュキーの `Arc::as_ptr` 置換用。
    #[serde(skip)]
    fit_generation: u64,
    /// 直近フレームで観測した学習済みモデルの Arc ポインタ（世代 ID 更新の変化検出用）。
    #[serde(skip)]
    fit_ptr: usize,
    /// 観測点クリックで開くトライアル詳細モーダル。
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
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
            anchor_cache: None,
            fit_generation: 0,
            fit_ptr: 0,
            detail_modal: TrialDetailModal::new(),
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
    fit_generation: u64,
    x_idx: usize,
    y_idx: usize,
    anchor: &[f64],
    n_grid: usize,
) -> SliceCacheKey {
    (
        fit_generation,
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
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
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
            ui.colored_label(COLOR_CONTOUR(), "Warning: X and Y must differ");
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

        // フィット採用（trained の Arc が別モデルへ差し替わった）を検出して世代 ID を進める。
        // キャッシュキーはこの世代 ID を使い、`Arc::as_ptr` のアドレス再利用（ABA）を避ける。
        let trained_ptr = Arc::as_ptr(&trained) as usize;
        if trained_ptr != self.fit_ptr {
            self.fit_ptr = trained_ptr;
            self.fit_generation = self.fit_generation.wrapping_add(1);
        }

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

        // アンカー解決は全 trial を走査する O(N) 処理。入力（世代・選択・DataFrame）が
        // 変わらないフレームでは前回結果を再利用する。
        let anchor_key: AnchorCacheKey = (
            self.fit_generation,
            self.anchor,
            Arc::as_ptr(&view.df) as usize,
        );
        if self.anchor_cache.as_ref().map(|(k, _)| k) != Some(&anchor_key) {
            self.anchor_cache = resolve_center(&trained, self.anchor, view, obj_names, directions)
                .map(|a| (anchor_key, a));
        }
        let Some((_, anchor)) = self.anchor_cache.as_ref() else {
            ui.colored_label(
                egui::Color32::RED,
                "Could not resolve the anchor point for the trained parameters.",
            );
            return;
        };
        let anchor = anchor.clone();

        // GP 系はグリッド点数の二乗に比例して重いため、描画パス同期実行の UI ブロックを
        // 抑えるようスライス解像度を頭打ちにする（Ridge / LightGBM は制限なし）。
        let effective_grid = if is_gp_kind(trained.model_kind) {
            self.n_grid.min(GP_GRID_CAP)
        } else {
            self.n_grid
        };

        let key = cache_key(self.fit_generation, x_idx, y_idx, &anchor, effective_grid);
        if self.cache.as_ref().map(|(k, _)| k) != Some(&key) {
            self.cache = tunny_core::surrogate_opt::surface_slice_at(
                &trained,
                &anchor,
                x_idx,
                y_idx,
                effective_grid,
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
        let detail_modal = &mut self.detail_modal;
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
        for (_, p, _) in &observed {
            v_min = v_min.min(p[2]);
            v_max = v_max.max(p[2]);
        }

        let (x_min, x_max) = value_range_of(std::slice::from_ref(&slice.x_values));
        let (y_min, y_max) = value_range_of(std::slice::from_ref(&slice.y_values));

        let observed_clip: Vec<([f32; 3], egui::Color32)> = observed
            .iter()
            .map(|&(_, [px, py, ov], kind)| {
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

        // 下に続くアンカー説明キャプション 1 行ぶんを先に差し引いてから
        // 3D キャンバスを確保する（キャプションの見切れ防止）。
        let caption_h = ui.text_style_height(&egui::TextStyle::Body) + ui.spacing().item_spacing.y;
        let avail = ui.available_size();
        let canvas_size = egui::vec2(
            (avail.x - 16.0).max(120.0),
            (avail.y - caption_h).max(160.0),
        );
        // ホバーツールチップ・クリック詳細用の列参照（観測点は実トライアル）。
        let px_col = view.numeric_column(&param_x);
        let py_col = view.numeric_column(&param_y);
        let obj_col = view.numeric_column(&objective_name);
        let feas = view.feasibility();

        ui.allocate_ui(canvas_size, |ui| {
            ui.set_min_size(canvas_size);
            let (painter, _rect, project, click_pos, hover_pos) = setup_3d_canvas(ui, camera);
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

            // 観測点のホバーツールチップ・クリック詳細（他の 3D 散布図と同じ操作感）。
            // "Show data" オフ時は observed が空のため何も起きない。
            let candidates: Vec<(u32, usize, egui::Pos2)> = observed
                .iter()
                .zip(observed_clip.iter())
                .filter_map(|(&(row, _, _), &(clip, _))| {
                    let (pos, _) = project(clip);
                    if !pos.x.is_finite() || !pos.y.is_finite() {
                        return None;
                    }
                    let trial_id = view.trial_ids.get(row).copied().unwrap_or(row as u32);
                    Some((trial_id, row, pos))
                })
                .collect();
            let fmt = |v: Option<f64>| v.map(|x| format!("{x:.4}")).unwrap_or_else(|| "—".into());
            show_hover_and_click_detail(
                ui,
                view,
                &candidates,
                hover_pos,
                click_pos,
                "response_surface_hover_tooltip",
                &mut *detail_modal,
                |row| {
                    vec![
                        (
                            param_x.clone(),
                            fmt(px_col.and_then(|c| c.get(row)).copied()),
                        ),
                        (
                            param_y.clone(),
                            fmt(py_col.and_then(|c| c.get(row)).copied()),
                        ),
                        (
                            objective_name.clone(),
                            fmt(obj_col.and_then(|c| c.get(row)).copied()),
                        ),
                    ]
                },
                |row| {
                    let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
                    vec![(
                        "Status".to_string(),
                        classify_observed(feas.is_feasible(row), rank)
                            .label()
                            .to_string(),
                    )]
                },
            );
        });

        ui.label(
            egui::RichText::new(format!(
                "Slice through {anchor_text} (other parameters fixed)"
            ))
            .weak(),
        );

        // クリックで開いたトライアル詳細モーダルを描画する。
        if detail_modal.is_open() {
            detail_modal.show(ui, view, param_names, obj_names, artifact_map);
        }
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
        assert_ne!(s.camera.rotation, [0.0, 0.0, 0.0, 1.0]);
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
