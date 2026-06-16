use std::collections::HashSet;

use crate::dataframe::{DataFrame, TrialRow};

use super::builders::TrialBuilder;
use super::state::ParserState;
use super::types::StudyMeta;

pub(super) fn finalize_state(state: ParserState) -> (Vec<StudyMeta>, Vec<DataFrame>) {
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

    for (trial_id, trial) in sorted_trials {
        if trial.state != 1 {
            continue;
        }
        let study_idx = trial.study_id as usize;
        if study_idx >= n_studies {
            continue;
        }

        {
            let study = &mut studies[study_idx];
            study.completed_trials += 1;
            for name in trial.param_display.keys() {
                study.param_names.insert(name.clone());
            }
            for name in trial.user_attrs_numeric.keys() {
                study.user_attr_names.insert(name.clone());
                per_study_unn[study_idx].insert(name.clone());
            }
            for name in trial.user_attrs_string.keys() {
                study.user_attr_names.insert(name.clone());
                per_study_usn[study_idx].insert(name.clone());
            }
            if trial.has_constraints {
                study.has_constraints = true;
            }
            if study.objective_names.is_empty() {
                if let Some(values) = &trial.values {
                    study.objective_names = (0..values.len())
                        .map(|index| format!("obj{index}"))
                        .collect();
                }
            }
        }

        per_study_max_c[study_idx] = per_study_max_c[study_idx].max(trial.constraint_values.len());

        per_study_rows[study_idx].push(TrialRow {
            trial_id,
            param_display: trial.param_display,
            param_category_label: trial.param_category_label,
            objective_values: trial.values.unwrap_or_default(),
            user_attrs_numeric: trial.user_attrs_numeric,
            user_attrs_string: trial.user_attrs_string,
            constraint_values: trial.constraint_values,
        });
    }

    let mut study_metas = Vec::with_capacity(n_studies);
    let mut dataframes = Vec::with_capacity(n_studies);

    for (index, builder) in studies.into_iter().enumerate() {
        let mut param_names: Vec<String> = builder.param_names.into_iter().collect();
        param_names.sort();
        let mut user_attr_names: Vec<String> = builder.user_attr_names.into_iter().collect();
        user_attr_names.sort();
        let objective_names = builder.objective_names;

        study_metas.push(StudyMeta {
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
        });

        let mut unn: Vec<String> = std::mem::take(&mut per_study_unn[index])
            .into_iter()
            .collect();
        unn.sort();
        let mut usn: Vec<String> = std::mem::take(&mut per_study_usn[index])
            .into_iter()
            .collect();
        usn.sort();

        // ピーク削減: 各 study の行 Vec を take して DataFrame 構築後に即解放する。
        // take で所有権を移動させることで、全 study 行が同時メモリ上に残らない。
        let study_rows = std::mem::take(&mut per_study_rows[index]);
        dataframes.push(DataFrame::from_trials(
            &study_rows,
            &param_names,
            &objective_names,
            &unn,
            &usn,
            per_study_max_c[index],
        ));
        // study_rows はここでドロップされ、この study の中間行データが解放される
    }

    (study_metas, dataframes)
}
