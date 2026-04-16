/// Trained Gaussian Process model.
pub(crate) struct GpModel {
    pub alpha: Vec<f64>,
    pub x_train: Vec<Vec<f64>>,
    pub log_ls: Vec<f64>,
    pub log_sf: f64,
    /// Cholesky factor L of (K_XX + σ_n² I), used for variance prediction.
    pub l: Vec<Vec<f64>>,
    /// Log noise standard deviation.
    #[allow(dead_code)]
    pub log_sn: f64,
}
