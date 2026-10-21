//! Temporary entry composition facade, removed in D7. I/O returns domain batches;
//! caller-owned management supplies immutable inputs to analysis. No global installs.
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
pub(crate) fn new_key(stored_id: u32) -> Result<tdashboard_study::StudyKey, String> {
    Ok(tdashboard_study::StudyKey {
        source: tdashboard_study::IdentityAllocator::new()
            .and_then(|mut a| a.source())
            .map_err(|e| e.to_string())?,
        stored_id,
    })
}

pub(crate) fn project_batch(
    key: tdashboard_study::StudyKey,
    batch: tdashboard_study::StudyBatch,
) -> Result<tdashboard_analysis::projection::Projection, String> {
    use tdashboard_study::*;
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
    Ok(tdashboard_analysis::projection::project(
        &manager.acquire().ok_or("No acquired Study")?,
    ))
}

#[cfg(test)]
pub fn project_rows(
    rows: Vec<tdashboard_io::decoded::TrialRow>,
) -> Vec<crate::dataframe::TrialRow> {
    rows.into_iter()
        .map(|r| crate::dataframe::TrialRow {
            distribution_metadata: crate::dataframe::DistributionMetadata {
                categorical: r.distribution_metadata.categorical,
                numeric_discrete: r.distribution_metadata.numeric_discrete,
                categories: r.distribution_metadata.categories,
            },
            trial_id: r.trial_id,
            trial_number: r.trial_number,
            param_display: r.param_display,
            param_category_label: r.param_category_label,
            objective_values: r.objective_values,
            user_attrs_numeric: r.user_attrs_numeric,
            user_attrs_string: r.user_attrs_string,
            user_attrs_json: r.user_attrs_json,
            constraint_values: r.constraint_values,
        })
        .collect()
}
pub fn parse_journal(data: &[u8]) -> Result<ParseResult, String> {
    let start = std::time::Instant::now();
    let studies = scan_study_list(data)?;
    let contents = studies
        .iter()
        .map(|m| parse_single_study(data, m.study_id))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ParseResult {
        studies: contents.iter().map(|(meta, _, _)| meta.clone()).collect(),
        duration_ms: start.elapsed().as_secs_f64() * 1000.0,
        contents,
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
    let key = new_key(id)?;
    let p = project_batch(key, study::read_full(data, key)?)?;
    Ok((io_meta(p.meta), p.dataframe, p.extras))
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
            meta: io_meta(projection.meta),
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

pub fn io_meta(m: tdashboard_analysis::projection::StudyMeta) -> StudyMeta {
    StudyMeta {
        study_id: m.study_id,
        name: m.name,
        directions: m
            .directions
            .into_iter()
            .map(|d| match d {
                tdashboard_study::Direction::Minimize => OptimizationDirection::Minimize,
                tdashboard_study::Direction::Maximize => OptimizationDirection::Maximize,
            })
            .collect(),
        completed_trials: m.completed_trials,
        total_trials: m.total_trials,
        param_names: m.param_names,
        objective_names: m.objective_names,
        user_attr_names: m.user_attr_names,
        has_constraints: m.has_constraints,
        param_bounds: m.param_bounds,
    }
}

pub fn analysis_meta(m: &StudyMeta) -> tdashboard_analysis::projection::StudyMeta {
    tdashboard_analysis::projection::StudyMeta {
        study_id: m.study_id,
        name: m.name.clone(),
        directions: m
            .directions
            .iter()
            .map(|d| match d {
                OptimizationDirection::Minimize => tdashboard_study::Direction::Minimize,
                OptimizationDirection::Maximize => tdashboard_study::Direction::Maximize,
            })
            .collect(),
        completed_trials: m.completed_trials,
        total_trials: m.total_trials,
        param_names: m.param_names.clone(),
        objective_names: m.objective_names.clone(),
        user_attr_names: m.user_attr_names.clone(),
        has_constraints: m.has_constraints,
        param_bounds: m.param_bounds.clone(),
    }
}
