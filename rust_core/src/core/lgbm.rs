use std::ffi::{CStr, CString};

use crate::lgbm_sys::{
    self, BoosterHandle, DatasetHandle, C_API_DTYPE_FLOAT32, C_API_DTYPE_FLOAT64,
    C_API_FEATURE_IMPORTANCE_GAIN, C_API_PREDICT_CONTRIB, C_API_PREDICT_NORMAL,
};

// ── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct LgbmError(pub String);

impl LgbmError {
    fn last() -> Self {
        let msg = unsafe {
            let ptr = lgbm_sys::LGBM_GetLastError();
            if ptr.is_null() {
                "unknown LightGBM error".to_string()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        LgbmError(msg)
    }
}

impl std::fmt::Display for LgbmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LgbmError: {}", self.0)
    }
}

// ── Safe handle wrappers ─────────────────────────────────────────────────────

pub struct LgbmDataset(DatasetHandle);

impl Drop for LgbmDataset {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { lgbm_sys::LGBM_DatasetFree(self.0) };
        }
    }
}

pub struct LgbmBooster(pub(crate) BoosterHandle);

impl Drop for LgbmBooster {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { lgbm_sys::LGBM_BoosterFree(self.0) };
        }
    }
}

// ── Config ───────────────────────────────────────────────────────────────────

pub struct LgbmRfConfig {
    pub num_iterations: usize,
    pub max_depth: i32,
    pub min_data_in_leaf: i32,
    pub bagging_fraction: f64,
    pub feature_fraction: f64,
    pub seed: i32,
}

impl Default for LgbmRfConfig {
    fn default() -> Self {
        Self {
            num_iterations: 64,
            max_depth: 10,
            min_data_in_leaf: 2,
            bagging_fraction: 0.8,
            feature_fraction: 0.8,
            seed: 42,
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_cstring(s: &str) -> Result<CString, LgbmError> {
    CString::new(s).map_err(|e| LgbmError(e.to_string()))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Creates a LightGBM dataset from a feature matrix and label vector.
pub fn to_lgbm_dataset(x: &[Vec<f64>], y: &[f64]) -> Result<LgbmDataset, LgbmError> {
    if x.is_empty() || y.is_empty() {
        return Err(LgbmError("empty data".into()));
    }
    let nrow = x.len() as i32;
    let ncol = x[0].len() as i32;
    if ncol == 0 {
        return Err(LgbmError("no features".into()));
    }
    let flat_x: Vec<f64> = x.iter().flat_map(|row| row.iter().copied()).collect();
    // min_data_in_leaf must be set at dataset construction time so LightGBM
    // does not pre-discard bins that would be "too small" given its default of 20.
    let params = make_cstring("min_data_in_bin=1 min_data_in_leaf=1")?;

    let mut handle: DatasetHandle = std::ptr::null_mut();
    let rc = unsafe {
        lgbm_sys::LGBM_DatasetCreateFromMat(
            flat_x.as_ptr().cast(),
            C_API_DTYPE_FLOAT64,
            nrow,
            ncol,
            1, // row-major
            params.as_ptr(),
            std::ptr::null_mut(),
            &mut handle,
        )
    };
    if rc != 0 {
        return Err(LgbmError::last());
    }

    let label_field = make_cstring("label")?;
    let labels_f32: Vec<f32> = y.iter().map(|&v| v as f32).collect();
    let rc = unsafe {
        lgbm_sys::LGBM_DatasetSetField(
            handle,
            label_field.as_ptr(),
            labels_f32.as_ptr().cast(),
            y.len() as i32,
            C_API_DTYPE_FLOAT32,
        )
    };
    if rc != 0 {
        unsafe { lgbm_sys::LGBM_DatasetFree(handle) };
        return Err(LgbmError::last());
    }

    Ok(LgbmDataset(handle))
}

/// Trains a LightGBM model in RandomForest mode.
pub fn train_lgbm_rf(x: &[Vec<f64>], y: &[f64], config: &LgbmRfConfig) -> Option<LgbmBooster> {
    let dataset = to_lgbm_dataset(x, y).ok()?;

    let params_str = format!(
        "boosting_type=rf num_iterations={ni} max_depth={md} \
         min_data_in_leaf={ml} bagging_fraction={bf} bagging_freq=1 \
         feature_fraction={ff} verbose=-1 num_threads=1 seed={seed} \
         objective=regression min_data_in_bin=1",
        ni = config.num_iterations,
        md = config.max_depth,
        ml = config.min_data_in_leaf,
        bf = config.bagging_fraction,
        ff = config.feature_fraction,
        seed = config.seed,
    );
    let params = make_cstring(&params_str).ok()?;

    let mut booster: BoosterHandle = std::ptr::null_mut();
    let rc = unsafe { lgbm_sys::LGBM_BoosterCreate(dataset.0, params.as_ptr(), &mut booster) };
    if rc != 0 {
        return None;
    }

    for _ in 0..config.num_iterations {
        let mut is_finished = 0i32;
        let rc = unsafe { lgbm_sys::LGBM_BoosterUpdateOneIter(booster, &mut is_finished) };
        if rc != 0 || is_finished != 0 {
            break;
        }
    }

    Some(LgbmBooster(booster))
}

/// Predicts with a trained booster (normal regression output).
pub fn lgbm_predict(booster: &LgbmBooster, x: &[Vec<f64>]) -> Vec<f64> {
    predict_internal(booster, x, C_API_PREDICT_NORMAL).unwrap_or_default()
}

/// Predicts SHAP contribution values.
///
/// Returns a flat-reshaped `Vec<Vec<f64>>` with shape `[n_samples][n_features + 1]`.
/// The last column is the bias term.
pub fn lgbm_predict_contrib(booster: &LgbmBooster, x: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if x.is_empty() {
        return Vec::new();
    }
    let ncol = x[0].len();
    let flat = predict_internal(booster, x, C_API_PREDICT_CONTRIB).unwrap_or_default();
    let cols = ncol + 1;
    if flat.is_empty() || cols == 0 {
        return Vec::new();
    }
    flat.chunks(cols).map(|c| c.to_vec()).collect()
}

/// Computes MSE on evaluation data.
pub fn lgbm_mse(booster: &LgbmBooster, x_eval: &[Vec<f64>], y_eval: &[f64]) -> Option<f64> {
    if y_eval.is_empty() {
        return None;
    }
    let preds = lgbm_predict(booster, x_eval);
    if preds.is_empty() {
        return None;
    }
    let n = preds.len().min(y_eval.len());
    let mse = preds[..n]
        .iter()
        .zip(&y_eval[..n])
        .map(|(p, y)| (p - y).powi(2))
        .sum::<f64>()
        / n as f64;
    Some(mse)
}

/// Gain-based feature importance, normalised so the values sum to 1.0.
pub fn lgbm_feature_importance(booster: &LgbmBooster, n_features: usize) -> Vec<f64> {
    if n_features == 0 {
        return Vec::new();
    }
    let mut out = vec![0.0f64; n_features];
    let rc = unsafe {
        lgbm_sys::LGBM_BoosterFeatureImportance(
            booster.0,
            -1, // all iterations
            C_API_FEATURE_IMPORTANCE_GAIN,
            out.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return vec![0.0; n_features];
    }
    let sum: f64 = out.iter().sum();
    if sum > 0.0 {
        out.iter_mut().for_each(|v| *v /= sum);
    }
    out
}

/// Converts MSE to R² given the true label values.
pub fn mse_to_r_squared(mse: f64, y_eval: &[f64]) -> f64 {
    if y_eval.is_empty() {
        return 0.0;
    }
    let n = y_eval.len() as f64;
    let y_mean = y_eval.iter().sum::<f64>() / n;
    let ss_tot = y_eval.iter().map(|y| (y - y_mean).powi(2)).sum::<f64>();
    if ss_tot == 0.0 {
        return 0.0;
    }
    1.0 - (mse * n) / ss_tot
}

// ── Internal ──────────────────────────────────────────────────────────────────

fn predict_internal(booster: &LgbmBooster, x: &[Vec<f64>], predict_type: i32) -> Option<Vec<f64>> {
    if x.is_empty() {
        return None;
    }
    let nrow = x.len() as i32;
    let ncol = x[0].len() as i32;
    let flat_x: Vec<f64> = x.iter().flat_map(|row| row.iter().copied()).collect();
    let empty_param = make_cstring("").ok()?;

    let mut out_len: i64 = 0;
    let rc = unsafe {
        lgbm_sys::LGBM_BoosterCalcNumPredict(
            booster.0,
            nrow,
            predict_type,
            0,  // start_iteration
            -1, // all iterations
            &mut out_len,
        )
    };
    if rc != 0 || out_len <= 0 {
        return None;
    }

    let mut out = vec![0.0f64; out_len as usize];
    let mut actual_len: i64 = 0;
    let rc = unsafe {
        lgbm_sys::LGBM_BoosterPredictForMat(
            booster.0,
            flat_x.as_ptr().cast(),
            C_API_DTYPE_FLOAT64,
            nrow,
            ncol,
            1, // row-major
            predict_type,
            0,
            -1,
            empty_param.as_ptr(),
            &mut actual_len,
            out.as_mut_ptr(),
        )
    };
    if rc != 0 {
        return None;
    }
    out.truncate(actual_len as usize);
    Some(out)
}

// ── 2D PDP ────────────────────────────────────────────────────────────────────

/// Compute a 2D partial dependence surface using a LightGBM RandomForest.
///
/// Returns `(grid1, grid2, z_values, r_squared)` where `z_values` has shape
/// `[n_grid][n_grid]` and grid axes span the data range of each column.
pub fn compute_pdp_2d_lgbm(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>, f64)> {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid < 2 {
        return None;
    }
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return None;
    }

    let x2d: Vec<Vec<f64>> = x_matrix
        .iter()
        .map(|row| vec![row[param1_idx], row[param2_idx]])
        .collect();

    let config = LgbmRfConfig {
        num_iterations: 100,
        ..Default::default()
    };
    let booster = train_lgbm_rf(&x2d, y, &config)?;

    let min1 = x2d.iter().map(|r| r[0]).fold(f64::INFINITY, f64::min);
    let max1 = x2d.iter().map(|r| r[0]).fold(f64::NEG_INFINITY, f64::max);
    let min2 = x2d.iter().map(|r| r[1]).fold(f64::INFINITY, f64::min);
    let max2 = x2d.iter().map(|r| r[1]).fold(f64::NEG_INFINITY, f64::max);

    let grid1 = pdp_linspace(min1, max1, n_grid);
    let grid2 = pdp_linspace(min2, max2, n_grid);

    let grid_points: Vec<Vec<f64>> = grid1
        .iter()
        .flat_map(|&g1| grid2.iter().map(move |&g2| vec![g1, g2]))
        .collect();
    let flat_z = lgbm_predict(&booster, &grid_points);

    if flat_z.len() != n_grid * n_grid {
        return None;
    }
    let z_values: Vec<Vec<f64>> = flat_z.chunks(n_grid).map(|c| c.to_vec()).collect();

    let mse = lgbm_mse(&booster, &x2d, y)?;
    let r_squared = mse_to_r_squared(mse, y);

    Some((grid1, grid2, z_values, r_squared))
}

fn pdp_linspace(min: f64, max: f64, n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(min + max) / 2.0];
    }
    (0..n)
        .map(|i| min + (max - min) * i as f64 / (n - 1) as f64)
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_data(n: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, (i % 3) as f64]).collect();
        let y: Vec<f64> = x.iter().map(|row| row[0] * 2.0 + row[1]).collect();
        (x, y)
    }

    #[test]
    fn default_config() {
        let cfg = LgbmRfConfig::default();
        assert_eq!(cfg.num_iterations, 64);
        assert_eq!(cfg.max_depth, 10);
        assert_eq!(cfg.min_data_in_leaf, 2);
        assert!((cfg.bagging_fraction - 0.8).abs() < 1e-9);
        assert!((cfg.feature_fraction - 0.8).abs() < 1e-9);
        assert_eq!(cfg.seed, 42);
    }

    #[test]
    fn train_returns_booster() {
        let (x, y) = synthetic_data(30);
        let booster = train_lgbm_rf(&x, &y, &LgbmRfConfig::default());
        assert!(
            booster.is_some(),
            "train_lgbm_rf should return Some(booster)"
        );
    }

    #[test]
    fn predict_length_matches_input() {
        let (x, y) = synthetic_data(30);
        let booster = train_lgbm_rf(&x, &y, &LgbmRfConfig::default()).unwrap();
        let preds = lgbm_predict(&booster, &x);
        assert_eq!(preds.len(), x.len());
    }

    #[test]
    fn mse_is_non_negative() {
        let (x, y) = synthetic_data(30);
        let booster = train_lgbm_rf(&x, &y, &LgbmRfConfig::default()).unwrap();
        let mse = lgbm_mse(&booster, &x, &y).expect("mse should be Some");
        assert!(mse >= 0.0, "MSE must be non-negative");
    }

    #[test]
    fn mse_empty_returns_none() {
        let (x, y) = synthetic_data(30);
        let booster = train_lgbm_rf(&x, &y, &LgbmRfConfig::default()).unwrap();
        assert!(lgbm_mse(&booster, &[], &[]).is_none());
    }

    #[test]
    fn feature_importance_sums_to_one() {
        let (x, y) = synthetic_data(30);
        let booster = train_lgbm_rf(&x, &y, &LgbmRfConfig::default()).unwrap();
        let imp = lgbm_feature_importance(&booster, 2);
        assert_eq!(imp.len(), 2);
        let sum: f64 = imp.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6 || sum == 0.0);
    }

    #[test]
    fn mse_to_r2_basic() {
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let r2 = mse_to_r_squared(0.0, &y);
        assert!((r2 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn predict_contrib_shape() {
        let (x, y) = synthetic_data(30);
        let booster = train_lgbm_rf(&x, &y, &LgbmRfConfig::default()).unwrap();
        let contrib = lgbm_predict_contrib(&booster, &x[..3]);
        // shape should be [3][n_features + 1] = [3][3]
        assert_eq!(contrib.len(), 3);
        assert_eq!(contrib[0].len(), x[0].len() + 1);
    }

    #[test]
    fn pipeline_train_predict_mse() {
        let (x, y) = synthetic_data(40);
        let dataset = to_lgbm_dataset(&x, &y);
        assert!(dataset.is_ok(), "to_lgbm_dataset should succeed");
        let booster = train_lgbm_rf(&x, &y, &LgbmRfConfig::default()).unwrap();
        let preds = lgbm_predict(&booster, &x);
        assert_eq!(preds.len(), x.len());
        let mse = lgbm_mse(&booster, &x, &y).unwrap();
        assert!(mse.is_finite());
    }

    #[test]
    fn pdp_2d_lgbm_shape() {
        let (x, y) = synthetic_data(30);
        let (grid1, grid2, z_values, r_squared) =
            compute_pdp_2d_lgbm(&x, &y, 0, 1, 5).expect("pdp_2d_lgbm should return Some");
        assert_eq!(grid1.len(), 5);
        assert_eq!(grid2.len(), 5);
        assert_eq!(z_values.len(), 5);
        assert_eq!(z_values[0].len(), 5);
        assert!(r_squared.is_finite());
    }

    #[test]
    fn pdp_2d_lgbm_returns_none_for_invalid_input() {
        let (x, y) = synthetic_data(30);
        assert!(compute_pdp_2d_lgbm(&x, &y, 0, 1, 0).is_none());
        assert!(compute_pdp_2d_lgbm(&x, &y, 0, 99, 5).is_none());
        assert!(compute_pdp_2d_lgbm(&[], &[], 0, 1, 5).is_none());
    }
}
