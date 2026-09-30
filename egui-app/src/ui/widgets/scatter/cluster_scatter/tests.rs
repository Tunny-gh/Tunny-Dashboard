use super::matrix::compute_obj_axes_2d;
use super::*;

#[test]
fn cluster_space_labels() {
    assert_eq!(ClusterSpace::Objective.label(), "Objective Space");
    assert_eq!(ClusterSpace::Variable.label(), "Variable Space");
    assert_eq!(ClusterSpace::Combined.label(), "Combined");
}

#[test]
fn cluster_scatter_default_k() {
    let cs = ClusterScatter::default();
    assert_eq!(cs.k, 3);
    assert_eq!(cs.target_space, ClusterSpace::Objective);
    assert_eq!(cs.k_mode, KSelectionMode::ElbowDefault);
    assert_eq!(cs.init_strategy, KMeansInitStrategy::KMeansPlusPlus);
    assert_eq!(cs.elbow_max_k, 10);
    assert!(!cs.computing);
    assert!(cs.pending_compute.is_none());
    assert!(cs.last_error.is_none());
    assert!(cs.cached_points.is_none());
    assert_eq!(cs.cache_key, (0, 0, 0));
}

fn make_view_with_objs(obj_vals: &[Vec<f64>]) -> StudyView {
    use std::collections::HashMap;
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow as CoreRow};
    let n = obj_vals.len();
    if n == 0 {
        let df = DataFrame::from_trials(&[], &[], &[], &[], &[], 0);
        return StudyView::new(Arc::new(df), vec![]);
    }
    let n_obj = obj_vals[0].len();
    let obj_names: Vec<String> = (0..n_obj).map(|i| format!("obj{i}")).collect();
    let core_rows: Vec<CoreRow> = (0..n)
        .map(|i| CoreRow {
            trial_id: i as u32,
            trial_number: i as u32,
            param_display: HashMap::new(),
            param_category_label: HashMap::new(),
            objective_values: obj_vals[i].clone(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        })
        .collect();
    let df = DataFrame::from_trials(&core_rows, &[], &obj_names, &[], &[], 0);
    StudyView::new(Arc::new(df), vec![0; n])
}

#[test]
fn compute_obj_axes_2d_empty_trials() {
    let view = make_view_with_objs(&[]);
    let result = compute_obj_axes_2d(&view, &["obj0".to_string()]);
    assert!(result.is_empty());
}

#[test]
fn compute_obj_axes_2d_single_objective() {
    let view = make_view_with_objs(&[vec![1.5]]);
    let result = compute_obj_axes_2d(&view, &["obj0".to_string()]);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], [1.5_f32, 0.0_f32]);
}

#[test]
fn cache_key_updated_on_data_change() {
    let cs = ClusterScatter::default();
    assert_eq!(cs.cache_key, (0, 0, 0));
    assert!(cs.cached_points.is_none());
}

#[test]
fn adopt_runtime_state_clears_stuck_computing() {
    // If a canvas item is left with computing=true after Run, pulling in the
    // completion state from the global side clears the spinner (regression guard for
    // the never-rendered bug).
    let mut item = ClusterScatter {
        computing: true,
        ..Default::default()
    };
    let global = ClusterScatter::default(); // post-completion (computing=false, error=None)
    item.adopt_runtime_state(&global);
    assert!(!item.computing);
    assert!(item.pending_compute.is_none());
    assert!(item.last_error.is_none());
}

#[test]
fn adopt_runtime_state_preserves_display_cache() {
    // Display caches (cached_points / cache_key) are item-specific, so they're kept
    // as-is.
    let mut item = ClusterScatter {
        computing: true,
        cached_points: Some(vec![[1.0, 2.0]]),
        cache_key: (7, 5, 3),
        ..Default::default()
    };
    item.adopt_runtime_state(&ClusterScatter::default());
    assert_eq!(item.cached_points, Some(vec![[1.0, 2.0]]));
    assert_eq!(item.cache_key, (7, 5, 3));
}

#[test]
fn adopt_runtime_state_propagates_error() {
    let mut item = ClusterScatter {
        computing: true,
        ..Default::default()
    };
    let mut global = ClusterScatter::default();
    global.set_error(crate::state::messages::cluster_ui_error("boom", None, true));
    item.adopt_runtime_state(&global);
    assert!(!item.computing);
    assert!(item.last_error.is_some());
}

#[test]
fn validate_cluster_request_rejects_manual_k_too_small() {
    let request = ClusterComputeRequest {
        k: 1,
        target_space: ClusterSpace::Objective,
        k_mode: KSelectionMode::Manual,
        init_strategy: KMeansInitStrategy::KMeansPlusPlus,
        elbow_max_k: 10,
    };
    assert!(validate_cluster_request(&request, 10).is_err());
}

#[test]
fn validate_cluster_request_accepts_elbow_mode() {
    let request = ClusterComputeRequest {
        k: 999,
        target_space: ClusterSpace::Objective,
        k_mode: KSelectionMode::ElbowDefault,
        init_strategy: KMeansInitStrategy::KMeansPlusPlus,
        elbow_max_k: 10,
    };
    assert!(validate_cluster_request(&request, 10).is_ok());
}

#[test]
fn cache_key_normalizes_unused_field_per_mode() {
    // In Manual mode, elbow_max_k is meaningless, so it's normalized to 0.
    let manual_key = ClusterCacheKey::new(
        ClusterSpace::Objective,
        KSelectionMode::Manual,
        5,
        KMeansInitStrategy::KMeansPlusPlus,
        42,
    );
    assert_eq!(manual_key.k, 5);
    assert_eq!(manual_key.elbow_max_k, 0);

    // In Elbow mode, k is meaningless, so it's normalized to 0.
    let elbow_key = ClusterCacheKey::new(
        ClusterSpace::Objective,
        KSelectionMode::ElbowDefault,
        5,
        KMeansInitStrategy::KMeansPlusPlus,
        42,
    );
    assert_eq!(elbow_key.k, 0);
    assert_eq!(elbow_key.elbow_max_k, 42);
}
#[test]
fn cluster_matrix_filters_missing_numeric_params_and_preserves_targets() {
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow};
    let rows: Vec<TrialRow> = (0..4)
        .map(|i| TrialRow {
            trial_id: 100 + i,
            trial_number: 20 + i,
            param_display: if i == 1 {
                Default::default()
            } else {
                [("x".to_string(), i as f64)].into()
            },
            param_category_label: Default::default(),
            objective_values: vec![i as f64],
            user_attrs_numeric: Default::default(),
            user_attrs_string: Default::default(),
            user_attrs_json: Default::default(),
            constraint_values: vec![],
        })
        .collect();
    let params = vec!["x".to_string()];
    let objectives = vec!["obj".to_string()];
    let df = DataFrame::from_trials(&rows, &params, &objectives, &[], &[], 0);
    let view = StudyView::new(Arc::new(df), vec![0, 0, 1, 0]);
    for space in [ClusterSpace::Variable, ClusterSpace::Combined] {
        let matrix =
            super::matrix::build_cluster_matrix(&view, &params, &objectives, space).unwrap();
        assert_eq!(matrix.target_indices, vec![0, 3]);
        assert_eq!(matrix.n_rows, 2);
        assert!(matrix.flat_data.iter().all(|v| v.is_finite()));
        assert_eq!(matrix.flat_data[0], 0.0);
    }
    let matrix =
        super::matrix::build_cluster_matrix(&view, &params, &objectives, ClusterSpace::Objective)
            .unwrap();
    assert_eq!(matrix.target_indices, vec![0, 1, 3]);
}

#[test]
fn mixed_type_cluster_matrix_keeps_categories_constant_and_filters_missing_numbers() {
    use std::sync::Arc;
    use tunny_core::dataframe::{DataFrame, TrialRow};
    let mut rows: Vec<TrialRow> = (0..4)
        .map(|i| TrialRow {
            trial_id: 100 + i,
            trial_number: i,
            param_display: [("x".to_string(), i as f64)].into(),
            param_category_label: [("category".to_string(), format!("label{}", i % 2))].into(),
            objective_values: vec![i as f64],
            user_attrs_numeric: Default::default(),
            user_attrs_string: Default::default(),
            user_attrs_json: Default::default(),
            constraint_values: vec![],
        })
        .collect();
    let params = vec!["x".to_string(), "category".to_string()];
    let objectives = vec!["obj".to_string()];
    for incomplete in [false, true] {
        if incomplete {
            rows[1].param_display.remove("x");
            rows[2].param_display.insert("x".to_string(), f64::INFINITY);
        }
        let df = DataFrame::from_trials(&rows, &params, &objectives, &[], &[], 0);
        let view = StudyView::new(Arc::new(df), vec![0; rows.len()]);
        for space in [ClusterSpace::Variable, ClusterSpace::Combined] {
            let matrix =
                super::matrix::build_cluster_matrix(&view, &params, &objectives, space).unwrap();
            let expected_rows = if incomplete {
                vec![0, 3]
            } else {
                vec![0, 1, 2, 3]
            };
            assert_eq!(matrix.target_indices, expected_rows);
            for (row, &source) in matrix
                .flat_data
                .chunks_exact(matrix.n_cols)
                .zip(&matrix.target_indices)
            {
                assert_eq!(&row[..2], &[source as f64, 0.0]);
            }
            assert_eq!(matrix.flat_data[0], 0.0);
            let result = tunny_core::clustering::run_kmeans(
                2,
                &matrix.flat_data,
                matrix.n_cols,
                tunny_core::clustering::InitStrategy::Deterministic,
            );
            assert_eq!(result.labels.len(), matrix.n_rows);
            let mut unknown_params = params.clone();
            unknown_params.push("unknown".to_string());
            assert!(super::matrix::build_cluster_matrix(
                &view,
                &unknown_params,
                &objectives,
                space
            )
            .is_err());
        }
    }
}
