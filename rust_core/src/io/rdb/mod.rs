//! D4/D7 legacy projection facade; storage queries and sessions live in I/O.
use crate::data::extras::StudyExtras;
use crate::dataframe::DataFrame;
use crate::journal_parser::{io_meta, new_key, project_batch, StudyMeta};
pub use tdashboard_io::rdb::*;
pub fn parse_single_study(
    backend: &mut dyn OptunaBackend,
    id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let key = new_key(id)?;
    let rows = tdashboard_io::rdb::parse_single_study_rows(backend, id)?;
    let p = project_batch(key, rows.into_batch(key))?;
    Ok((
        io_meta(p.meta),
        std::sync::Arc::unwrap_or_clone(p.dataframe),
        p.extras,
    ))
}
pub fn parse_single_study_url(
    url: &RdbUrl,
    id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let key = new_key(id)?;
    let rows = tdashboard_io::rdb::parse_single_study_rows_url(url, id)?;
    let p = project_batch(key, rows.into_batch(key))?;
    Ok((
        io_meta(p.meta),
        std::sync::Arc::unwrap_or_clone(p.dataframe),
        p.extras,
    ))
}
