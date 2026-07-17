/// Strategy for selecting the initial k-means centroids.
///
/// Both variants use linfa's default initialization (k-means++, sampling
/// with probability proportional to D²). The only difference is how the
/// random seed is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitStrategy {
    /// Runs k-means++ with a seed derived from the data shape (n, k).
    KMeansPlusPlus,
    /// Runs k-means++ with a fixed seed (42). Because the seed is fixed,
    /// the same input always yields the same initial centroids and result
    /// (prioritizes reproducibility).
    Deterministic,
}

/// Selection of the feature space fed into PCA.
///
/// Specifies which numeric columns of the active study are used as PCA input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PcaSpace {
    /// Parameter columns only.
    Param,
    /// Objective columns only.
    Objective,
    /// All of parameters + objectives + numeric user attrs.
    All,
}

/// Result of a PCA computation.
///
/// Components are sorted by explained variance in descending order. If the
/// input is invalid (e.g. n < 2), all fields are returned empty.
#[derive(Debug, Clone)]
pub struct PcaResult {
    /// Coordinates of each row projected into principal-component space. `projections[row][component]`.
    pub projections: Vec<Vec<f64>>,
    /// Loadings (eigenvectors) of each principal component. `loadings[component][feature]`.
    pub loadings: Vec<Vec<f64>>,
    /// Explained variance of each component (eigenvalues, descending).
    pub explained_variance: Vec<f64>,
    /// Explained variance ratio of each component (eigenvalue / sum of all eigenvalues). Same length as `explained_variance`.
    pub explained_ratio: Vec<f64>,
    /// Names of the features (columns) used as input. Empty when using the matrix API.
    pub feature_names: Vec<String>,
}

/// Result of k-means clustering.
#[derive(Debug, Clone)]
pub struct KmeansResult {
    /// Cluster assignment for each row (0..k-1). Empty if the input is invalid.
    pub labels: Vec<usize>,
    /// Centroid of each cluster. `centroids[cluster][feature]`.
    pub centroids: Vec<Vec<f64>>,
    /// Within-Cluster Sum of Squares.
    pub wcss: f64,
    /// The configured upper bound on the number of iterations. Since linfa
    /// does not expose the actual iteration count, this value is always the
    /// setting passed as `max_n_iterations`, not the number of iterations
    /// actually performed at runtime.
    pub iterations: usize,
}

/// Result of estimating the cluster count via the elbow method.
#[derive(Debug, Clone)]
pub struct ElbowResult {
    /// WCSS for each cluster count k = 2..=max_k.
    pub wcss_per_k: Vec<f64>,
    /// Recommended cluster count, estimated from the bend (max second
    /// difference) in the WCSS curve.
    pub recommended_k: usize,
}
