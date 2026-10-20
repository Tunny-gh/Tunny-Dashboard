//! D4/D7 legacy projection facade; SQLite acquisition/session lives in I/O.
use crate::data::extras::StudyExtras;
use crate::dataframe::DataFrame;
use crate::journal_parser::{project_decoded, StudyMeta};
pub use tdashboard_io::sqlite::*;
pub fn parse_single_study(
    path: &std::path::Path,
    id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let (m, t, e) = tdashboard_io::sqlite::parse_single_study(path, id)?;
    Ok((m, project_decoded(t), e))
}
#[cfg(test)]
use crate::data::extras::TrialState;
#[cfg(test)]
use crate::journal_parser::OptimizationDirection;
#[cfg(test)]
use crate::rdb::{self, SqliteBackend};
#[cfg(test)]
use rusqlite::Connection;
#[cfg(test)]
mod tests;
