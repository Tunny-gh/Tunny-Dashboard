//! Raw FFI bindings to the LightGBM C API.
//!
//! Links directly against `libs/lib_lightgbm.dll` (Windows) or
//! `libs/lib_lightgbm.dylib` (macOS) placed at the workspace root.
//! On Windows the import library `libs/lib_lightgbm.lib` must also be present
//! (generated from the DLL exports via `lib.exe /def`).

use std::ffi::{c_char, c_void};

pub type DatasetHandle = *mut c_void;
pub type BoosterHandle = *mut c_void;

// Data-type tags passed to the C API
pub const C_API_DTYPE_FLOAT32: i32 = 0;
pub const C_API_DTYPE_FLOAT64: i32 = 1;

// Prediction type tags
pub const C_API_PREDICT_NORMAL: i32 = 0;
#[allow(dead_code)]
pub const C_API_PREDICT_RAWSCORE: i32 = 1;
#[allow(dead_code)]
pub const C_API_PREDICT_LEAFINDEX: i32 = 2;
pub const C_API_PREDICT_CONTRIB: i32 = 3;

// Feature importance types
#[allow(dead_code)]
pub const C_API_FEATURE_IMPORTANCE_SPLIT: i32 = 0;
pub const C_API_FEATURE_IMPORTANCE_GAIN: i32 = 1;

extern "C" {
    /// Returns a pointer to the last error message string.
    pub fn LGBM_GetLastError() -> *const c_char;

    /// Creates a dataset from a dense matrix stored as a flat f64 array.
    pub fn LGBM_DatasetCreateFromMat(
        data: *const c_void,
        data_type: i32,
        nrow: i32,
        ncol: i32,
        is_row_major: i32,
        parameters: *const c_char,
        reference: DatasetHandle,
        out: *mut DatasetHandle,
    ) -> i32;

    /// Sets a metadata field (e.g. "label", "weight") on a dataset.
    pub fn LGBM_DatasetSetField(
        handle: DatasetHandle,
        field_name: *const c_char,
        field_data: *const c_void,
        num_element: i32,
        data_type: i32,
    ) -> i32;

    /// Frees a dataset.
    pub fn LGBM_DatasetFree(handle: DatasetHandle) -> i32;

    /// Returns the number of data rows in a dataset.
    #[allow(dead_code)]
    pub fn LGBM_DatasetGetNumData(handle: DatasetHandle, out: *mut i32) -> i32;

    /// Returns the number of features in a dataset.
    #[allow(dead_code)]
    pub fn LGBM_DatasetGetNumFeature(handle: DatasetHandle, out: *mut i32) -> i32;

    /// Creates a booster (trains one iteration internally if parameters include them).
    pub fn LGBM_BoosterCreate(
        train_data: DatasetHandle,
        parameters: *const c_char,
        out: *mut BoosterHandle,
    ) -> i32;

    /// Frees a booster.
    pub fn LGBM_BoosterFree(handle: BoosterHandle) -> i32;

    /// Runs one training iteration.  Sets `*is_finished` to 1 when done.
    pub fn LGBM_BoosterUpdateOneIter(handle: BoosterHandle, is_finished: *mut i32) -> i32;

    /// Returns the current training iteration count.
    #[allow(dead_code)]
    pub fn LGBM_BoosterGetCurrentIteration(handle: BoosterHandle, out_iteration: *mut i32) -> i32;

    /// Returns the number of features used by the booster.
    #[allow(dead_code)]
    pub fn LGBM_BoosterGetNumFeature(handle: BoosterHandle, out_len: *mut i32) -> i32;

    /// Computes the number of output values for a given prediction call.
    pub fn LGBM_BoosterCalcNumPredict(
        handle: BoosterHandle,
        num_row: i32,
        predict_type: i32,
        start_iteration: i32,
        num_iteration: i32,
        out_len: *mut i64,
    ) -> i32;

    /// Predicts for a dense matrix.  `out_result` must have capacity `*out_len`.
    pub fn LGBM_BoosterPredictForMat(
        handle: BoosterHandle,
        data: *const c_void,
        data_type: i32,
        nrow: i32,
        ncol: i32,
        is_row_major: i32,
        predict_type: i32,
        start_iteration: i32,
        num_iteration: i32,
        parameter: *const c_char,
        out_len: *mut i64,
        out_result: *mut f64,
    ) -> i32;

    /// Computes feature importances.  `out_results` must have length `num_feature`.
    /// `importance_type`: 0 = SPLIT count, 1 = GAIN.
    pub fn LGBM_BoosterFeatureImportance(
        handle: BoosterHandle,
        num_iteration: i32,
        importance_type: i32,
        out_results: *mut f64,
    ) -> i32;
}
