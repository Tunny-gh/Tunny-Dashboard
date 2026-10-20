//! Neutral CSV field encoding; row selection and statistics remain with callers.
pub mod formatting;
mod writer;
pub use writer::{CsvField, CsvWriter};
