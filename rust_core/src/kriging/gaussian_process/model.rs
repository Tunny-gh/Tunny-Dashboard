#[derive(Debug, Clone)]
pub(crate) struct GpKernel {
    pub log_ls: Vec<f64>,
    pub log_sf: f64,
    pub log_sn: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct GpFittedModel {
    pub kernel: GpKernel,
    pub alpha: Vec<f64>,
    pub x_train: Vec<Vec<f64>>,
    pub l: Vec<Vec<f64>>,
}

pub(crate) type GpModel = GpFittedModel;
