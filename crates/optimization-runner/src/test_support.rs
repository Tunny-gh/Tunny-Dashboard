//! Recorded-result readback composed without the legacy core.
use tdashboard_analysis::{
    data::dataframe::DataFrame, data::extras::StudyExtras, projection::StudyMeta,
};
use tdashboard_study::{IdentityAllocator, LoadKind, StudyKey, StudyManager};

pub(crate) fn parse_single_study(
    data: &[u8],
    stored_id: u32,
) -> Result<(StudyMeta, DataFrame, StudyExtras), String> {
    let key = StudyKey {
        source: IdentityAllocator::new()
            .and_then(|mut a| a.source())
            .map_err(|e| e.to_string())?,
        stored_id,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager
        .begin(LoadKind::Initial)
        .map_err(|e| e.to_string())?;
    let batch = tdashboard_io::journal::parser::study::read_full(data, key)?;
    let receipt = manager
        .apply(manager.header(ticket).map_err(|e| e.to_string())?, batch)
        .map_err(|e| e.to_string())?;
    manager
        .finish(ticket, receipt.next_sequence)
        .map_err(|e| e.to_string())?;
    let p =
        tdashboard_analysis::projection::project(&manager.acquire().ok_or("No acquired Study")?);
    Ok((
        p.meta,
        std::sync::Arc::unwrap_or_clone(p.dataframe),
        p.extras,
    ))
}
