//! MCDM chart module root.
//!
//! Split into submodules by responsibility:
//! - `compute`: compute-request/cache-key types shared across MCDM charts.
//! - `controls`: shared settings UI and execution state (`McdmControls`).
//! - `rank_chart`: the ranking bar chart widget.
//! - `table`: the ranking table widget.
//! - `ranking`: ranking-row construction shared by `rank_chart` and `table`.
//!
//! Everything is re-exported here so existing paths through
//! `crate::ui::widgets::decision::mcdm_chart::X` keep working unchanged.

mod compute;
mod controls;
mod rank_chart;
mod ranking;
mod table;
#[cfg(test)]
mod tests;

pub use compute::{McdmCacheKey, McdmComputeRequest};
pub use controls::McdmControls;
pub use rank_chart::McdmRankChart;
pub use table::McdmTable;

// Re-exported here (test-only) so that `tests`'s `use super::*;` can resolve
// these external types without each submodule needing to expose them.
#[cfg(test)]
use crate::state::results::{McdmMethod, McdmResult, WeightMode};
#[cfg(test)]
use crate::state::types::StudyView;
