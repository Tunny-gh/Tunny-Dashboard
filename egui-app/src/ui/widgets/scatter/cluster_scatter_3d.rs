use crate::state::app_state::AppState;
use crate::theme::chart_colors::{COLOR_INFEASIBLE, COLOR_NON_PARETO_DIM};
use crate::theme::colormap_name::colormap_from_name;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::cluster_scatter::{
    validate_cluster_request, ClusterCacheKey, ClusterComputeRequest, ClusterSpace,
    KMeansInitStrategy, KSelectionMode,
};
use crate::ui::widgets::scatter_3d::{
    compute_range_from_col, draw_3d_axes, draw_3d_grid, normalize_to_clip, setup_3d_canvas,
    show_hover_and_click_detail, show_objective_combo, ArcballCamera, Range3DCache,
};
use crate::ui::widgets::scatter_matrix::{downsample_indices_to_cap, MAX_SCATTER_POINTS};
use crate::ui::widgets::trial_detail_modal::TrialDetailModal;

/// クラスタ 3D 散布図ウィジェット
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClusterScatter3D {
    pub x_objective: usize,
    pub y_objective: usize,
    pub z_objective: usize,
    pub camera: ArcballCamera,
    pub show_infeasible: bool,
    // クラスタリング設定（2D の ClusterScatter と同じ操作を 3D からも可能にする）
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    /// Elbow（自動）モードで探索する k の上限。
    pub elbow_max_k: usize,
    #[serde(skip)]
    pub computing: bool,
    #[serde(skip)]
    pub pending_compute: Option<ClusterComputeRequest>,
    #[serde(skip)]
    pub last_error: Option<crate::state::messages::ClusterUiError>,
    #[serde(skip)]
    range_cache: Range3DCache<(usize, usize, usize, usize)>,
    /// 点クリックで開くトライアル詳細モーダル
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
}

impl Default for ClusterScatter3D {
    fn default() -> Self {
        Self {
            x_objective: 0,
            y_objective: 1,
            z_objective: 2,
            camera: ArcballCamera::isometric_default(),
            show_infeasible: true,
            k: 3,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
            computing: false,
            pending_compute: None,
            last_error: None,
            range_cache: Range3DCache::default(),
            detail_modal: TrialDetailModal::new(),
        }
    }
}

impl ClusterScatter3D {
    /// 現在の設定に対応するキャッシュキーを返す。
    pub fn cache_key(&self) -> ClusterCacheKey {
        ClusterCacheKey::new(
            self.target_space,
            self.k_mode,
            self.k,
            self.init_strategy,
            self.elbow_max_k,
        )
    }

    pub fn show(&mut self, ui: &mut egui::Ui, app_state: &mut AppState) {
        let Some(ctx) = &app_state.current_study else {
            ui.centered_and_justified(|ui| {
                ui.label("Select a study");
            });
            return;
        };
        let obj_names = &ctx.meta.objective_names;
        if obj_names.len() < 3 {
            ui.centered_and_justified(|ui| {
                ui.label("Need at least 3 objectives for 3D cluster view");
            });
            return;
        }

        let ctx = app_state.current_study.as_ref().unwrap();
        let obj_names = &ctx.meta.objective_names;
        let view = &ctx.view;
        let trial_count = view.row_count();
        let has_constraints = view.feasibility().has_constraints();
        // クラスタリング対象はパレートフロント（pareto_rank == 0）の解数で判定する。
        let pareto_count = view.pareto_rank.iter().filter(|&&r| r == 0).count();

        // クラスタリング設定 + Run（2D と同じ操作）
        self.show_cluster_controls(ui, pareto_count);
        if let Some(err) = self.last_error.clone() {
            ui.label(egui::RichText::new(&err.user_message).color(ERROR_COLOR()));
            if let Some(detail) = &err.detail_for_dev {
                ui.label(egui::RichText::new(detail).small().weak());
            }
            if err.retryable && ui.button("Retry").clicked() {
                self.try_queue_compute(pareto_count);
            }
        }

        // Range cache
        let cache_key = (
            self.x_objective,
            self.y_objective,
            self.z_objective,
            trial_count,
        );
        let col = |idx: usize| obj_names.get(idx).and_then(|n| view.numeric_column(n));
        let ranges = self.range_cache.get_or_compute(cache_key, || {
            [
                compute_range_from_col(col(self.x_objective)),
                compute_range_from_col(col(self.y_objective)),
                compute_range_from_col(col(self.z_objective)),
            ]
        });
        let [(x_min, x_max), (y_min, y_max), (z_min, z_max)] = ranges;

        let x_name = obj_names.get(self.x_objective).cloned().unwrap_or_default();
        let y_name = obj_names.get(self.y_objective).cloned().unwrap_or_default();
        let z_name = obj_names.get(self.z_objective).cloned().unwrap_or_default();

        // Column data
        let x_col = obj_names
            .get(self.x_objective)
            .and_then(|n| view.numeric_column(n));
        let y_col = obj_names
            .get(self.y_objective)
            .and_then(|n| view.numeric_column(n));
        let z_col = obj_names
            .get(self.z_objective)
            .and_then(|n| view.numeric_column(n));
        let feas = view.feasibility();

        // Axis selectors
        ui.horizontal(|ui| {
            show_objective_combo(ui, "X:", "clu3d_x", &mut self.x_objective, obj_names);
            show_objective_combo(ui, "Y:", "clu3d_y", &mut self.y_objective, obj_names);
            show_objective_combo(ui, "Z:", "clu3d_z", &mut self.z_objective, obj_names);
            if has_constraints {
                ui.separator();
                ui.checkbox(&mut self.show_infeasible, "Show Infeasible");
            }
        });

        // Cluster coloring（このチャート固有の設定キーでキャッシュを参照する）
        let cluster_key = self.cache_key();
        let cluster = app_state.cluster_cache.get(&cluster_key);
        let n_clusters = cluster.map(|r| r.n_clusters).unwrap_or(1).max(1);
        let has_cluster = cluster.is_some();
        let colormap = colormap_from_name(&app_state.selected_colormap);
        let cluster_color = |label: i32| -> egui::Color32 {
            if label < 0 {
                return egui::Color32::GRAY;
            }
            let t = if n_clusters == 1 {
                0.5_f32
            } else {
                (label as f32 / (n_clusters - 1) as f32).clamp(0.0, 1.0)
            };
            colormap.interpolate(t)
        };

        let (painter, rect, project, click_pos, hover_pos) = setup_3d_canvas(ui, &mut self.camera);

        draw_3d_grid(&painter, &project);
        draw_3d_axes(
            &painter,
            &project,
            [&x_name, &y_name, &z_name],
            [(x_min, x_max), (y_min, y_max), (z_min, z_max)],
        );

        // 全 trial を毎フレーム 3 回（infeasible / other / feasible）深度ソートするのは重いため、
        // 2D 系と同じ 1500 点上限で間引いてから描画・ソートする（M-13）。
        let all_indices: Vec<u32> = (0..trial_count as u32).collect();
        let displayed = downsample_indices_to_cap(&all_indices, MAX_SCATTER_POINTS);

        // Collect points
        let show_infeasible = self.show_infeasible;
        let mut feasible_pts: Vec<(egui::Pos2, f32, egui::Color32)> =
            Vec::with_capacity(displayed.len());
        let mut infeasible_pts: Vec<(egui::Pos2, f32)> = Vec::new();
        // クラスタリング対象外（非パレートフロント）の実行可能解 → 半透明で背面描画
        let mut other_pts: Vec<(egui::Pos2, f32)> = Vec::new();
        // 左クリックでの点ヒット判定用（描画した点の trial_id・行・スクリーン座標）
        let mut candidates: Vec<(u32, usize, egui::Pos2)> = Vec::with_capacity(displayed.len());

        for &idx in &displayed {
            let i = idx as usize;
            let xv = x_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let yv = y_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let zv = z_col.and_then(|c| c.get(i)).copied().unwrap_or(0.0);
            let clip = [
                normalize_to_clip(xv, x_min, x_max),
                normalize_to_clip(yv, y_min, y_max),
                normalize_to_clip(zv, z_min, z_max),
            ];
            let (pos, depth) = project(clip);
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);

            if !feas.is_feasible(i) {
                if show_infeasible {
                    infeasible_pts.push((pos, depth));
                    candidates.push((trial_id, i, pos));
                }
                continue;
            }

            candidates.push((trial_id, i, pos));

            let label = cluster.and_then(|r| r.labels.get(i).copied()).unwrap_or(0);

            if has_cluster && label < 0 {
                // クラスタリング済みだが非パレートフロント → 半透明で描画
                other_pts.push((pos, depth));
            } else {
                feasible_pts.push((pos, depth, cluster_color(label)));
            }
        }

        infeasible_pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _) in &infeasible_pts {
            painter.circle_filled(*pos, 3.0, COLOR_INFEASIBLE());
        }
        other_pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _) in &other_pts {
            painter.circle_filled(*pos, 2.5, COLOR_NON_PARETO_DIM());
        }
        feasible_pts.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (pos, _, color) in &feasible_pts {
            painter.circle_filled(*pos, 3.5, *color);
        }

        if !has_cluster && !self.computing {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Click Run to compute clusters",
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(180, 180, 180),
            );
        }

        // クラスタ番号（未クラスタリング・非パレートフロントは "—"）を行 index から求める。
        let cluster_str_for = |row: usize| -> String {
            let label = cluster
                .and_then(|r| r.labels.get(row).copied())
                .unwrap_or(-1);
            if has_cluster && label >= 0 {
                label.to_string()
            } else {
                "—".to_string()
            }
        };
        show_hover_and_click_detail(
            ui,
            view,
            &candidates,
            hover_pos,
            click_pos,
            "cluster3d_hover_tooltip",
            &mut self.detail_modal,
            |row| {
                let fmt =
                    |v: Option<f64>| v.map(|x| format!("{x:.4}")).unwrap_or_else(|| "—".into());
                let mut rows = vec![
                    (x_name.clone(), fmt(x_col.and_then(|c| c.get(row)).copied())),
                    (y_name.clone(), fmt(y_col.and_then(|c| c.get(row)).copied())),
                    (z_name.clone(), fmt(z_col.and_then(|c| c.get(row)).copied())),
                    ("Cluster".to_string(), cluster_str_for(row)),
                ];
                if feas.has_constraints() {
                    rows.push((
                        "Feasible".to_string(),
                        if feas.is_feasible(row) { "Yes" } else { "No" }.to_string(),
                    ));
                }
                rows
            },
            |row| {
                let mut context = vec![("Cluster".to_string(), cluster_str_for(row))];
                if feas.has_constraints() {
                    context.push((
                        "Feasible".to_string(),
                        if feas.is_feasible(row) { "Yes" } else { "No" }.to_string(),
                    ));
                }
                context
            },
        );

        // 詳細モーダルを描画する。
        if self.detail_modal.is_open() {
            self.detail_modal.show(
                ui,
                view,
                &ctx.meta.param_names,
                obj_names,
                &app_state.artifact_map,
            );
        }
    }

    /// クラスタリング設定 UI（k / モード / 空間 / Init / Run）を描画する。
    /// 2D の ClusterScatter::show_header と同じ操作感。
    fn show_cluster_controls(&mut self, ui: &mut egui::Ui, pareto_count: usize) {
        ui.horizontal(|ui| {
            let k_editable = !self.computing && self.k_mode == KSelectionMode::Manual;
            ui.label("k:");
            ui.add_enabled(
                k_editable,
                egui::DragValue::new(&mut self.k).range(2..=pareto_count.max(2)),
            );

            let elbow_max_k_editable =
                !self.computing && self.k_mode == KSelectionMode::ElbowDefault;
            ui.label("Max k:");
            ui.add_enabled(
                elbow_max_k_editable,
                egui::DragValue::new(&mut self.elbow_max_k).range(2..=50),
            );

            egui::ComboBox::from_id_salt("cluster_scatter_3d_k_mode")
                .selected_text(self.k_mode.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.k_mode,
                        KSelectionMode::ElbowDefault,
                        KSelectionMode::ElbowDefault.label(),
                    );
                    ui.selectable_value(
                        &mut self.k_mode,
                        KSelectionMode::Manual,
                        KSelectionMode::Manual.label(),
                    );
                });

            egui::ComboBox::from_id_salt("cluster_scatter_3d_space")
                .selected_text(self.target_space.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.target_space,
                        ClusterSpace::Objective,
                        ClusterSpace::Objective.label(),
                    );
                    ui.selectable_value(
                        &mut self.target_space,
                        ClusterSpace::Variable,
                        ClusterSpace::Variable.label(),
                    );
                    ui.selectable_value(
                        &mut self.target_space,
                        ClusterSpace::Combined,
                        ClusterSpace::Combined.label(),
                    );
                });

            ui.label("Init:");
            egui::ComboBox::from_id_salt("cluster_scatter_3d_init")
                .selected_text(self.init_strategy.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.init_strategy,
                        KMeansInitStrategy::KMeansPlusPlus,
                        KMeansInitStrategy::KMeansPlusPlus.label(),
                    );
                    ui.selectable_value(
                        &mut self.init_strategy,
                        KMeansInitStrategy::Deterministic,
                        KMeansInitStrategy::Deterministic.label(),
                    );
                });

            if ui
                .add_enabled(!self.computing, egui::Button::new("Run"))
                .clicked()
            {
                self.try_queue_compute(pareto_count);
            }

            if self.computing {
                ui.spinner();
                ui.label("Running clustering...");
            }
        });
    }

    fn try_queue_compute(&mut self, pareto_count: usize) {
        let request = ClusterComputeRequest {
            k: self.k,
            target_space: self.target_space,
            k_mode: self.k_mode,
            init_strategy: self.init_strategy,
            elbow_max_k: self.elbow_max_k,
        };

        match validate_cluster_request(&request, pareto_count) {
            Ok(()) => {
                self.pending_compute = Some(request);
                self.computing = true;
                self.last_error = None;
            }
            Err(err) => {
                self.pending_compute = None;
                self.last_error = Some(err);
            }
        }
    }

    pub fn set_error(&mut self, err: crate::state::messages::ClusterUiError) {
        self.computing = false;
        self.last_error = Some(err);
    }

    pub fn clear_runtime_state(&mut self) {
        self.computing = false;
        self.pending_compute = None;
        self.last_error = None;
    }

    /// 共有のクラスタリング実行状態（computing / pending / error）を取り込む。
    /// クラスタリング結果は `app_state.cluster_cache` に集約されるため、
    /// キャンバスの各アイテム（独立した WidgetStates）にも完了状態を反映する必要がある。
    pub fn adopt_runtime_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.pending_compute = src.pending_compute.clone();
        self.last_error = src.last_error.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_scatter_3d_default_axes() {
        let w = ClusterScatter3D::default();
        assert_eq!(w.x_objective, 0);
        assert_eq!(w.y_objective, 1);
        assert_eq!(w.z_objective, 2);
        assert!(w.show_infeasible);
        assert_ne!(w.camera.rotation, [0.0, 0.0, 0.0, 1.0]);
    }
}
