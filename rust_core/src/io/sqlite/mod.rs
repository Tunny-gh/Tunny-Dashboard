//! D4/D7 legacy projection facade; SQLite acquisition/session lives in I/O.
use crate::data::extras::StudyExtras;
use crate::dataframe::DataFrame;
use crate::journal_parser::{io_meta, new_key, project_batch, StudyMeta};
pub use tdashboard_io::sqlite::*;
pub fn parse_single_study(
    path: &std::path::Path,
    id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let key = new_key(id)?;
    let rows = tdashboard_io::sqlite::parse_single_study_rows(path, id)?;
    let p = project_batch(key, rows.into_batch(key))?;
    Ok((io_meta(p.meta), p.dataframe, p.extras))
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
