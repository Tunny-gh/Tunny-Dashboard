#![allow(dead_code)]
use tdashboard_analysis::projection::{project, Projection};
use tdashboard_study::*;

pub fn try_journal(data: &[u8], id: u32) -> Result<Projection, String> {
    let key = key(id);
    Ok(project(&acquire(
        key,
        tdashboard_io::journal::parser::study::read_full(data, key)?,
    )))
}

pub fn try_csv(data: &[u8], name: &str) -> Result<Projection, String> {
    let key = key(0);
    let parsed = tdashboard_io::flat_csv::parse_flat_csv(data, name)?;
    Ok(project(&acquire(key, parsed.into_batch(key))))
}

/// Observes each committed input; the table is derived, never authority.
pub fn observe_stream(
    data: &[u8],
    id: u32,
    size: usize,
    mut sink: impl FnMut(StreamObservation),
) -> Result<(), String> {
    let key = key(id);
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut table = tdashboard_analysis::projection::TableProjection::default();
    let mut emitted = std::collections::HashSet::new();
    let mut first = true;
    tdashboard_io::journal::parser::study::read_batches_with_final(
        data,
        key,
        size,
        |batch, last| {
            let upserts = match &batch {
                StudyBatch::Replace { trials, .. } => trials,
                StudyBatch::Delta { upserts, .. } => upserts,
            };
            let corrected = upserts.iter().any(|t| emitted.contains(&t.key.stored_id));
            header = manager.apply(header, batch).unwrap().next_header(ticket);
            if last {
                manager.finish(ticket, header.sequence).unwrap();
            }
            let input = manager.acquire().unwrap();
            let projection = table.project(&input);
            sink(StreamObservation {
                new_rows: projection
                    .trial_rows
                    .into_iter()
                    .filter(|r| emitted.insert(r.trial_id))
                    .collect(),
                param_names: projection.meta.param_names.clone(),
                objective_names: projection.meta.objective_names.clone(),
                user_attr_numeric_names: projection
                    .dataframe
                    .user_attr_numeric_col_names()
                    .to_vec(),
                user_attr_string_names: projection.dataframe.user_attr_string_col_names().to_vec(),
                max_constraints: input.data().constraint_schema().width,
                meta: projection.meta,
                extras: projection.extras,
                updated_existing: corrected,
                dataframe: projection.dataframe,
                is_first: first,
                is_final: last,
            });
            first = false;
            Ok(())
        },
    )
}

pub struct StreamObservation {
    pub meta: tdashboard_analysis::projection::StudyMeta,
    pub extras: tdashboard_analysis::extras::StudyExtras,
    pub dataframe: std::sync::Arc<tdashboard_analysis::dataframe::DataFrame>,
    pub new_rows: Vec<tdashboard_analysis::dataframe::TrialRow>,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_numeric_names: Vec<String>,
    pub user_attr_string_names: Vec<String>,
    pub max_constraints: usize,
    pub is_first: bool,
    pub is_final: bool,
    pub updated_existing: bool,
}

impl StreamObservation {
    pub fn apply_to(self, frame: &mut tdashboard_analysis::dataframe::DataFrame) {
        *frame = std::sync::Arc::unwrap_or_clone(self.dataframe);
    }
}

pub fn key(stored_id: u32) -> StudyKey {
    StudyKey {
        source: IdentityAllocator::new().unwrap().source().unwrap(),
        stored_id,
    }
}

pub fn acquire(key: StudyKey, batch: StudyBatch) -> StudyInput {
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let receipt = manager
        .apply(manager.header(ticket).unwrap(), batch)
        .unwrap();
    manager.finish(ticket, receipt.next_sequence).unwrap();
    manager.acquire().unwrap()
}

pub fn journal(data: &[u8], id: u32) -> Projection {
    let key = key(id);
    project(&acquire(
        key,
        tdashboard_io::journal::parser::study::read_full(data, key).unwrap(),
    ))
}

pub fn csv(data: &[u8], name: &str) -> Projection {
    let key = key(0);
    let parsed = tdashboard_io::flat_csv::parse_flat_csv(data, name).unwrap();
    project(&acquire(key, parsed.into_batch(key)))
}

pub fn streamed(data: &[u8], id: u32, size: usize) -> Projection {
    let key = key(id);
    let mut manager = StudyManager::new(key);
    let ticket = manager.begin(LoadKind::Initial).unwrap();
    let mut header = manager.header(ticket).unwrap();
    let mut table = tdashboard_analysis::projection::TableProjection::default();
    let mut result = None;
    tdashboard_io::journal::parser::study::read_batches_with_final(
        data,
        key,
        size,
        |batch, last| {
            header = manager.apply(header, batch).unwrap().next_header(ticket);
            if last {
                manager.finish(ticket, header.sequence).unwrap();
            }
            result = Some(table.project(&manager.acquire().unwrap()));
            Ok(())
        },
    )
    .unwrap();
    result.unwrap()
}
