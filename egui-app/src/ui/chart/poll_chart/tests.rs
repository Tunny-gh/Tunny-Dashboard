use super::*;
use crate::state::app_state::{StudyContext, StudyMeta, StudyView};
use crate::state::results::ConvergenceHistory;
use std::sync::Arc;
use std::time::Duration;
use tunny_core::dataframe::{DataFrame, TrialRow};
use tunny_core::indicators::{compute_indicator_histories, MoIndicator, SeriesInput};

fn study(id: u32, constraints: Vec<Vec<f64>>, constrained: bool) -> StudyContext {
    let objective_names = vec!["x".into(), "y".into()];
    let rows: Vec<TrialRow> = constraints
        .into_iter()
        .enumerate()
        .map(|(i, constraint_values)| TrialRow {
            trial_id: id * 1000 + 11 + i as u32 * 7,
            trial_number: i as u32 * 3,
            objective_values: vec![i as f64 + 1.0, 200.0 - i as f64],
            constraint_values,
            param_display: Default::default(),
            distribution_metadata: Default::default(),
            param_category_label: Default::default(),
            user_attrs_numeric: Default::default(),
            user_attrs_string: Default::default(),
            user_attrs_json: Default::default(),
        })
        .collect();
    let mut df = DataFrame::from_trials(&rows, &[], &objective_names, &[], &[], 0);
    if constrained {
        df.mark_constrained();
    }
    StudyContext {
        meta: StudyMeta {
            study_id: id,
            name: format!("study {id}"),
            directions: vec![Direction::Minimize; 2],
            completed_trials: rows.len(),
            param_names: vec![],
            objective_names,
            param_bounds: Default::default(),
        },
        view: StudyView::new(Arc::new(df), vec![]),
        pareto_indices: vec![],
    }
}

#[test]
fn mcdm_dispatch_propagates_negative_weight_failure_before_empty_front() {
    use crate::state::messages::McdmChartSource;
    use crate::state::results::McdmMethod;
    use crate::ui::widgets::mcdm_chart::{McdmComputeRequest, McdmControls};

    let ctx = study(1, vec![], false);
    for &method in McdmMethod::all() {
        for weights in [[2.0, -1.0], [-1.0, -1.0], [f64::NAN, f64::NEG_INFINITY]] {
            let mut controls = McdmControls {
                pending_compute: Some(McdmComputeRequest {
                    method,
                    weights: weights.to_vec(),
                    v: 0.5,
                }),
                ..Default::default()
            };
            let (tx, rx) = mpsc::sync_channel(1);
            mcdm::dispatch_mcdm_compute(
                &mut controls,
                &ctx,
                &ctx.meta.objective_names,
                &ctx.meta.directions,
                McdmChartSource::Rank,
                &tx,
            );
            assert!(controls.pending_compute.is_none());
            match rx.recv_timeout(Duration::from_secs(30)).unwrap() {
                AppMessage::McdmFailed { message, .. } => {
                    assert_eq!(message, "Criterion weights must be nonnegative")
                }
                _ => panic!("Negative weights must fail, never produce a cacheable result"),
            }
        }
    }
}

fn poll(
    base: &StudyContext,
    comparison: &StudyContext,
    indicator: MoIndicator,
) -> (ConvergenceHistory, ConvergenceHistory) {
    let mut state = AppState::new();
    state.current_study = Some(base.clone());
    state.comparison_studies = vec![comparison.clone()];
    state.convergence_indicator = indicator;
    let mut widgets = WidgetStates::default();
    let (tx, rx) = mpsc::sync_channel(1);
    poll_convergence_indicators(&state, &mut widgets, &tx);
    assert!(widgets.convergence.computing);
    match rx.recv_timeout(Duration::from_secs(30)).unwrap() {
        AppMessage::IndicatorHistoryDone {
            indicator: actual_indicator,
            base,
            mut comparisons,
            ..
        } => {
            assert_eq!(actual_indicator, indicator);
            assert_eq!(comparisons.len(), 1);
            (base, comparisons.remove(0))
        }
        _ => panic!("Expected indicator histories"),
    }
}

// Build the oracle from explicitly verified rows, retaining all original slots.
fn assert_histories(
    base: &StudyContext,
    comparison: &StudyContext,
    base_rows: &[usize],
    comparison_rows: &[usize],
) {
    let selected = |study: &StudyContext, verified_rows: &[usize]| {
        let step = (study.trial_count() / 50).max(1);
        let rows: Vec<usize> = (0..study.trial_count()).step_by(step).collect();
        let ids: Vec<u32> = rows.iter().map(|&i| study.view.trial_ids[i]).collect();
        let objectives: Vec<Vec<f64>> = rows
            .iter()
            .map(|&i| {
                if !verified_rows.contains(&i) {
                    return vec![f64::NAN; 2];
                }
                study
                    .meta
                    .objective_names
                    .iter()
                    .map(|name| study.view.numeric_column(name).unwrap()[i])
                    .collect()
            })
            .collect();
        (ids, objectives)
    };
    let (base_ids, base_objs) = selected(base, base_rows);
    let (comp_ids, comp_objs) = selected(comparison, comparison_rows);
    for indicator in MoIndicator::all() {
        let expected = compute_indicator_histories(
            &[
                SeriesInput {
                    trial_ids: &base_ids,
                    objectives: &base_objs,
                },
                SeriesInput {
                    trial_ids: &comp_ids,
                    objectives: &comp_objs,
                },
            ],
            &[true, true],
            indicator,
            None,
        );
        let (actual_base, actual_comp) = poll(base, comparison, indicator);
        for (actual, expected) in [&actual_base, &actual_comp].iter().zip(&expected) {
            assert_eq!(actual.trial_ids, expected.trial_ids);
            assert_eq!(actual.values, expected.values);
        }
        assert_eq!(actual_base.ref_point, expected[0].ref_point);
        assert_eq!(actual_base.sample_step, (base.trial_count() / 50).max(1));
        assert_eq!(
            actual_comp.sample_step,
            (comparison.trial_count() / 50).max(1)
        );
    }
}

#[test]
fn convergence_mixed_constraints_exclude_unverified_and_violations_preserving_trial_axis() {
    let base = study(
        1,
        (0..120)
            .map(|i| match i % 6 {
                0 => vec![],
                1 | 2 => vec![-1.0, 0.0],
                3 => vec![-1.0],
                4 => vec![0.1, f64::NAN],
                _ => vec![f64::INFINITY, -1.0],
            })
            .collect(),
        true,
    );
    let comparison = study(
        2,
        vec![
            vec![f64::NAN, -1.0],
            vec![-1.0, 0.0],
            vec![0.1],
            vec![-1.0],
            vec![0.0, -2.0],
            vec![f64::NEG_INFINITY, 0.0],
        ],
        true,
    );
    // Keep original row sampling (step 2), rather than resampling compacted rows.
    let base_rows: Vec<usize> = (2..120).step_by(6).collect();
    assert_histories(&base, &comparison, &base_rows, &[1, 4]);
    let (base_history, comparison_history) = poll(&base, &comparison, MoIndicator::Hypervolume);
    assert_eq!(base_history.trial_ids[0], 1011);
    assert_eq!(base_history.trial_ids[1], 1025);
    assert_eq!(base_history.trial_ids[4], 1067);
    assert_eq!(comparison_history.trial_ids[1], 2018);
    assert_eq!(comparison_history.trial_ids[4], 2039);
    // The renderer uses sample index * step, not trial_id. Retained slots keep
    // verified base rows 2 and 8 at x=2 and x=8, and comparison row 4 at x=4.
    assert_eq!(base_history.values.len(), 60);
    assert_eq!(comparison_history.values.len(), 6);
    assert_eq!(base_history.sample_step, 2);
    assert_eq!(comparison_history.sample_step, 1);
    assert_eq!(base_history.values[0], 0.0);
    assert_eq!(base_history.values[2], base_history.values[1]);
    assert_eq!(base_history.values[3], base_history.values[1]);
    assert_eq!(comparison_history.values[2], comparison_history.values[1]);
    assert_eq!(comparison_history.values[3], comparison_history.values[1]);
}

#[test]
fn convergence_attribute_only_and_all_unverified_have_empty_verified_front() {
    let attribute_only = study(1, vec![vec![]; 3], true);
    let all_unverified = study(2, vec![vec![], vec![-1.0], vec![f64::NAN, 0.0]], true);
    assert!(attribute_only.view.feasibility().has_constraints());
    assert!(all_unverified.view.feasibility().has_constraints());
    assert_histories(&attribute_only, &all_unverified, &[], &[]);
    let (base_history, comparison_history) =
        poll(&attribute_only, &all_unverified, MoIndicator::Hypervolume);
    assert!(base_history.values.is_empty());
    assert!(comparison_history.values.is_empty());
    assert!(base_history.ref_point.is_empty());
    let verified = study(3, vec![vec![0.0]; 2], true);
    // An empty verified series must not contaminate the other study's shared set.
    assert_histories(&attribute_only, &verified, &[], &[0, 1]);
    assert_histories(&verified, &all_unverified, &[0, 1], &[]);
}

#[test]
fn convergence_genuinely_unconstrained_sampling_is_unchanged() {
    let base = study(1, vec![vec![]; 120], false);
    let comparison = study(2, vec![vec![]; 3], false);
    assert!(!base.view.feasibility().has_constraints());
    assert!(!comparison.view.feasibility().has_constraints());
    let base_rows: Vec<usize> = (0..120).step_by(2).collect();
    assert_histories(&base, &comparison, &base_rows, &[0, 1, 2]);
}
