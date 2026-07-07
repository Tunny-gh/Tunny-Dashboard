//! Optuna の試行データを保持する DataFrame とそのスナップショット管理。
//!
//! 列指向データを保持する `DataFrame`（`model`）、feasibility 判定（`feasibility`）、
//! Study ごとのスナップショット共有状態（`state`）、行データ型（`types`）で構成する。
//!
//! Reference: docs/implements/TASK-102/dataframe-requirements.md

mod feasibility;
mod model;
mod state;
mod types;

#[cfg(test)]
mod tests;

pub use feasibility::Feasibility;
pub use model::DataFrame;
pub use state::{
    active_extras_snapshot, active_snapshot, extras_snapshot, select_study, snapshot,
    store_dataframes, store_extras, store_extras_for, swap_extras, swap_snapshot, with_active_df,
    with_df, SharedStudyStore,
};
pub use types::TrialRow;
