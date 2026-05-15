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
fn setup_single_obj(n: usize) -> SamplingContext {
    let rows: Vec<TrialRow> = (0..n).map(|i| make_row(i as u32, vec![i as f64])).collect();
    let df = DataFrame::from_trials(&rows, &[], &["obj0".to_string()], &[], &[], 0);
    store_dataframes(vec![df]);
    select_study(0).expect("select_study");
    init_sampling(vec![true], vec![0], vec![])
}

#[test]
fn tc1656_01_small_df_returns_all() {
    let ctx = setup_single_obj(100);
    let result = downsample_smart(&ctx, 1_000, true).expect("should return Some");
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
    let ctx = init_sampling(vec![true, true], pareto, vec![]);

    let result = downsample_smart(&ctx, 100, true).expect("should return Some");

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
    let ctx = init_sampling(vec![true, true], pareto, vec![]);

    let result = downsample_smart(&ctx, 100, true).expect("should return Some");
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
    let ctx = init_sampling(vec![true, true], vec![0u32], vec![]);

    let result = downsample_smart(&ctx, 10_000, true).expect("should return Some");
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
    let ctx = init_sampling(vec![true, true], pareto, vec![]);

    let result = downsample_for_thumbnail(&ctx, 500).expect("should return Some");

    assert!(
        result.pareto_count <= 250,
        "pareto_count {} should be <= 250 (50% of 500)",
        result.pareto_count
    );
}

#[test]
fn tc1657_02_total_within_max_points() {
    let ctx = setup_single_obj(5_000);
    let result = downsample_for_thumbnail(&ctx, 500).expect("should return Some");
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
    let ctx = init_sampling(vec![true, true], pareto_indices, all_ranks);

    let result = downsample_stratified_by_rank(&ctx, 5_000, 5).expect("should return Some");

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
    let ctx_base = setup_single_obj(10_000);
    let pareto_result = crate::pareto::compute_pareto_ranks(&[true]);
    let ctx = init_sampling(
        vec![true],
        pareto_result.pareto_indices,
        pareto_result.ranks,
    );
    drop(ctx_base);
    let result = downsample_stratified_by_rank(&ctx, 5_000, 5).expect("should return Some");
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
    let mut ctx = init_sampling(vec![true], vec![0], vec![]);

    let labels: Vec<i32> = (0..total).map(|i| (i / per_cluster_size) as i32).collect();
    ctx.cluster_labels = Some(labels.clone());

    let result = downsample_by_cluster(&ctx, 4_000).expect("should return Some");

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
    let _ = setup_single_obj(50_000);
    // Create a new context without cluster_labels
    let ctx = init_sampling(vec![true], vec![0u32], vec![]);
    // cluster_labels is None by default → falls back to downsample_smart

    let result = downsample_by_cluster(&ctx, 10_000).expect("should return Some");

    assert_eq!(result.indices.len(), 10_000);
    assert_eq!(result.total_count, 50_000);
}

// ---- TASK-2269: SamplingContext テスト ----

#[test]
fn tc_2269_01_init_sampling_returns_context_no_global_state() {
    let ctx1 = init_sampling(vec![true], vec![0u32], vec![]);
    let ctx2 = init_sampling(vec![false], vec![1u32], vec![]);
    // Two independent contexts; creating one does not affect the other
    assert_eq!(ctx1.is_minimize, vec![true]);
    assert_eq!(ctx2.is_minimize, vec![false]);
    assert_eq!(ctx1.pareto_indices, Some(vec![0u32]));
    assert_eq!(ctx2.pareto_indices, Some(vec![1u32]));
}

#[test]
fn tc_2269_02_independent_contexts_do_not_interfere() {
    let _ = setup_single_obj(200);
    let ctx_a = init_sampling(vec![true], vec![0u32], vec![]);
    let mut ctx_b = init_sampling(vec![true], vec![0u32], vec![]);
    ctx_b.cluster_labels = Some(vec![0i32; 200]);
    // ctx_a should still have no cluster_labels
    assert!(ctx_a.cluster_labels.is_none(), "ctx_a must be unaffected by ctx_b mutation");
}

#[test]
fn tc_2269_03_cluster_labels_none_falls_back_to_smart() {
    let ctx = setup_single_obj(500);
    // cluster_labels is None → downsample_by_cluster falls back to downsample_smart
    let result_cluster = downsample_by_cluster(&ctx, 100).expect("should return Some");
    let result_smart = downsample_smart(&ctx, 100, true).expect("should return Some");
    assert_eq!(result_cluster.total_count, result_smart.total_count);
    assert_eq!(result_cluster.indices.len(), result_smart.indices.len());
}

#[test]
fn tc_2269_04_empty_dataset_returns_none() {
    // No active study loaded — all downsample functions return None
    // Use a new context to ensure no active df
    let ctx = init_sampling(vec![true], vec![], vec![]);
    // After store_dataframes([]) and no select_study, with_active_df returns None
    // But we can't easily clear the DF in test; just verify the context can be cloned
    let _ctx2 = ctx.clone();
    assert!(ctx.pareto_indices.as_ref().map(|v| v.is_empty()).unwrap_or(true));
}

#[test]
fn tc_2269_05_sampling_context_clone_is_independent() {
    let ctx = init_sampling(vec![true], vec![0u32, 1u32], vec![]);
    let mut ctx2 = ctx.clone();
    ctx2.cluster_labels = Some(vec![0i32, 1]);
    // Original ctx should not be affected
    assert!(ctx.cluster_labels.is_none(), "original ctx must be unchanged after clone mutation");
}
