use crate::state::messages::AppMessage;

pub fn fixture_chunk(
    stored_id: u32,
    read: impl FnOnce(tdashboard_study::StudyKey) -> Result<tdashboard_study::StudyBatch, String>,
) -> Result<AppMessage, String> {
    use tdashboard_study::*;
    let key = StudyKey {
        source: IdentityAllocator::new()
            .and_then(|mut a| a.source())
            .map_err(|e| e.to_string())?,
        stored_id,
    };
    let batch = read(key)?;
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
    let p =
        tdashboard_analysis::projection::project(&manager.acquire().ok_or("No acquired Study")?);
    Ok(AppMessage::StudyChunkLoaded {
        study_id: stored_id,
        has_constraints: p.meta.has_constraints,
        param_names: p.meta.param_names.clone(),
        objective_names: p.meta.objective_names.clone(),
        user_attr_numeric_names: p.dataframe.user_attr_numeric_col_names().to_vec(),
        user_attr_string_names: p.dataframe.user_attr_string_col_names().to_vec(),
        max_constraints: p.dataframe.constraint_col_names().len(),
        meta: crate::io::journal::convert_projection_meta(p.meta),
        new_rows: p.trial_rows,
        is_first: true,
        is_final: true,
    })
}
