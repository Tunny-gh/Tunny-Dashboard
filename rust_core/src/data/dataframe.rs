//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/implements/TASK-102/dataframe-requirements.md

mod buffers;
mod model;
mod state;
mod types;

#[cfg(test)]
mod tests;

pub use model::DataFrame;
pub use state::{
    active_snapshot, select_study, snapshot, store_dataframes, swap_snapshot, with_active_df,
    with_df, SharedStudyStore,
};
pub use types::{DataFrameInfo, GpuBufferData, SelectStudyResult, TrialRow};
