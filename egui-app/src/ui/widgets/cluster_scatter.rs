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
}

impl Default for ClusterScatter {
    fn default() -> Self {
        Self {
            k: 3,
            target_space: ClusterSpace::Objective,
            computing: false,
            result: None,
        }
    }
}

impl ClusterScatter {
    pub fn new() -> Self {
        Self::default()
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
            ClusterStats { cluster_id: 0, count: 5, centroid: vec![] },
            ClusterStats { cluster_id: 1, count: 3, centroid: vec![] },
            ClusterStats { cluster_id: 2, count: 7, centroid: vec![] },
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
    }
}
