//! Request-local I/O acquisition and explicit Study ownership.
pub use tdashboard_io::storage::scan_studies;
use tdashboard_study::*;

pub fn load_study(storage: &str, id: u32) -> Result<(StudyInput, String), String> {
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
    Ok((manager.acquire().ok_or("No acquired Study")?, display))
}
