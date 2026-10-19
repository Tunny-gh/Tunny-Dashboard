//! Temporary legacy projection facade. D4 replaces projections; D7 removes
//! remaining bulk/export entry imports. No parser installs process-global data.
pub use tdashboard_io::journal::parser::study;
pub use tdashboard_io::journal::parser::{scan_study_list, OptimizationDirection, StudyMeta};
pub mod study_projection;
#[cfg(test)]
use tdashboard_io::journal::parser::distribution::Distribution;

pub struct ParseResult {
    pub studies: Vec<StudyMeta>,
    pub contents: Vec<(
        StudyMeta,
        crate::dataframe::DataFrame,
        crate::data::extras::StudyExtras,
    )>,
    pub duration_ms: f64,
}
pub(crate) fn project_decoded(
    table: tdashboard_io::decoded::DecodedTable,
) -> crate::dataframe::DataFrame {
    let mut frame = crate::dataframe::DataFrame::from_trials(
        &table.rows,
        &table.param_names,
        &table.objective_names,
        &table.numeric_attrs,
        &table.string_attrs,
        table.constraint_width,
    );
    if table.constrained {
        frame.mark_constrained();
    }
    frame
}
pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
    let result = tdashboard_io::journal::parser::parse_journal(data)?;
    Ok(ParseResult {
        studies: result.studies,
        duration_ms: result.duration_ms,
        contents: result
            .contents
            .into_iter()
            .map(|(m, t, e)| (m, project_decoded(t), e))
            .collect(),
    })
}
pub fn parse_single_study(
    data: &[u8],
    id: u32,
) -> Result<
    (
        StudyMeta,
        crate::dataframe::DataFrame,
        crate::data::extras::StudyExtras,
    ),
    String,
> {
    let (meta, table, extras) = tdashboard_io::journal::parser::parse_single_study(data, id)?;
    Ok((meta, project_decoded(table), extras))
}

pub struct StudyStreamBatch {
    pub extras: crate::data::extras::StudyExtras,
    pub meta: StudyMeta,
    pub new_rows: Vec<crate::dataframe::TrialRow>,
    pub param_names: Vec<String>,
    pub objective_names: Vec<String>,
    pub user_attr_numeric_names: Vec<String>,
    pub user_attr_string_names: Vec<String>,
    pub max_constraints: usize,
    pub is_first: bool,
    pub is_final: bool,
    pub replacement: Option<crate::dataframe::DataFrame>,
}
impl StudyStreamBatch {
    pub fn apply_to(self, frame: &mut crate::dataframe::DataFrame) {
        if let Some(replacement) = self.replacement {
            *frame = replacement;
        } else {
            frame.append_trials(
                &self.new_rows,
                &self.param_names,
                &self.objective_names,
                &self.user_attr_numeric_names,
                &self.user_attr_string_names,
                self.max_constraints,
            );
            if self.meta.has_constraints {
                frame.mark_constrained();
            }
        }
    }
}
/// D4 compatibility projection over owned, fallible I/O replay. Live GUI delivery
/// uses I/O batches directly, not this legacy callback interface.
pub fn parse_single_study_streaming(
    data: &[u8],
    id: u32,
    size: usize,
    mut sink: impl FnMut(StudyStreamBatch),
) -> Result<(), String> {
    use tdashboard_study::*;
    let key = StudyKey {
        source: IdentityAllocator::new()
            .and_then(|mut a| a.source())
            .map_err(|e| e.to_string())?,
        stored_id: id,
    };
    let mut manager = StudyManager::new(key);
    let ticket = manager
        .begin(LoadKind::Initial)
        .map_err(|e| e.to_string())?;
    let mut header = manager.header(ticket).map_err(|e| e.to_string())?;
    let mut table = study_projection::TableProjection::default();
    let mut first = true;
    let mut emitted = std::collections::HashSet::new();
    let mut width = 0;
    study::read_batches_with_final(data, key, size, |batch, final_batch| {
        let updates = match &batch {
            StudyBatch::Replace { trials, .. } => trials,
            StudyBatch::Delta { upserts, .. } => upserts,
        };
        let corrected = updates.iter().any(|t| emitted.contains(&t.key.stored_id));
        header = manager
            .apply(header, batch)
            .map_err(|e| e.to_string())?
            .next_header(ticket);
        if final_batch {
            manager
                .finish(ticket, header.sequence)
                .map_err(|e| e.to_string())?;
        }
        let input = manager.acquire().ok_or("No acquired Journal input")?;
        let mut projection = table.project(&input);
        let new_rows = projection
            .trial_rows
            .drain(..)
            .filter(|r| emitted.insert(r.trial_id))
            .collect();
        let new_width = input.data().constraint_schema().width;
        let mut numeric = projection.dataframe.user_attr_numeric_col_names().to_vec();
        numeric.sort();
        let mut text = projection.dataframe.user_attr_string_col_names().to_vec();
        text.sort();
        let replacement = (corrected || new_width < width).then_some(projection.dataframe);
        width = new_width;
        sink(StudyStreamBatch {
            extras: projection.extras,
            param_names: projection.meta.param_names.clone(),
            objective_names: projection.meta.objective_names.clone(),
            user_attr_numeric_names: numeric,
            user_attr_string_names: text,
            meta: projection.meta,
            new_rows,
            max_constraints: width,
            is_first: first,
            is_final: final_batch,
            replacement,
        });
        first = false;
        Ok(())
    })
}
#[cfg(test)]
mod tests;
