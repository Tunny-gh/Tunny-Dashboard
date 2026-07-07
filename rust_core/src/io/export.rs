//! CSV 出力・レポート集計機能を提供するモジュール。
//!
//! Design:
//! テキスト生成（`csv` / `formatting` / `report`）と実ファイル書き込み（`writer`）を分離し、
//! CSV エスケープ・数式インジェクション対策・数値フォーマットは `formatting` に集約する。
//!
//! Reference: docs/tasks/tunny-dashboard-tasks.md TASK-1101

mod csv;
mod formatting;
mod report;
mod writer;

pub use csv::serialize_csv;
pub use report::compute_report_stats;
pub use writer::{CsvField, CsvWriter};

#[cfg(test)]
use csv::{parse_columns_json, serialize_csv_from_df};
#[cfg(test)]
use formatting::{escape_csv_field, format_f64};
#[cfg(test)]
use report::compute_report_stats_from_df;

#[cfg(test)]
mod tests;
