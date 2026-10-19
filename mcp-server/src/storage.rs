//! Request-local I/O -> explicit Study management -> legacy analysis projection.
//! D4 replaces projection; D7 removes the remaining report/entry import glue.
pub use tdashboard_io::storage::scan_studies;
use tdashboard_study::*;

pub fn load_study(
    storage: &str,
    id: u32,
) -> Result<
    (
        tunny_core::journal_parser::StudyMeta,
        tunny_core::dataframe::DataFrame,
        tunny_core::data::extras::StudyExtras,
        String,
    ),
    String,
> {
    let key = StudyKey {
        source: IdentityAllocator::new()
            .and_then(|mut a| a.source())
            .map_err(|e| e.to_string())?,
        stored_id: id,
    };
    let (batch, display) = tdashboard_io::storage::load_study(storage, key)?;
    let mut manager = StudyManager::new(key);
    let ticket = manager
        .begin(LoadKind::Initial)
        .map_err(|e| e.to_string())?;
    let receipt = manager
        .apply(manager.header(ticket).map_err(|e| e.to_string())?, batch)
        .map_err(|e| e.to_string())?;
    manager
        .finish(ticket, receipt.next_sequence)
        .map_err(|e| e.to_string())?;
    let projection = tunny_core::journal_parser::study_projection::project(
        &manager.acquire().ok_or("No acquired Study")?,
    );
    Ok((
        projection.meta,
        projection.dataframe,
        projection.extras,
        display,
    ))
}
