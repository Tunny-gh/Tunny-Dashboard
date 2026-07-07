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

/// PCA にかける特徴空間の選択。
///
/// アクティブ Study のどの数値列を PCA の入力に使うかを指定する。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PcaSpace {
    /// パラメータ列のみ。
    Param,
    /// 目的関数列のみ。
    Objective,
    /// パラメータ + 目的関数 + 数値 user attr のすべて。
    All,
}

/// PCA の計算結果。
///
/// 成分は説明分散の降順に整列される。入力が不正（n < 2 など）の場合は
/// すべて空の結果を返す。
#[derive(Debug, Clone)]
pub struct PcaResult {
    /// 各行を主成分空間へ射影した座標。`projections[row][component]`。
    pub projections: Vec<Vec<f64>>,
    /// 各主成分の負荷量（固有ベクトル）。`loadings[component][feature]`。
    pub loadings: Vec<Vec<f64>>,
    /// 各成分の説明分散（固有値、降順）。
    pub explained_variance: Vec<f64>,
    /// 各成分の寄与率（固有値 / 全固有値の和）。`explained_variance` と同長。
    pub explained_ratio: Vec<f64>,
    /// 入力に使った特徴（列）名。行列 API 経由では空。
    pub feature_names: Vec<String>,
}

/// k-means クラスタリングの結果。
#[derive(Debug, Clone)]
pub struct KmeansResult {
    /// 各行の所属クラスタ（0..k-1）。入力が不正な場合は空。
    pub labels: Vec<usize>,
    /// 各クラスタの重心。`centroids[cluster][feature]`。
    pub centroids: Vec<Vec<f64>>,
    /// クラスタ内平方和（Within-Cluster Sum of Squares）。
    pub wcss: f64,
    /// 設定した最大反復回数の上限値。linfa は実際の反復回数を公開しないため、
    /// この値は常に `max_n_iterations` に指定した設定値であり、実行時に実際に
    /// 何回反復したかを表すものではない。
    pub iterations: usize,
}

/// エルボー法によるクラスタ数推定の結果。
#[derive(Debug, Clone)]
pub struct ElbowResult {
    /// k = 2..=max_k の各クラスタ数に対する WCSS。
    pub wcss_per_k: Vec<f64>,
    /// WCSS 曲線の屈曲（二階差分最大）から推定した推奨クラスタ数。
    pub recommended_k: usize,
}
