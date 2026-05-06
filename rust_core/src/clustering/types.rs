/// k-means 初期重心選択戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitStrategy {
    /// k-means++: D² 比例確率でサンプリング（固定シードで再現可能）
    KMeansPlusPlus,
    /// 決定論的スプレッド: 累積距離しきい値で等間隔選択
    Deterministic,
}

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PcaSpace {
    /// Documentation.
    Param,
    /// Documentation.
    Objective,
    /// Documentation.
    All,
}

/// Documentation.
///
/// Documentation.
#[derive(Debug, Clone)]
pub struct PcaResult {
    /// Documentation.
    pub projections: Vec<Vec<f64>>,
    /// Documentation.
    pub loadings: Vec<Vec<f64>>,
    /// Documentation.
    pub explained_variance: Vec<f64>,
    /// Documentation.
    pub feature_names: Vec<String>,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct KmeansResult {
    /// Documentation.
    pub labels: Vec<usize>,
    /// Documentation.
    pub centroids: Vec<Vec<f64>>,
    /// Documentation.
    pub wcss: f64,
    /// Documentation.
    pub iterations: usize,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct ElbowResult {
    /// Documentation.
    pub wcss_per_k: Vec<f64>,
    /// Documentation.
    pub recommended_k: usize,
}

/// Documentation.
#[derive(Debug, Clone)]
pub struct ClusterStat {
    /// Documentation.
    pub cluster_id: usize,
    /// Documentation.
    pub size: usize,
    /// Documentation.
    pub centroid: Vec<f64>,
    /// Documentation.
    pub std_dev: Vec<f64>,
    /// Documentation.
    /// Documentation.
    pub significant_features: Vec<bool>,
}
