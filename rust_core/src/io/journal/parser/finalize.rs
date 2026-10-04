use std::collections::HashSet;

use crate::data::extras::{StudyExtras, TrialExtra, TrialState};
use crate::dataframe::{DataFrame, TrialRow};

use super::builders::TrialBuilder;
use super::state::ParserState;
use super::types::StudyMeta;

/// Finalized data for a single study, as returned by `finalize_state`.
///
/// This used to return three parallel Vecs, `(Vec<StudyMeta>, Vec<DataFrame>, Vec<StudyExtras>)`,
/// but that's now combined into a per-study struct to avoid repeated `nth(pos).unwrap()` calls on the caller side.
pub(super) struct FinalizedStudy {
    pub(super) meta: StudyMeta,
    pub(super) dataframe: DataFrame,
    pub(super) extras: StudyExtras,
}

pub(super) fn finalize_state(state: ParserState) -> Vec<FinalizedStudy> {
    let ParserState {
        mut studies,
        trial_builders,
        ..
    } = state;
    let n_studies = studies.len();

    let mut sorted_trials: Vec<(u32, TrialBuilder)> = trial_builders.into_iter().collect();
    sorted_trials.sort_by_key(|(trial_id, _)| *trial_id);

    let mut per_study_rows: Vec<Vec<TrialRow>> = (0..n_studies).map(|_| Vec::new()).collect();
    let mut per_study_unn: Vec<HashSet<String>> = (0..n_studies).map(|_| HashSet::new()).collect();
    let mut per_study_usn: Vec<HashSet<String>> = (0..n_studies).map(|_| HashSet::new()).collect();
    let mut per_study_max_c: Vec<usize> = vec![0; n_studies];
    // Extra info for all trials (any state). Ordered by ascending trial_id (since sorted_trials is ascending).
    let mut per_study_extras: Vec<Vec<TrialExtra>> = (0..n_studies).map(|_| Vec::new()).collect();

    for (trial_id, mut trial) in sorted_trials {
        let study_idx = trial.study_id as usize;
        if study_idx >= n_studies {
            continue;
        }

        // extras collects all trials regardless of state, independently of the DataFrame (which is COMPLETE-only).
        let mut intermediate_values = std::mem::take(&mut trial.intermediate_values);
        intermediate_values.sort_by_key(|(step, _)| *step);
        per_study_extras[study_idx].push(TrialExtra {
            trial_id,
            trial_number: trial.trial_number,
            state: TrialState::from_journal(trial.state),
            datetime_start: trial.datetime_start,
            datetime_complete: trial.datetime_complete,
            intermediate_values,
        });

        studies[study_idx].has_constraints |=
            trial.has_constraints || !trial.constraint_values.is_empty();
        per_study_max_c[study_idx] = per_study_max_c[study_idx].max(trial.constraint_values.len());
        if trial.state != 1 {
            continue;
        }

        {
            let study = &mut studies[study_idx];
            study.completed_trials += 1;
            for name in trial.param_display.keys() {
                study.param_names.insert(name.clone());
            }
            for name in trial.distribution_metadata.categories.keys() {
                study.param_names.insert(name.clone());
            }
            for name in trial.user_attrs_json.keys() {
                study.user_attr_names.insert(name.clone());
            }
            for name in trial.user_attrs_numeric.keys() {
                per_study_unn[study_idx].insert(name.clone());
            }
            for name in trial.user_attrs_string.keys() {
                per_study_usn[study_idx].insert(name.clone());
            }
            if study.objective_names.is_empty() {
                if let Some(values) = &trial.values {
                    study.objective_names = (0..values.len())
                        .map(|index| format!("obj{index}"))
                        .collect();
                }
            }
        }

        per_study_rows[study_idx].push(TrialRow {
            distribution_metadata: trial.distribution_metadata,
            trial_id,
            trial_number: trial.trial_number,
            param_display: trial.param_display,
            param_category_label: trial.param_category_label,
            objective_values: trial.values.unwrap_or_default(),
            user_attrs_numeric: trial.user_attrs_numeric,
            user_attrs_string: trial.user_attrs_string,
            user_attrs_json: trial.user_attrs_json,
            constraint_values: trial.constraint_values,
        });
    }

    let mut finalized: Vec<FinalizedStudy> = Vec::with_capacity(n_studies);

    for (index, builder) in studies.into_iter().enumerate() {
        let mut param_names: Vec<String> = builder.param_names.into_iter().collect();
        param_names.sort();
        let mut user_attr_names: Vec<String> = builder.user_attr_names.into_iter().collect();
        user_attr_names.sort();
        let objective_names = builder.objective_names;

        let meta = StudyMeta {
            study_id: builder.study_id,
            name: builder.name,
            directions: builder.directions,
            completed_trials: builder.completed_trials,
            total_trials: builder.total_trials,
            param_names: param_names.clone(),
            objective_names: objective_names.clone(),
            user_attr_names,
            has_constraints: builder.has_constraints,
            param_bounds: builder.param_bounds,
        };

        let mut unn: Vec<String> = std::mem::take(&mut per_study_unn[index])
            .into_iter()
            .collect();
        unn.sort();
        let mut usn: Vec<String> = std::mem::take(&mut per_study_usn[index])
            .into_iter()
            .collect();
        usn.sort();

        // Peak memory reduction: take each study's row Vec and free it right after building the DataFrame.
        // Moving ownership via take means row data for all studies never coexists in memory at once.
        let study_rows = std::mem::take(&mut per_study_rows[index]);
        let mut dataframe = DataFrame::from_trials(
            &study_rows,
            &param_names,
            &objective_names,
            &unn,
            &usn,
            per_study_max_c[index],
        );
        if meta.has_constraints {
            dataframe.mark_constrained();
        }
        // study_rows is dropped here, freeing this study's intermediate row data

        finalized.push(FinalizedStudy {
            meta,
            dataframe,
            extras: StudyExtras {
                trials: std::mem::take(&mut per_study_extras[index]),
            },
        });
    }

    finalized
}
