use std::collections::HashMap;

use crate::render::colormap::tab10_palette;
use linfa::traits::{Fit, Transformer};
use linfa::DatasetBase;
use linfa_reduction::Pca;
use ndarray::Array2;

/// クラスタ統計
pub struct ClusterStats {
    pub cluster_id: usize,
    pub count: usize,
    pub centroid: Vec<f64>,
}

/// クラスタリング対象空間
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn cache_offset(&self) -> usize {
        match self {
            ClusterSpace::Objective => 10_000,
            ClusterSpace::Variable => 20_000,
            ClusterSpace::Combined => 30_000,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct ClusterComputeRequest {
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
}

#[derive(Debug, Clone)]
pub struct ClusterMatrix {
    pub flat_data: Vec<f64>,
    pub n_rows: usize,
    pub n_cols: usize,
}

impl ClusterMatrix {
    pub fn is_valid_for_clustering(&self) -> bool {
        self.n_rows >= 2 && self.n_cols > 0
    }
}

/// クラスタリング結果
pub struct ClusteringResult {
    pub labels: Vec<usize>,
    pub pca_components: Vec<[f64; 2]>,
    pub cluster_stats: Vec<ClusterStats>,
}

/// クラスタ散布図ウィジェット
pub struct ClusterScatter {
    pub k: usize,
    pub target_space: ClusterSpace,
    pub k_mode: KSelectionMode,
    pub init_strategy: KMeansInitStrategy,
    pub computing: bool,
    pub pending_compute: Option<ClusterComputeRequest>,
    pub last_error: Option<crate::state::messages::ClusterUiError>,
    pub result: Option<ClusteringResult>,
    cached_pca: Option<Vec<[f32; 2]>>,
    cache_key: (usize, usize), // (trial_count, n_clusters)
}

impl Default for ClusterScatter {
    fn default() -> Self {
        Self {
            k: 3,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::ElbowDefault,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
            computing: false,
            pending_compute: None,
            last_error: None,
            result: None,
            cached_pca: None,
            cache_key: (0, 0),
        }
    }
}

impl ClusterScatter {
    pub fn new() -> Self {
        Self::default()
    }

    /// クラスタ散布図を描画する
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        trial_rows: &[crate::state::app_state::TrialRow],
        cluster_result: Option<&crate::state::app_state::ClusterResult>,
        param_names: &[String],
        obj_names: &[String],
        chart_colors: &[egui::Color32],
    ) {
        if trial_rows.len() < 2 {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("At least 2 trials are required.").weak());
            });
            return;
        }

        self.show_header(ui, trial_rows.len());

        if self.computing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Running clustering...");
            });
            return;
        }

        if let Some(err) = &self.last_error {
            ui.label(egui::RichText::new(&err.user_message).color(egui::Color32::RED));
            if let Some(detail) = &err.detail_for_dev {
                ui.label(egui::RichText::new(detail).small().weak());
            }
            if err.retryable && ui.button("Retry").clicked() {
                self.try_queue_compute(trial_rows.len());
            }
            ui.separator();
        }

        let Some(cr) = cluster_result else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Clustering has not been run yet.").weak());
            });
            return;
        };

        if cr.labels.len() != trial_rows.len() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Cluster result is inconsistent. Please run again.")
                        .color(egui::Color32::RED),
                );
            });
            return;
        }

        // キャッシュ確認・更新
        let new_key = (
            trial_rows.len(),
            cr.n_clusters + self.target_space.cache_offset(),
        );
        if self.cached_pca.is_none() || self.cache_key != new_key {
            self.cached_pca = Some(compute_pca_2d(
                trial_rows,
                param_names,
                obj_names,
                &self.target_space,
            ));
            self.cache_key = new_key;
        }
        let pca_points = self.cached_pca.as_ref().unwrap();

        // tab10 パレット（10色）
        let palette = tab10_palette();

        // クラスタ別に (point, original_index) を集約
        let mut cluster_points: HashMap<i32, Vec<([f64; 2], usize)>> = HashMap::new();
        for (i, &[x, y]) in pca_points.iter().enumerate() {
            let label = cr.labels.get(i).copied().unwrap_or(0);
            cluster_points
                .entry(label)
                .or_default()
                .push(([x as f64, y as f64], i));
        }

        egui_plot::Plot::new("cluster_scatter").show(ui, |plot_ui| {
            for (label, pts_with_idx) in &cluster_points {
                let representative_color = if !chart_colors.is_empty() {
                    pts_with_idx
                        .first()
                        .and_then(|&(_, idx)| chart_colors.get(idx).copied())
                        .unwrap_or(palette[*label as usize % palette.len()])
                } else {
                    palette[*label as usize % palette.len()]
                };
                let pts: Vec<[f64; 2]> = pts_with_idx.iter().map(|&(pt, _)| pt).collect();
                let points = egui_plot::Points::new(pts)
                    .color(representative_color)
                    .radius(3.0)
                    .name(format!("Cluster {}", label));
                plot_ui.points(points);
            }
        });
    }

    fn show_header(&mut self, ui: &mut egui::Ui, trial_count: usize) {
        ui.horizontal(|ui| {
            ui.label("k:");
            ui.add_enabled(
                !self.computing,
                egui::DragValue::new(&mut self.k).range(2..=trial_count.max(2)),
            );

            egui::ComboBox::from_id_salt("cluster_scatter_k_mode")
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

            egui::ComboBox::from_id_salt("cluster_scatter_space")
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

            egui::ComboBox::from_id_salt("cluster_scatter_init")
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
                self.try_queue_compute(trial_count);
            }
        });
    }

    fn try_queue_compute(&mut self, trial_count: usize) {
        let request = ClusterComputeRequest {
            k: self.k,
            target_space: self.target_space,
            k_mode: self.k_mode,
            init_strategy: self.init_strategy,
        };

        match validate_cluster_request(&request, trial_count) {
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
}

fn build_cluster_matrix_data(
    trial_rows: &[crate::state::app_state::TrialRow],
    param_names: &[String],
    obj_names: &[String],
    target_space: ClusterSpace,
) -> ClusterMatrix {
    let n_rows = trial_rows.len();
    let n_cols = target_space.feature_count(param_names.len(), obj_names.len());

    let flat_data = match target_space {
        ClusterSpace::Objective => trial_rows
            .iter()
            .flat_map(|r| (0..obj_names.len()).map(|i| r.objectives.get(i).copied().unwrap_or(0.0)))
            .collect(),
        ClusterSpace::Variable => trial_rows
            .iter()
            .flat_map(|r| {
                param_names
                    .iter()
                    .map(|name| r.params.get(name).copied().unwrap_or(0.0))
            })
            .collect(),
        ClusterSpace::Combined => trial_rows
            .iter()
            .flat_map(|r| {
                param_names
                    .iter()
                    .map(|name| r.params.get(name).copied().unwrap_or(0.0))
                    .chain(
                        (0..obj_names.len()).map(|i| r.objectives.get(i).copied().unwrap_or(0.0)),
                    )
            })
            .collect(),
    };

    ClusterMatrix {
        flat_data,
        n_rows,
        n_cols,
    }
}

pub fn build_cluster_matrix(
    trial_rows: &[crate::state::app_state::TrialRow],
    param_names: &[String],
    obj_names: &[String],
    target_space: ClusterSpace,
) -> Result<ClusterMatrix, crate::state::messages::ClusterUiError> {
    let matrix = build_cluster_matrix_data(trial_rows, param_names, obj_names, target_space);
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

/// パラメータ値を PCA 2D 投影する。
/// パラメータが 1 次元以下の場合は第 1 軸のみ使用してフォールバックする。
fn compute_pca_2d(
    trial_rows: &[crate::state::app_state::TrialRow],
    param_names: &[String],
    obj_names: &[String],
    target_space: &ClusterSpace,
) -> Vec<[f32; 2]> {
    let matrix = build_cluster_matrix_data(trial_rows, param_names, obj_names, *target_space);
    let n = matrix.n_rows;
    let p = matrix.n_cols;

    if p == 0 || n == 0 {
        return vec![[0.0, 0.0]; n];
    }

    let Ok(data) = Array2::from_shape_vec((n, p), matrix.flat_data) else {
        return vec![[0.0, 0.0]; n];
    };

    // 1次元のときはそのまま返す
    if p == 1 {
        return data.column(0).iter().map(|&v| [v as f32, 0.0]).collect();
    }

    // linfa PCA で 2D 投影を試みる
    let n_components = 2.min(p);
    let dataset = DatasetBase::from(data.clone());
    match Pca::params(n_components).fit(&dataset) {
        Ok(pca) => {
            let projected = pca.transform(dataset);
            projected
                .records()
                .rows()
                .into_iter()
                .map(|row| {
                    [
                        row[0] as f32,
                        if row.len() > 1 { row[1] as f32 } else { 0.0 },
                    ]
                })
                .collect()
        }
        Err(_) => {
            // フォールバック: 最初の 2 次元をそのまま使用
            data.rows()
                .into_iter()
                .map(|row| [row[0] as f32, row[1] as f32])
                .collect()
        }
    }
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

/// クラスタラベルが 0..k-1 の範囲に収まるか確認する
pub fn cluster_labels_valid(labels: &[usize], k: usize) -> bool {
    labels.iter().all(|&l| l < k)
}

/// 全クラスタの件数合計がデータ件数と一致するか確認する
pub fn cluster_stats_count_sum(stats: &[ClusterStats]) -> usize {
    stats.iter().map(|s| s.count).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_labels_valid_all_in_range() {
        let labels = vec![0, 1, 2, 0, 1, 2];
        assert!(cluster_labels_valid(&labels, 3));
    }

    #[test]
    fn cluster_labels_invalid_out_of_range() {
        let labels = vec![0, 1, 3]; // 3 >= k=3
        assert!(!cluster_labels_valid(&labels, 3));
    }

    #[test]
    fn cluster_stats_count_sum_matches_total() {
        let stats = vec![
            ClusterStats {
                cluster_id: 0,
                count: 5,
                centroid: vec![],
            },
            ClusterStats {
                cluster_id: 1,
                count: 3,
                centroid: vec![],
            },
            ClusterStats {
                cluster_id: 2,
                count: 7,
                centroid: vec![],
            },
        ];
        assert_eq!(cluster_stats_count_sum(&stats), 15);
    }

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
        assert!(!cs.computing);
        assert!(cs.pending_compute.is_none());
        assert!(cs.last_error.is_none());
        assert!(cs.result.is_none());
        assert!(cs.cached_pca.is_none());
        assert_eq!(cs.cache_key, (0, 0));
    }

    #[test]
    fn compute_pca_2d_empty_trials() {
        let result = compute_pca_2d(
            &[],
            &["x".to_string()],
            &["y".to_string()],
            &ClusterSpace::Variable,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn compute_pca_2d_no_params_returns_n_zeros() {
        use crate::state::app_state::{TrialRow, TrialState};
        let trial = TrialRow {
            trial_id: 0,
            trial_number: 0,
            params: std::collections::HashMap::new(),
            objectives: vec![1.0],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: std::collections::HashMap::new(),
        };
        let result = compute_pca_2d(&[trial], &[], &[], &ClusterSpace::Variable);
        // p==0 → returns vec![[0.0, 0.0]; n] where n==1
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], [0.0_f32, 0.0_f32]);
    }

    #[test]
    fn cache_key_updated_on_data_change() {
        let cs = ClusterScatter::default();
        assert_eq!(cs.cache_key, (0, 0));
        // キャッシュキーが正しく初期化されていること
        assert!(cs.cached_pca.is_none());
    }

    #[test]
    fn validate_cluster_request_rejects_manual_k_too_small() {
        let request = ClusterComputeRequest {
            k: 1,
            target_space: ClusterSpace::Objective,
            k_mode: KSelectionMode::Manual,
            init_strategy: KMeansInitStrategy::KMeansPlusPlus,
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
        };
        assert!(validate_cluster_request(&request, 10).is_ok());
    }
}
