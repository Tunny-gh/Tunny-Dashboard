/// Trained Gaussian Process model.
pub(crate) struct GpModel {
    pub alpha: Vec<f64>,
    pub x_train: Vec<Vec<f64>>,
    pub log_ls: Vec<f64>,
    pub log_sf: f64,
}
