use std::ffi::{CStr, CString};

use crate::lgbm_sys::{
    self, BoosterHandle, DatasetHandle, C_API_DTYPE_FLOAT32, C_API_DTYPE_FLOAT64,
    C_API_FEATURE_IMPORTANCE_GAIN, C_API_PREDICT_CONTRIB, C_API_PREDICT_NORMAL,
};
use crate::math::grid::linspace;

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

// SAFETY: LightGBM's Booster handle holds no thread-specific state (TLS,
// etc.), so moving ownership to another thread is safe.
// Sync is not implemented: concurrent predict calls on the same handle are
// not thread-safe in the LightGBM C API, so if sharing is required, the
// caller must serialize access with a Mutex or similar.
unsafe impl Send for LgbmBooster {}

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

/// Range-checked `usize -> i32` conversion for dimensions passed to the C API.
fn dim_to_i32(n: usize, what: &str) -> Result<i32, LgbmError> {
    i32::try_from(n).map_err(|_| LgbmError(format!("{what} {n} exceeds i32::MAX")))
}

/// Validates that `x` is a non-empty rectangular matrix and returns its column
/// count. LightGBM reads `nrow * ncol` contiguous f64s, so a ragged matrix
/// (rows shorter than `x[0]`) would make the C API read past the flattened
/// buffer (out-of-bounds). Returns `None` for empty or ragged input.
fn rectangular_ncol(x: &[Vec<f64>]) -> Option<usize> {
    let ncol = x.first()?.len();
    if ncol == 0 || x.iter().any(|row| row.len() != ncol) {
        return None;
    }
    Some(ncol)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Creates a LightGBM dataset from a feature matrix and label vector.
pub fn to_lgbm_dataset(x: &[Vec<f64>], y: &[f64]) -> Result<LgbmDataset, LgbmError> {
    if x.is_empty() || y.is_empty() {
        return Err(LgbmError("empty data".into()));
    }
    let ncol_usize = rectangular_ncol(x)
        .ok_or_else(|| LgbmError("feature matrix must be non-empty and rectangular".into()))?;
    if x.len() != y.len() {
        return Err(LgbmError("row count and label count differ".into()));
    }
    let nrow = dim_to_i32(x.len(), "nrow")?;
    let ncol = dim_to_i32(ncol_usize, "ncol")?;
    let n_labels = dim_to_i32(y.len(), "label count")?;
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
            n_labels,
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
    let dataset = match to_lgbm_dataset(x, y) {
        Ok(ds) => ds,
        Err(e) => {
            log_lgbm_failure("dataset creation", &e.0);
            return None;
        }
    };

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
    let params = match make_cstring(&params_str) {
        Ok(p) => p,
        Err(e) => {
            log_lgbm_failure("parameter string", &e.0);
            return None;
        }
    };

    let mut booster: BoosterHandle = std::ptr::null_mut();
    let rc = unsafe { lgbm_sys::LGBM_BoosterCreate(dataset.0, params.as_ptr(), &mut booster) };
    if rc != 0 {
        log_lgbm_failure("booster creation", &LgbmError::last().0);
        return None;
    }
    // Wrap immediately so the booster is freed on any early return below.
    let booster = LgbmBooster(booster);

    for _ in 0..config.num_iterations {
        let mut is_finished = 0i32;
        let rc = unsafe { lgbm_sys::LGBM_BoosterUpdateOneIter(booster.0, &mut is_finished) };
        // rc != 0 is a genuine training failure and must be distinguished from
        // the is_finished == 1 early-stop (which is a normal successful end).
        if rc != 0 {
            log_lgbm_failure("boosting iteration", &LgbmError::last().0);
            return None;
        }
        if is_finished != 0 {
            break;
        }
    }

    Some(booster)
}

/// Records an FFI failure so the underlying cause is not silently lost.
/// Debug builds only, to avoid polluting release output; the public API still
/// degrades gracefully by returning `None`/empty.
fn log_lgbm_failure(stage: &str, msg: &str) {
    #[cfg(debug_assertions)]
    eprintln!("LightGBM {stage} failed: {msg}");
    #[cfg(not(debug_assertions))]
    {
        let _ = (stage, msg);
    }
}

/// Predicts with a trained booster (normal regression output).
///
/// Returns `None` when the underlying FFI call fails (invalid/ragged input, a
/// LightGBM error, etc.) so callers can distinguish failure from an empty result
/// instead of it being silently flattened into an empty `Vec`.
pub fn lgbm_predict(booster: &LgbmBooster, x: &[Vec<f64>]) -> Option<Vec<f64>> {
    predict_internal(booster, x, C_API_PREDICT_NORMAL)
}

/// Predicts SHAP contribution values.
///
/// Returns a flat-reshaped `Vec<Vec<f64>>` with shape `[n_samples][n_features + 1]`.
/// The last column is the bias term. An empty `Vec` signals a failed/empty prediction.
pub fn lgbm_predict_contrib(booster: &LgbmBooster, x: &[Vec<f64>]) -> Vec<Vec<f64>> {
    if x.is_empty() {
        return Vec::new();
    }
    let ncol = x[0].len();
    let Some(flat) = predict_internal(booster, x, C_API_PREDICT_CONTRIB) else {
        return Vec::new();
    };
    // cols = ncol + 1 is always >= 1, so no zero-chunk guard is needed.
    let cols = ncol + 1;
    if flat.is_empty() {
        return Vec::new();
    }
    flat.chunks(cols).map(|c| c.to_vec()).collect()
}

/// Computes MSE on evaluation data.
pub fn lgbm_mse(booster: &LgbmBooster, x_eval: &[Vec<f64>], y_eval: &[f64]) -> Option<f64> {
    if y_eval.is_empty() {
        return None;
    }
    let preds = lgbm_predict(booster, x_eval)?;
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
    let zeros = || vec![0.0f64; n_features];

    // LGBM_BoosterFeatureImportance writes exactly the model's own feature count,
    // regardless of the caller's `n_features`. Query it first and size the buffer
    // to it; a mismatch with the caller's expectation means the importances would
    // not line up with the caller's parameter list, so fail safe with zeros.
    let mut model_features: i32 = 0;
    let rc = unsafe { lgbm_sys::LGBM_BoosterGetNumFeature(booster.0, &mut model_features) };
    if rc != 0 || model_features < 0 {
        log_lgbm_failure("feature count query", &LgbmError::last().0);
        return zeros();
    }
    let model_features = model_features as usize;
    if model_features != n_features {
        log_lgbm_failure(
            "feature importance",
            &format!("model has {model_features} features but caller expected {n_features}"),
        );
        return zeros();
    }

    // Buffer sized to the model's actual feature count avoids any out-of-bounds
    // write by the C API.
    let mut out = vec![0.0f64; model_features];
    let rc = unsafe {
        lgbm_sys::LGBM_BoosterFeatureImportance(
            booster.0,
            -1, // all iterations
            C_API_FEATURE_IMPORTANCE_GAIN,
            out.as_mut_ptr(),
        )
    };
    if rc != 0 {
        log_lgbm_failure("feature importance", &LgbmError::last().0);
        return zeros();
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
    // Ragged input would make the C API read past the flattened buffer (OOB).
    let ncol_usize = rectangular_ncol(x)?;
    let nrow = dim_to_i32(x.len(), "nrow").ok()?;
    let ncol = dim_to_i32(ncol_usize, "ncol").ok()?;
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
    // LGBM_BoosterPredictForMat's contract is that actual_len never exceeds the
    // out_len computed by LGBM_BoosterCalcNumPredict for the same inputs. If it
    // did, the C call would already have written past the end of `out`, so this
    // is a best-effort UB detector rather than a preventable condition.
    assert!(
        actual_len <= out_len,
        "LightGBM wrote {actual_len} predictions but the buffer was only sized for {out_len}"
    );
    out.truncate(actual_len as usize);
    Some(out)
}

// ── 2D PDP ────────────────────────────────────────────────────────────────────

type Pdp2dResult = Option<(Vec<f64>, Vec<f64>, Vec<Vec<f64>>, f64)>;

/// Compute a 2D partial dependence surface using a LightGBM RandomForest.
///
/// Trains on the full feature matrix. For each grid cell `(g1, g2)` the two
/// target columns are fixed to those values in every row; the average prediction
/// gives the PDP value, marginalising out all non-target dimensions.
///
/// Returns `(grid1, grid2, z_values, r_squared)` where `z_values` has shape
/// `[n_grid][n_grid]` and grid axes span the data range of each target column.
pub fn compute_pdp_2d_lgbm(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param1_idx: usize,
    param2_idx: usize,
    n_grid: usize,
) -> Pdp2dResult {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid < 2 {
        return None;
    }
    let p = x_matrix[0].len();
    if param1_idx >= p || param2_idx >= p {
        return None;
    }

    let config = LgbmRfConfig {
        num_iterations: 100,
        ..Default::default()
    };
    let booster = train_lgbm_rf(x_matrix, y, &config)?;

    let min1 = x_matrix
        .iter()
        .map(|r| r[param1_idx])
        .fold(f64::INFINITY, f64::min);
    let max1 = x_matrix
        .iter()
        .map(|r| r[param1_idx])
        .fold(f64::NEG_INFINITY, f64::max);
    let min2 = x_matrix
        .iter()
        .map(|r| r[param2_idx])
        .fold(f64::INFINITY, f64::min);
    let max2 = x_matrix
        .iter()
        .map(|r| r[param2_idx])
        .fold(f64::NEG_INFINITY, f64::max);

    let grid1 = linspace(min1, max1, n_grid);
    let grid2 = linspace(min2, max2, n_grid);
    let n_rows = x_matrix.len();

    // For each grid cell, fix the two target columns in every row and average the
    // predictions (marginalising over the remaining dimensions). Build one big
    // batch — n_grid×n_grid cells × n_rows rows — and predict once.
    let all_rows: Vec<Vec<f64>> = grid1
        .iter()
        .flat_map(|&g1| {
            grid2.iter().flat_map(move |&g2| {
                x_matrix.iter().map(move |r| {
                    let mut row = r.clone();
                    row[param1_idx] = g1;
                    row[param2_idx] = g2;
                    row
                })
            })
        })
        .collect();
    let flat_preds = lgbm_predict(&booster, &all_rows)?;
    if flat_preds.len() != n_grid * n_grid * n_rows {
        return None;
    }

    // Average each block of n_rows into one PDP value, then chunk into rows.
    let averaged: Vec<f64> = flat_preds
        .chunks(n_rows)
        .map(|chunk| chunk.iter().sum::<f64>() / chunk.len() as f64)
        .collect();
    let z_values: Vec<Vec<f64>> = averaged.chunks(n_grid).map(|c| c.to_vec()).collect();

    let mse = lgbm_mse(&booster, x_matrix, y)?;
    let r_squared = mse_to_r_squared(mse, y);

    Some((grid1, grid2, z_values, r_squared))
}

// ── 1D PDP ────────────────────────────────────────────────────────────────────

type Pdp1dResult = Option<(Vec<f64>, Vec<f64>, f64)>;

/// 1D partial dependence curve using a LightGBM RandomForest.
///
/// Trains on the full feature matrix. For each grid point v the target column
/// is fixed to v in every row; the average prediction gives the PDP value.
///
/// Returns `(grid, values, r_squared)` where `grid` spans the data range of
/// the target column and `values.len() == grid.len() == n_grid`.
pub fn compute_pdp_1d_lgbm(
    x_matrix: &[Vec<f64>],
    y: &[f64],
    param_idx: usize,
    n_grid: usize,
) -> Pdp1dResult {
    let n = y.len();
    if n < 2 || x_matrix.is_empty() || n_grid < 2 {
        return None;
    }
    let p = x_matrix[0].len();
    if param_idx >= p {
        return None;
    }

    let config = LgbmRfConfig {
        num_iterations: 100,
        ..Default::default()
    };
    let booster = train_lgbm_rf(x_matrix, y, &config)?;

    let min_j = x_matrix
        .iter()
        .map(|r| r[param_idx])
        .fold(f64::INFINITY, f64::min);
    let max_j = x_matrix
        .iter()
        .map(|r| r[param_idx])
        .fold(f64::NEG_INFINITY, f64::max);
    let grid = linspace(min_j, max_j, n_grid);
    let n_rows = x_matrix.len();

    // Single batch: create all grid-varied rows, predict once, then average.
    let all_rows: Vec<Vec<f64>> = grid
        .iter()
        .flat_map(|&v| {
            x_matrix.iter().map(move |r| {
                let mut row = r.clone();
                row[param_idx] = v;
                row
            })
        })
        .collect();
    let all_preds = lgbm_predict(&booster, &all_rows)?;
    if all_preds.len() != n_grid * n_rows {
        return None;
    }
    let values: Vec<f64> = all_preds
        .chunks(n_rows)
        .map(|chunk| chunk.iter().sum::<f64>() / chunk.len() as f64)
        .collect();

    let mse = lgbm_mse(&booster, x_matrix, y)?;
    let r_squared = mse_to_r_squared(mse, y);

    Some((grid, values, r_squared))
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
        let preds = lgbm_predict(&booster, &x).expect("predict should succeed");
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
        let preds = lgbm_predict(&booster, &x).expect("predict should succeed");
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

    #[test]
    fn pdp_1d_lgbm_shape() {
        let (x, y) = synthetic_data(30);
        let (grid, values, r_squared) =
            compute_pdp_1d_lgbm(&x, &y, 0, 5).expect("pdp_1d_lgbm should return Some");
        assert_eq!(grid.len(), 5);
        assert_eq!(values.len(), 5);
        assert!(r_squared.is_finite());
    }

    #[test]
    fn pdp_1d_lgbm_returns_none_for_invalid_input() {
        let (x, y) = synthetic_data(30);
        assert!(compute_pdp_1d_lgbm(&x, &y, 0, 0).is_none()); // n_grid < 2
        assert!(compute_pdp_1d_lgbm(&x, &y, 99, 5).is_none()); // param_idx out of bounds
        assert!(compute_pdp_1d_lgbm(&[], &[], 0, 5).is_none()); // empty data
    }

    #[test]
    fn pdp_1d_lgbm_monotone_for_linear_data() {
        let n = 40;
        let x: Vec<Vec<f64>> = (0..n).map(|i| vec![i as f64, 0.0]).collect();
        let y: Vec<f64> = x.iter().map(|r| r[0] * 2.0).collect();
        let (_, values, _) = compute_pdp_1d_lgbm(&x, &y, 0, 10).unwrap();
        // PDP should be non-decreasing for linear data
        for i in 0..values.len() - 1 {
            assert!(
                values[i] <= values[i + 1] + 1e-6,
                "PDP should be non-decreasing"
            );
        }
    }
}
