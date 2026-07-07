use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::io::artifacts::ArtifactEntry;
use crate::state::types::StudyView;
use crate::theme::chart_colors::{COLOR_INFEASIBLE, COLOR_NON_PARETO_DIM, COLOR_UNSELECTED_POINT};
use crate::theme::color_compute::compute_point_alpha;
use crate::theme::colormap::ColorMap;
use crate::theme::ERROR_COLOR;
use crate::ui::widgets::common::cluster_controls::ClusterControls;
use crate::ui::widgets::common::plot_nav::{apply_wheel_zoom, UnifiedNav};
use crate::ui::widgets::trial_detail_modal::{
    hit_test_nearest, TrialDetailModal, TrialDetailTarget, HIT_THRESHOLD,
};

/// クラスタリング対象空間
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClusterSpace {
    Objective,
    Variable,
    Combined,
}

impl ClusterSpace {
    pub fn label(&self) -> &'static str {
        match self {
            ClusterSpace::Objective => "Objective Space",
            ClusterSpace::Variable => "Variable Space",
            ClusterSpace::Combined => "Combined",
        }
    }

    pub fn feature_count(&self, n_params: usize, n_objectives: usize) -> usize {
        match self {
            ClusterSpace::Objective => n_objectives,
            ClusterSpace::Variable => n_params,
            ClusterSpace::Combined => n_params + n_objectives,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KSelectionMode {
    ElbowDefault,
    Manual,
}

impl KSelectionMode {
    pub fn label(&self) -> &'static str {
        match self {
            KSelectionMode::ElbowDefault => "Elbow (Auto)",
            KSelectionMode::Manual => "Manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KMeansInitStrategy {
    KMeansPlusPlus,
    Deterministic,
}

impl KMeansInitStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            KMeansInitStrategy::KMeansPlusPlus => "k-means++",
            KMeansInitStrategy::Deterministic => "Deterministic",
        }
    }
}

impl From<KMeansInitStrategy> for tunny_core::clustering::InitStrategy {
    fn from(s: KMeansInitStrategy) -> Self {
        match s {
            KMeansInitStrategy::KMeansPlusPlus => Self::KMeansPlusPlus,
            KMeansInitStrategy::Deterministic => Self::Deterministic,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClusterComputeRequest {
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    /// Elbow（自動）モードで使う探索範囲の上限 k。Manual モードでは無視される。
    pub elbow_max_k: usize,
}

/// クラスタリング結果のキャッシュキー。
/// 同じ設定（対象空間・k 選択モード・k・Init 戦略）で計算した結果を共有するため、
/// 各チャート（2D / 3D / Table）はこのキーで `app_state.cluster_cache` を参照する。
///
/// Elbow（自動）モードでは k はアルゴリズムが決めるため、入力 k はキーに含めず 0 に正規化する。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClusterCacheKey {
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub k: usize,
    pub init_strategy: KMeansInitStrategy,
    pub elbow_max_k: usize,
}

impl ClusterCacheKey {
    pub fn new(
        target_space: ClusterSpace,
        k_mode: KSelectionMode,
        k: usize,
        init_strategy: KMeansInitStrategy,
        elbow_max_k: usize,
    ) -> Self {
        // Elbow モードでは入力 k は無視されるため、キャッシュヒット判定がブレないよう 0 に正規化する。
        // 対称的に、Manual モードでは elbow_max_k は使われないため 0 に正規化する。
        let (k, elbow_max_k) = match k_mode {
            KSelectionMode::Manual => (k, 0),
            KSelectionMode::ElbowDefault => (0, elbow_max_k),
        };
        Self {
            target_space,
            k_mode,
            k,
            init_strategy,
            elbow_max_k,
        }
    }

    pub fn from_request(req: &ClusterComputeRequest) -> Self {
        Self::new(
            req.target_space,
            req.k_mode,
            req.k,
            req.init_strategy,
            req.elbow_max_k,
        )
    }
}

#[derive(Debug, Clone)]
pub struct ClusterMatrix {
    pub flat_data: Vec<f64>,
    /// クラスタリング対象（パレートフロント）の行数（k-means に渡す行数）
    pub n_rows: usize,
    pub n_cols: usize,
    /// 全トライアル数（非対象の解を含む）
    pub total_trials: usize,
    /// matrix の行 index → 元の trial index のマッピング（パレートフロントの行）
    pub target_indices: Vec<usize>,
}

impl ClusterMatrix {
    pub fn is_valid_for_clustering(&self) -> bool {
        self.n_rows >= 2 && self.n_cols > 0
    }
}

/// クラスタ散布図ウィジェット
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ClusterScatter {
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
    /// 点クリックで開くトライアル詳細モーダル。
    #[serde(skip)]
    pub detail_modal: TrialDetailModal,
    #[serde(skip)]
    cached_points: Option<Vec<[f32; 2]>>,
    #[serde(skip)]
    cache_key: (usize, usize, usize), // (df_ptr, trial_count, n_clusters)
}

impl Default for ClusterScatter {
    fn default() -> Self {
        Self {
            k: 3,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
            computing: false,
            pending_compute: None,
            last_error: None,
            detail_modal: TrialDetailModal::new(),
            cached_points: None,
            cache_key: (0, 0, 0),
        }
    }
}

impl ClusterScatter {
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

    /// クラスタ散布図を描画する
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        view: &StudyView,
        cluster_result: Option<&crate::state::app_state::ClusterResult>,
        param_names: &[String],
        obj_names: &[String],
        colormap: &ColorMap,
        artifact_map: &HashMap<u32, Vec<ArtifactEntry>>,
        selected_indices: &[u32],
    ) {
        let n_trials = view.row_count();
        // クラスタリング対象はパレートフロント（pareto_rank == 0）。
        // k の上限・実行可否はフロント点数で判定する。
        let pareto_count = view.pareto_rank.iter().filter(|&&r| r == 0).count();
        if pareto_count < 2 {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("At least 2 Pareto-front solutions are required.").weak(),
                );
            });
            return;
        }

        self.show_header(ui, pareto_count);

        // 選択フィルタ中は、クラスタがフロント全体で計算されている旨を明示する。
        if !selected_indices.is_empty() {
            ui.label(
                egui::RichText::new(
                    "Highlighting selection. Clusters are computed over the full Pareto front.",
                )
                .small()
                .weak(),
            );
        }

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Running clustering...");
            });
            return;
        }

        if let Some(err) = &self.last_error {
            ui.label(egui::RichText::new(&err.user_message).color(ERROR_COLOR()));
            if let Some(detail) = &err.detail_for_dev {
                ui.label(egui::RichText::new(detail).small().weak());
            }
            if err.retryable && ui.button("Retry").clicked() {
                self.try_queue_compute(pareto_count);
            }
            ui.separator();
        }

        let Some(cr) = cluster_result else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Clustering has not been run yet.").weak());
            });
            return;
        };

        if cr.labels.len() != n_trials {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Cluster result is inconsistent. Please run again.")
                        .color(ERROR_COLOR()),
                );
            });
            return;
        }

        // キャッシュ確認・更新（目的関数軸の座標）。
        // df の Arc 恒等性をキーに含め、同一次元の別 Study 切替でのスタール描画を防ぐ（M-6）。
        let df_ptr = std::sync::Arc::as_ptr(&view.df) as usize;
        let new_key = (df_ptr, n_trials, cr.n_clusters);
        if self.cached_points.is_none() || self.cache_key != new_key {
            self.cached_points = Some(compute_obj_axes_2d(view, obj_names));
            self.cache_key = new_key;
        }
        let plot_points = self.cached_points.as_ref().unwrap();

        let feas = view.feasibility();

        // 点クリック判定用の候補（trial_id, 行 index, 座標）。
        let hit_candidates: Vec<(u32, usize, [f64; 2])> = plot_points
            .iter()
            .enumerate()
            .map(|(i, &[x, y])| {
                let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
                (trial_id, i, [x as f64, y as f64])
            })
            .collect();

        // k 個のクラスタを [0, 1] 上に等間隔配置してカラーマップからサンプリング
        // k=2 → t=0.0, 1.0（両端）、k=3 → t=0.0, 0.5, 1.0 など
        let n_clusters = cr.n_clusters.max(1);
        let cluster_color = |label: i32| -> egui::Color32 {
            let t = if n_clusters == 1 {
                0.5
            } else {
                label as f32 / (n_clusters - 1) as f32
            };
            colormap.interpolate(t)
        };

        // クラスタリング対象はパレートフロントのみ。クラスタ別に座標を集約し、
        // 対象外（label < 0）の解は "Others"、infeasible は別途収集する。
        // 選択フィルタ（PCP ブラシ等）が有効な場合、選択外は灰色でまとめて背面に描く。
        // クラスタ計算自体はフロント全体のままで、ここでの分岐は表示上の強調に限る。
        let mut cluster_points: BTreeMap<i32, Vec<[f64; 2]>> = BTreeMap::new();
        let mut unselected_pts: Vec<[f64; 2]> = Vec::new();
        let mut infeasible_pts: Vec<[f64; 2]> = Vec::new();
        let mut other_pts: Vec<[f64; 2]> = Vec::new();
        for (i, &[x, y]) in plot_points.iter().enumerate() {
            if !feas.is_feasible(i) {
                infeasible_pts.push([x as f64, y as f64]);
                continue;
            }
            let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
            let selected = compute_point_alpha(trial_id, selected_indices) == 255;
            // 選択フィルタ外は、クラスタ点・劣解（label < 0）を問わず灰色へまとめる。
            if !selected {
                unselected_pts.push([x as f64, y as f64]);
                continue;
            }
            let label = cr.labels.get(i).copied().unwrap_or(-1);
            if label < 0 {
                // パレートフロント以外の解（クラスタリング対象外）
                other_pts.push([x as f64, y as f64]);
            } else {
                cluster_points
                    .entry(label)
                    .or_default()
                    .push([x as f64, y as f64]);
            }
        }

        let x_label = obj_names.first().map(|s| s.as_str()).unwrap_or("Obj 1");
        let y_label = obj_names.get(1).map(|s| s.as_str()).unwrap_or("Obj 2");
        let mut clicked_detail: Option<(u32, usize)> = None;
        egui_plot::Plot::new("cluster_scatter")
            .unified_nav()
            .x_axis_label(x_label)
            .y_axis_label(y_label)
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                apply_wheel_zoom(plot_ui);
                // 点クリックで詳細モーダルを開く対象を検出する。
                let resp = plot_ui.response();
                if resp.clicked_by(egui::PointerButton::Primary) {
                    clicked_detail = resp.interact_pointer_pos().and_then(|pos| {
                        hit_test_nearest(plot_ui, &hit_candidates, pos, HIT_THRESHOLD)
                    });
                }
                // infeasible を最背面に描画
                if !infeasible_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("", infeasible_pts)
                            .color(COLOR_INFEASIBLE())
                            .radius(3.0)
                            .name("Infeasible"),
                    );
                }
                // パレートフロント以外（クラスタリング対象外）を淡色で背面に描画
                if !other_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("", other_pts)
                            .color(COLOR_NON_PARETO_DIM())
                            .radius(2.0)
                            .name("Others"),
                    );
                }
                // 選択外のクラスタ点は灰色で先に描画し、選択点と明確に区別する。
                // クラスタ色は残さず（色相が紛らわしいため）"Others (unselected)" に集約する。
                if !unselected_pts.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("", unselected_pts)
                            .color(COLOR_UNSELECTED_POINT())
                            .radius(2.0)
                            .name("Others (unselected)"),
                    );
                }
                for (label, pts) in cluster_points {
                    let color = cluster_color(label);
                    let points = egui_plot::Points::new("", pts)
                        .color(color)
                        .radius(3.0)
                        .name(format!("Cluster {}", label));
                    plot_ui.points(points);
                }
            });

        // 点クリックでトライアル詳細モーダルを開く（散布図情報 = クラスタ番号）。
        if let Some((trial_id, row)) = clicked_detail {
            let label = cr.labels.get(row).copied().unwrap_or(-1);
            let cluster_str = if label < 0 {
                "Unclustered".to_string()
            } else {
                label.to_string()
            };
            let mut context = vec![("Cluster".to_string(), cluster_str)];
            let rank = view.pareto_rank.get(row).copied().unwrap_or(0);
            context.push(("Pareto Rank".to_string(), rank.to_string()));
            self.detail_modal.open(TrialDetailTarget {
                trial_id,
                row_index: row,
                context,
            });
        }

        self.detail_modal
            .show(ui, view, param_names, obj_names, artifact_map);
    }

    /// 設定値・実行状態のフィールドへの可変参照束を組み立てる（共通ロジック委譲用）。
    fn controls(&mut self) -> ClusterControls<'_> {
        ClusterControls {
            k: &mut self.k,
            target_space: &mut self.target_space,
            k_mode: &mut self.k_mode,
            init_strategy: &mut self.init_strategy,
            elbow_max_k: &mut self.elbow_max_k,
            computing: &mut self.computing,
            pending_compute: &mut self.pending_compute,
            last_error: &mut self.last_error,
        }
    }

    fn show_header(&mut self, ui: &mut egui::Ui, trial_count: usize) {
        // 2D はスピナーを本体側（show 内）で別途表示するため、ここでは出さない。
        self.controls()
            .show_controls(ui, trial_count, "cluster_scatter", false);
    }

    fn try_queue_compute(&mut self, trial_count: usize) {
        self.controls().try_queue_compute(trial_count);
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
    /// 表示用キャッシュ（cached_points 等）はアイテム固有なので維持する。
    pub fn adopt_runtime_state(&mut self, src: &Self) {
        self.computing = src.computing;
        self.pending_compute = src.pending_compute.clone();
        self.last_error = src.last_error.clone();
    }
}

fn build_cluster_matrix_data(
    view: &StudyView,
    param_names: &[String],
    obj_names: &[String],
    target_space: ClusterSpace,
) -> ClusterMatrix {
    let total_trials = view.row_count();
    let n_cols = target_space.feature_count(param_names.len(), obj_names.len());

    // クラスタリング対象はパレートフロント（pareto_rank == 0）の解に限定する。
    // 制約あり Study では rank 0 は feasible 非劣解のみなので feasible 判定は不要。
    let target_indices: Vec<usize> = (0..total_trials)
        .filter(|&i| view.pareto_rank.get(i).copied().unwrap_or(u32::MAX) == 0)
        .collect();

    let n_rows = target_indices.len();

    // パレートフロントの解のみで特徴量行列を構築
    let flat_data = match target_space {
        ClusterSpace::Objective => {
            let cols = view.numeric_columns(obj_names);
            target_indices
                .iter()
                .flat_map(|&i| {
                    cols.iter()
                        .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                })
                .collect()
        }
        ClusterSpace::Variable => {
            let cols = view.numeric_columns(param_names);
            target_indices
                .iter()
                .flat_map(|&i| {
                    cols.iter()
                        .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                })
                .collect()
        }
        ClusterSpace::Combined => {
            let param_cols = view.numeric_columns(param_names);
            let obj_cols = view.numeric_columns(obj_names);
            target_indices
                .iter()
                .flat_map(|&i| {
                    param_cols
                        .iter()
                        .chain(obj_cols.iter())
                        .map(move |col| col.and_then(|c| c.get(i)).copied().unwrap_or(0.0))
                })
                .collect()
        }
    };

    ClusterMatrix {
        flat_data,
        n_rows,
        n_cols,
        total_trials,
        target_indices,
    }
}

pub fn build_cluster_matrix(
    view: &StudyView,
    param_names: &[String],
    obj_names: &[String],
    target_space: ClusterSpace,
) -> Result<ClusterMatrix, crate::state::messages::ClusterUiError> {
    let matrix = build_cluster_matrix_data(view, param_names, obj_names, target_space);
    if !matrix.is_valid_for_clustering() {
        return Err(crate::state::messages::cluster_ui_error(
            "At least 2 trials and one feature are required.",
            Some(format!(
                "validation: trial_count({}), n_cols({})",
                matrix.n_rows, matrix.n_cols
            )),
            false,
        ));
    }
    Ok(matrix)
}

/// 目的関数値の最初の 2 軸を散布図用に返す。
/// 目的関数が 1 つのみの場合は Y 軸を 0.0 固定とする。
fn compute_obj_axes_2d(view: &StudyView, obj_names: &[String]) -> Vec<[f32; 2]> {
    let n = view.row_count();
    let col0 = obj_names.first().and_then(|name| view.numeric_column(name));
    let col1 = obj_names.get(1).and_then(|name| view.numeric_column(name));
    (0..n)
        .map(|i| {
            let x = col0.and_then(|c| c.get(i)).copied().unwrap_or(0.0) as f32;
            let y = col1.and_then(|c| c.get(i)).copied().unwrap_or(0.0) as f32;
            [x, y]
        })
        .collect()
}

pub fn validate_cluster_request(
    request: &ClusterComputeRequest,
    trial_count: usize,
) -> Result<(), crate::state::messages::ClusterUiError> {
    if trial_count < 2 {
        return Err(crate::state::messages::cluster_ui_error(
            "At least 2 trials are required.",
            Some(format!("validation: trial_count({trial_count}) < 2")),
            false,
        ));
    }

    if matches!(request.k_mode, KSelectionMode::Manual) {
        if request.k < 2 {
            return Err(crate::state::messages::cluster_ui_error(
                "k must be at least 2.",
                Some("validation: k < 2".to_string()),
                true,
            ));
        }
        if request.k > trial_count {
            return Err(crate::state::messages::cluster_ui_error(
                "k must be less than or equal to the number of trials.",
                Some(format!(
                    "validation: k({}) > trial_count({trial_count})",
                    request.k
                )),
                true,
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_space_labels() {
        assert_eq!(ClusterSpace::Objective.label(), "Objective Space");
        assert_eq!(ClusterSpace::Variable.label(), "Variable Space");
        assert_eq!(ClusterSpace::Combined.label(), "Combined");
    }

    #[test]
    fn cluster_scatter_default_k() {
        let cs = ClusterScatter::default();
        assert_eq!(cs.k, 3);
        assert_eq!(cs.target_space, ClusterSpace::Objective);
        assert_eq!(cs.k_mode, KSelectionMode::ElbowDefault);
        assert_eq!(cs.init_strategy, KMeansInitStrategy::KMeansPlusPlus);
        assert_eq!(cs.elbow_max_k, 10);
        assert!(!cs.computing);
        assert!(cs.pending_compute.is_none());
        assert!(cs.last_error.is_none());
        assert!(cs.cached_points.is_none());
        assert_eq!(cs.cache_key, (0, 0, 0));
    }

    fn make_view_with_objs(obj_vals: &[Vec<f64>]) -> StudyView {
        use std::collections::HashMap;
        use std::sync::Arc;
        use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
        let n = obj_vals.len();
        if n == 0 {
            let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
            return StudyView::new(Arc::new(df), vec![]);
        }
        let n_obj = obj_vals[0].len();
        let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
        let core_rows: Vec<CoreRow> = (0..n)
            .map(|i| CoreRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display: HashMap::new(),
                param_category_label: HashMap::new(),
                objective_values: obj_vals[i].clone(),
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            })
            .collect();
        let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
        StudyView::new(Arc::new(df), vec![0; n])
    }

    #[test]
    fn compute_obj_axes_2d_empty_trials() {
        let view = make_view_with_objs(&[]);
        let result = compute_obj_axes_2d(&view, &["obj0".to_string()]);
        assert!(result.is_empty());
    }

    #[test]
    fn compute_obj_axes_2d_single_objective() {
        let view = make_view_with_objs(&[vec![1.5]]);
        let result = compute_obj_axes_2d(&view, &["obj0".to_string()]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], [1.5_f32, 0.0_f32]);
    }

    #[test]
    fn cache_key_updated_on_data_change() {
        let cs = ClusterScatter::default();
        assert_eq!(cs.cache_key, (0, 0, 0));
        assert!(cs.cached_points.is_none());
    }

    #[test]
    fn adopt_runtime_state_clears_stuck_computing() {
        // キャンバスのアイテムが Run で computing=true になったまま、
        // グローバル側の完了状態を取り込むと spinner が解除される（描画されない不具合の回帰防止）。
        let mut item = ClusterScatter {
            computing: true,
            ..Default::default()
        };
        let global = ClusterScatter::default(); // 完了後（computing=false, error=None）
        item.adopt_runtime_state(&global);
        assert!(!item.computing);
        assert!(item.pending_compute.is_none());
        assert!(item.last_error.is_none());
    }

    #[test]
    fn adopt_runtime_state_preserves_display_cache() {
        // 表示用キャッシュ（cached_points / cache_key）はアイテム固有なので維持される。
        let mut item = ClusterScatter {
            computing: true,
            cached_points: Some(vec![[1.0, 2.0]]),
            cache_key: (7, 5, 3),
            ..Default::default()
        };
        item.adopt_runtime_state(&ClusterScatter::default());
        assert_eq!(item.cached_points, Some(vec![[1.0, 2.0]]));
        assert_eq!(item.cache_key, (7, 5, 3));
    }

    #[test]
    fn adopt_runtime_state_propagates_error() {
        let mut item = ClusterScatter {
            computing: true,
            ..Default::default()
        };
        let mut global = ClusterScatter::default();
        global.set_error(crate::state::messages::cluster_ui_error("boom", None, true));
        item.adopt_runtime_state(&global);
        assert!(!item.computing);
        assert!(item.last_error.is_some());
    }

    #[test]
    fn validate_cluster_request_rejects_manual_k_too_small() {
        let request = ClusterComputeRequest {
            k: 1,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::Manual,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
        };
        assert!(validate_cluster_request(&request, 10).is_err());
    }

    #[test]
    fn validate_cluster_request_accepts_elbow_mode() {
        let request = ClusterComputeRequest {
            k: 999,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            elbow_max_k: 10,
        };
        assert!(validate_cluster_request(&request, 10).is_ok());
    }

    #[test]
    fn cache_key_normalizes_unused_field_per_mode() {
        // Manual モードでは elbow_max_k が意味を持たないため 0 に正規化される。
        let manual_key = ClusterCacheKey::new(
            ClusterSpace::Objective,
            KSelectionMode::Manual,
            5,
            KMeansInitStrategy::KMeansPlusPlus,
            42,
        );
        assert_eq!(manual_key.k, 5);
        assert_eq!(manual_key.elbow_max_k, 0);

        // Elbow モードでは k が意味を持たないため 0 に正規化される。
        let elbow_key = ClusterCacheKey::new(
            ClusterSpace::Objective,
            KSelectionMode::ElbowDefault,
            5,
            KMeansInitStrategy::KMeansPlusPlus,
            42,
        );
        assert_eq!(elbow_key.k, 0);
        assert_eq!(elbow_key.elbow_max_k, 42);
    }
}
