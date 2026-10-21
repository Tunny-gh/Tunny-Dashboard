//! D4/D7 legacy CSV projection facade; format interpretation lives in I/O.
use crate::dataframe::DataFrame;
#[cfg(test)]
use crate::journal_parser::OptimizationDirection;
use crate::journal_parser::{io_meta, new_key, project_batch, StudyMeta};
pub struct FlatCsvParseResult {
    pub meta: StudyMeta,
    pub dataframe: DataFrame,
    pub images: Vec<(u32, String)>,
}
pub fn parse_flat_csv(data: &[u8], name: &str) -> Result<FlatCsvParseResult, String> {
    let parsed = tdashboard_io::flat_csv::parse_flat_csv(data, name)?;
    let images = parsed.images.clone();
    let key = new_key(parsed.meta.study_id)?;
    let p = project_batch(key, parsed.into_batch(key))?;
    Ok(FlatCsvParseResult {
        meta: io_meta(p.meta),
        dataframe: std::sync::Arc::unwrap_or_clone(p.dataframe),
        images,
    })
}
#[cfg(test)]
mod tests;
