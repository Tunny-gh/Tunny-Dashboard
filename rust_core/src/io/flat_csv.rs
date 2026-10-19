//! D4/D7 legacy CSV projection facade; format interpretation lives in I/O.
use crate::dataframe::DataFrame;
#[cfg(test)]
use crate::journal_parser::OptimizationDirection;
use crate::journal_parser::{project_decoded, StudyMeta};
pub struct FlatCsvParseResult {
    pub meta: StudyMeta,
    pub dataframe: DataFrame,
    pub images: Vec<(u32, String)>,
}
pub fn parse_flat_csv(data: &[u8], name: &str) -> Result<FlatCsvParseResult, String> {
    let parsed = tdashboard_io::flat_csv::parse_flat_csv(data, name)?;
    Ok(FlatCsvParseResult {
        meta: parsed.meta,
        dataframe: project_decoded(parsed.dataframe),
        images: parsed.images,
    })
}
#[cfg(test)]
mod tests;
