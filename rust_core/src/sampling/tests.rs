use super::*;
use crate::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use std::collections::{HashMap, HashSet};

fn make_row(trial_id: u32, obj_values: Vec<f64>) -> TrialRow {
    TrialRow {
        trial_id,
        param_display: HashMap::new(),
        param_category_label: HashMap::new(),
        objective_values: obj_values,
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![],
    }
}

/// Build a DataFrame with `n` rows (single objective = row index).
/// Pareto Rank 1 = row 0 (smallest value under minimisation).
fn setup_single_obj(n: usize) {
    let rows: Vec<TrialRow> = (0..n).map(|i| make_row(i as u32, vec![i as f64])).collect();
    let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    init_sampling(vec![true], vec![0], vec![]);
}

#[test]
fn tc1656_01_small_df_returns_all() {
    setup_single_obj(100);
    let result = downsample_smart(1_000, true).expect("should return Some");
    assert_eq!(result.indices.len(), 100, "expected all 100 rows");
    assert_eq!(result.total_count, 100);
}

#[test]
fn tc1656_02_pareto_always_included() {
    let n = 200usize;
    let rows: Vec<TrialRow> = (0..n)
        .map(|i| make_row(i as u32, vec![i as f64, i as f64]))
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &[],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    let pareto = crate::pareto::compute_pareto_ranks(&[true, true]).pareto_indices;
    init_sampling(vec![true, true], pareto, vec![]);

    let result = downsample_smart(100, true).expect("should return Some");

    assert!(
        result.indices.contains(&0),
        "Pareto Rank 1 point (index 0) must be included"
    );
    assert_eq!(result.indices.len(), 100);
    assert!(result.pareto_count >= 1);
}

#[test]
fn tc1656_03_pareto_exceeds_max_points() {
    let pareto_size = 200usize;
    let n = 500usize;
    let rows: Vec<TrialRow> = (0..n)
        .map(|i| {
            if i < pareto_size {
                make_row(i as u32, vec![(pareto_size - i) as f64, i as f64])
            } else {
                make_row(
                    i as u32,
                    vec![pareto_size as f64 + 1.0, pareto_size as f64 + 1.0],
                )
            }
        })
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &[],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    let pareto = crate::pareto::compute_pareto_ranks(&[true, true]).pareto_indices;
    init_sampling(vec![true, true], pareto, vec![]);

    let result = downsample_smart(100, true).expect("should return Some");
    assert_eq!(result.indices.len(), 100, "must be capped at max_points");
}

#[test]
fn tc1656_04_performance_50k() {
    let n = 50_000usize;
    let rows: Vec<TrialRow> = (0..n)
        .map(|i| make_row(i as u32, vec![i as f64, i as f64]))
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &[],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    init_sampling(vec![true, true], vec![0u32], vec![]);

    let result = downsample_smart(10_000, true).expect("should return Some");
    assert_eq!(result.total_count, 50_000);
    assert_eq!(result.indices.len(), 10_000);
    #[cfg(debug_assertions)]
    let threshold_ms = 200.0_f64;
    #[cfg(not(debug_assertions))]
    let threshold_ms = 5.0_f64;

    assert!(
        result.duration_ms < threshold_ms,
        "expected < {threshold_ms}ms, got {:.2}ms",
        result.duration_ms
    );
}

#[test]
fn tc1657_01_pareto_capped_at_50_percent() {
    let pareto_size = 400usize;
    let total = 10_000usize;
    let rows: Vec<TrialRow> = (0..total)
        .map(|i| {
            if i < pareto_size {
                make_row(i as u32, vec![(pareto_size - i) as f64, i as f64])
            } else {
                make_row(
                    i as u32,
                    vec![pareto_size as f64 + 1.0, pareto_size as f64 + 1.0],
                )
            }
        })
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &[],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    let pareto = crate::pareto::compute_pareto_ranks(&[true, true]).pareto_indices;
    init_sampling(vec![true, true], pareto, vec![]);

    let result = downsample_for_thumbnail(500).expect("should return Some");

    assert!(
        result.pareto_count <= 250,
        "pareto_count {} should be <= 250 (50% of 500)",
        result.pareto_count
    );
}

#[test]
fn tc1657_02_total_within_max_points() {
    setup_single_obj(5_000);
    let result = downsample_for_thumbnail(500).expect("should return Some");
    assert!(
        result.indices.len() <= 500,
        "indices.len() = {} must be <= 500",
        result.indices.len()
    );
}

#[test]
fn tc1658_01_rank1_all_included() {
    let rank1_size = 50usize;
    let total = 10_000usize;
    let rows: Vec<TrialRow> = (0..total)
        .map(|i| {
            if i < rank1_size {
                make_row(i as u32, vec![(rank1_size - i) as f64, i as f64])
            } else {
                make_row(
                    i as u32,
                    vec![rank1_size as f64 + 1.0, rank1_size as f64 + 1.0],
                )
            }
        })
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &[],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    let pareto_result = crate::pareto::compute_pareto_ranks(&[true, true]);
    let all_ranks = pareto_result.ranks.clone();
    let pareto_indices = pareto_result.pareto_indices.clone();
    init_sampling(vec![true, true], pareto_indices, all_ranks);

    let result = downsample_stratified_by_rank(5_000, 5).expect("should return Some");

    let result_set: HashSet<u32> = result.indices.iter().copied().collect();
    assert!(result.pareto_count <= result.indices.len());
    assert_eq!(
        result.pareto_count, rank1_size,
        "all rank1 must be included"
    );
    assert!(result.indices.len() >= rank1_size);
    for &idx in &result.indices[..result.pareto_count] {
        assert!(
            result_set.contains(&idx),
            "rank1 index {} not in result",
            idx
        );
    }
}

#[test]
fn tc1658_02_total_within_max_points() {
    setup_single_obj(10_000);
    let pareto_result = crate::pareto::compute_pareto_ranks(&[true]);
    init_sampling(
        vec![true],
        pareto_result.pareto_indices,
        pareto_result.ranks,
    );
    let result = downsample_stratified_by_rank(5_000, 5).expect("should return Some");
    assert!(
        result.indices.len() <= 5_000,
        "indices.len() = {} must be <= 5000",
        result.indices.len()
    );
}

#[test]
fn tc1659_01_equal_sampling_per_cluster() {
    let clusters = 4usize;
    let per_cluster_size = 2_000usize;
    let total = clusters * per_cluster_size;

    let rows: Vec<TrialRow> = (0..total)
        .map(|i| make_row(i as u32, vec![i as f64]))
        .collect();
    let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    init_sampling(vec![true], vec![0], vec![]);

    let labels: Vec<i32> = (0..total).map(|i| (i / per_cluster_size) as i32).collect();
    set_cluster_labels(labels.clone());

    let result = downsample_by_cluster(4_000).expect("should return Some");

    assert_eq!(result.indices.len(), 4_000);

    let mut counts = vec![0usize; clusters];
    for &idx in &result.indices {
        let cluster = labels[idx as usize] as usize;
        counts[cluster] += 1;
    }
    for (c, &cnt) in counts.iter().enumerate() {
        assert_eq!(
            cnt, 1_000,
            "cluster {} expected 1000 points, got {}",
            c, cnt
        );
    }
}

#[test]
fn tc1659_02_fallback_without_labels() {
    setup_single_obj(50_000);
    reset_sampling();
    init_sampling(vec![true], vec![0u32], vec![]);

    let result = downsample_by_cluster(10_000).expect("should return Some");

    assert_eq!(result.indices.len(), 10_000);
    assert_eq!(result.total_count, 50_000);
}
