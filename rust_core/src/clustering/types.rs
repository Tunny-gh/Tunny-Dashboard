/// k-means 初期重心選択戦略
///
/// どちらも linfa のデフォルト初期化（k-means++、D² 比例確率でサンプリング）を使用する。
/// 違いは乱数シードの決め方のみ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitStrategy {
    /// k-means++ をデータ形状（n, k）由来のシードで実行する。
    KMeansPlusPlus,
    /// k-means++ を固定シード（42）で実行する。シードが固定されているため、
    /// 同じ入力に対しては常に同じ初期重心・結果が得られる（再現性重視）。
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
    /// 設定した最大反復回数の上限値。linfa は実際の反復回数を公開しないため、
    /// この値は常に `max_n_iterations` に指定した設定値であり、実行時に実際に
    /// 何回反復したかを表すものではない。
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
