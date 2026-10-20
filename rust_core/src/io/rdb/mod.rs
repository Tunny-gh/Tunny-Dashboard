//! D4/D7 legacy projection facade; storage queries and sessions live in I/O.
use crate::data::extras::StudyExtras;
use crate::dataframe::DataFrame;
use crate::journal_parser::{project_decoded, StudyMeta};
pub use tdashboard_io::rdb::*;
pub fn parse_single_study(
    backend: &mut dyn OptunaBackend,
    id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let (m, t, e) = tdashboard_io::rdb::parse_single_study(backend, id)?;
    Ok((m, project_decoded(t), e))
}
pub fn parse_single_study_url(
    url: &RdbUrl,
    id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let (m, t, e) = tdashboard_io::rdb::parse_single_study_url(url, id)?;
    Ok((m, project_decoded(t), e))
}
