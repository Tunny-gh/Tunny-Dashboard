use std::cell::RefCell;

use super::model::DataFrame;
use super::types::SelectStudyResult;

struct GlobalState {
    dataframes: Vec<DataFrame>,
    /// Documentation.
    active_study_id: Option<u32>,
}

thread_local! {
/// Documentation.
/// Documentation.
    static GLOBAL_STATE: RefCell<GlobalState> = const {
        RefCell::new(GlobalState {
            dataframes: Vec::new(),
            active_study_id: None,
        })
    };
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn store_dataframes(dfs: Vec<DataFrame>) {
    GLOBAL_STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.dataframes = dfs;
        s.active_study_id = None;
    });
}

/// Documentation.
///
/// Documentation.
/// Documentation.
pub fn select_study(study_id: u32) -> Result<SelectStudyResult, String> {
    GLOBAL_STATE.with(|state| {
        let mut s = state.borrow_mut();
        let result = s
            .dataframes
            .get(study_id as usize)
            .map(|df| SelectStudyResult {
                data_frame_info: df.info(),
                gpu_buffer_data: df.gpu_buffers(),
            })
            .ok_or_else(|| {
                format!(
                    "study_id {} not found (total: {})",
                    study_id,
                    s.dataframes.len()
                )
            });
        if result.is_ok() {
            s.active_study_id = Some(study_id);
        }
        result
    })
}

/// Documentation.
///
/// Documentation.
pub fn with_active_df<T, F: FnOnce(&DataFrame) -> T>(f: F) -> Option<T> {
    GLOBAL_STATE.with(|state| {
        let s = state.borrow();
        let idx = s.active_study_id? as usize;
        s.dataframes.get(idx).map(f)
    })
}

/// Documentation.
pub fn with_df<T, F: FnOnce(&DataFrame) -> T>(study_id: u32, f: F) -> Option<T> {
    GLOBAL_STATE.with(|state| {
        let s = state.borrow();
        s.dataframes.get(study_id as usize).map(f)
    })
}
