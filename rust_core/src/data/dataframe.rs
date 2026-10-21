//! The DataFrame holding Optuna trial data and its snapshot management.
//!
//! Composed of `DataFrame` holding column-oriented data (`model`), feasibility
//! determination (`feasibility`), per-study snapshot shared state (`state`), and row
//! data types (`types`).
//!
//! Reference: docs/implements/TASK-102/dataframe-requirements.md

mod state;
pub use tdashboard_analysis::dataframe::*;

#[cfg(test)]
mod tests;

pub use state::{
    active_extras_snapshot, active_snapshot, extras_snapshot, select_study, snapshot,
    store_dataframes, store_extras, store_extras_for, swap_extras, swap_snapshot, with_active_df,
    with_df, SharedStudyStore,
};
