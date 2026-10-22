use crate::state::app_state::{Direction, StudyMeta};
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

/// Runs select_study in the background and returns an AppMessage.
///
/// Does not duplicate the row-oriented `Vec<TrialRow>`; only sends `study_id` and the
/// Pareto rank (MEM-001). On receiving `StudySelected`, the UI side fetches the
/// `Arc<DataFrame>` from the shared store and builds the `StudyView`.
pub fn select_study_task(meta: StudyMeta) -> AppMessage {
    let is_minimize: Vec<bool> = meta
        .directions
        .iter()
        .map(|d| matches!(d, Direction::Minimize))
        .collect();

    let study_id = meta.study_id;
    match tunny_core::dataframe::select_study(study_id) {
        Ok(()) => {
            let df = tunny_core::dataframe::snapshot(study_id).unwrap();
            let pareto = tunny_core::pareto::compute_pareto_ranks(&df, &is_minimize);
            let pareto_indices = pareto.pareto_indices;
            AppMessage::StudySelected {
                meta,
                study_id,
                pareto_rank: pareto.ranks,
                pareto_indices,
            }
        }
        Err(e) => AppMessage::Error(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::Direction;

    fn make_study(id: u32, name: &str, completed: usize) -> StudyMeta {
        StudyMeta {
            study_id: id,
            name: name.to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: completed,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        }
    }

    #[test]
    fn select_study_task_invalid_id_returns_error() {
        let meta = make_study(999, "nonexistent", 0);
        let msg = select_study_task(meta);
        match msg {
            AppMessage::Error(_) => {}
            _ => panic!("Expected Error for nonexistent study"),
        }
    }
}
