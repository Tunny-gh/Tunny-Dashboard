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
pub use state::{select_study, store_dataframes, with_active_df, with_df};
pub use types::{DataFrameInfo, GpuBufferData, SelectStudyResult, TrialRow};
