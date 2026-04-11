//! Module documentation.
//!
//! Module documentation.
//! Design:
//! Module documentation.
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Module documentation.
//! Module documentation.
//! Module documentation.
//! Module documentation.
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-1101

mod csv;
mod formatting;
mod report;

pub use csv::serialize_csv;
pub use report::compute_report_stats;

#[cfg(test)]
use csv::{parse_columns_json, serialize_csv_from_df};
#[cfg(test)]
use formatting::{escape_csv_field, format_f64};
#[cfg(test)]
use report::compute_report_stats_from_df;

#[cfg(test)]
mod tests;
