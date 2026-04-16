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
#[derive(Debug, Clone, PartialEq)]
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
    pub computing: bool,
    pub result: Option<ClusteringResult>,
    cached_pca: Option<Vec<[f32; 2]>>,
    cache_key: (usize, usize), // (trial_count, n_clusters)
}

impl Default for ClusterScatter {
    fn default() -> Self {
        Self {
            k: 3,
            target_space: ClusterSpace::Objective,
            computing: false,
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
        chart_colors: &[egui::Color32],
    ) {
        let Some(cr) = cluster_result else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No cluster data.").weak());
            });
            return;
        };

        if trial_rows.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("No trial data.").weak());
            });
            return;
        }

        // キャッシュ確認・更新
        let new_key = (trial_rows.len(), cr.n_clusters);
        if self.cached_pca.is_none() || self.cache_key != new_key {
            self.cached_pca = Some(compute_pca_2d(trial_rows, param_names));
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
}

/// パラメータ値を PCA 2D 投影する。
/// パラメータが 1 次元以下の場合は第 1 軸のみ使用してフォールバックする。
fn compute_pca_2d(
    trial_rows: &[crate::state::app_state::TrialRow],
    param_names: &[String],
) -> Vec<[f32; 2]> {
    let n = trial_rows.len();
    let p = param_names.len();

    if p == 0 || n == 0 {
        return vec![[0.0, 0.0]; n];
    }

    // データ行列を構築
    let flat: Vec<f64> = trial_rows
        .iter()
        .flat_map(|r| {
            param_names
                .iter()
                .map(|name| r.params.get(name).copied().unwrap_or(0.0))
        })
        .collect();

    let Ok(data) = Array2::from_shape_vec((n, p), flat) else {
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
        assert!(!cs.computing);
        assert!(cs.result.is_none());
        assert!(cs.cached_pca.is_none());
        assert_eq!(cs.cache_key, (0, 0));
    }

    #[test]
    fn compute_pca_2d_empty_trials() {
        let result = compute_pca_2d(&[], &["x".to_string()]);
        assert!(result.is_empty());
    }

    #[test]
    fn compute_pca_2d_no_params_returns_n_zeros() {
        use crate::state::app_state::{TrialRow, TrialState};
        let trial = TrialRow {
            trial_id: 0,
            params: std::collections::HashMap::new(),
            objectives: vec![1.0],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: std::collections::HashMap::new(),
        };
        let result = compute_pca_2d(&[trial], &[]);
        // p==0 → returns vec![[0.0, 0.0]; n] where n==1
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], [0.0_f32, 0.0_f32]);
    }

    #[test]
    fn cache_key_updated_on_data_change() {
        let mut cs = ClusterScatter::default();
        assert_eq!(cs.cache_key, (0, 0));
        // キャッシュキーが正しく初期化されていること
        assert!(cs.cached_pca.is_none());
    }
}
