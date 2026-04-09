/// Criterion benchmarks for downsampling functions.
///
/// Performance target: < 5ms per function at 50,000 points (native target).
/// Run with: cargo bench --no-default-features
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use tunny_core::{
    dataframe::{select_study, store_dataframes, DataFrame, TrialRow},
    sampling::{
        downsample_by_cluster, downsample_for_thumbnail, downsample_smart,
        downsample_stratified_by_rank, init_sampling, set_cluster_labels,
    },
};

/// Build a DataFrame with `n` rows (2 objectives, 3 params).
/// Row 0 is the Pareto-optimal point.
fn setup_df(n: usize) {
    let rows: Vec<TrialRow> = (0..n)
        .map(|i| {
            let fi = i as f64;
            TrialRow {
                trial_id: i as u32,
                param_display: {
                    let mut m = HashMap::new();
                    m.insert("x1".to_string(), fi * 0.001);
                    m.insert("x2".to_string(), fi * 0.002);
                    m.insert("x3".to_string(), fi * 0.003);
                    m
                },
                param_category_label: HashMap::new(),
                objective_values: vec![fi, (n as f64) - fi],
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            }
        })
        .collect();

    let df = DataFrame::from_trials(
        &rows,
        &["x1".to_string(), "x2".to_string(), "x3".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    store_dataframes(vec![df]);
    select_study(0).expect("select_study failed");

    // Row 0 is the Pareto point (min obj0=0, max obj1=n-1 → only it dominates all others
    // in multi-objective case row 0 has min obj0 but max obj1, so real Pareto front = all points).
    // For benchmark purposes use all indices as Pareto (worst case).
    let pareto_indices: Vec<u32> = (0..n.min(500) as u32).collect();
    let all_ranks: Vec<u32> = (0..n as u32).map(|i| if i < 500 { 1 } else { 2 }).collect();
    init_sampling(vec![true, true], pareto_indices, all_ranks);

    // Set dummy cluster labels (round-robin into 5 clusters)
    let labels: Vec<i32> = (0..n).map(|i| (i % 5) as i32).collect();
    set_cluster_labels(labels);
}

fn bench_downsample_smart(c: &mut Criterion) {
    let n = 50_000usize;
    setup_df(n);
    c.bench_with_input(
        BenchmarkId::new("downsample_smart", n),
        &n,
        |b, _| {
            b.iter(|| downsample_smart(10_000, true));
        },
    );
}

fn bench_downsample_for_thumbnail(c: &mut Criterion) {
    let n = 50_000usize;
    setup_df(n);
    c.bench_with_input(
        BenchmarkId::new("downsample_for_thumbnail", n),
        &n,
        |b, _| {
            b.iter(|| downsample_for_thumbnail(500));
        },
    );
}

fn bench_downsample_stratified_by_rank(c: &mut Criterion) {
    let n = 50_000usize;
    setup_df(n);
    c.bench_with_input(
        BenchmarkId::new("downsample_stratified_by_rank", n),
        &n,
        |b, _| {
            b.iter(|| downsample_stratified_by_rank(1_000, 5));
        },
    );
}

fn bench_downsample_by_cluster(c: &mut Criterion) {
    let n = 50_000usize;
    setup_df(n);
    c.bench_with_input(
        BenchmarkId::new("downsample_by_cluster", n),
        &n,
        |b, _| {
            b.iter(|| downsample_by_cluster(10_000));
        },
    );
}

fn bench_all_six_keys(c: &mut Criterion) {
    let n = 50_000usize;
    setup_df(n);
    c.bench_with_input(
        BenchmarkId::new("all_six_keys_combined", n),
        &n,
        |b, _| {
            b.iter(|| {
                let _ = downsample_smart(10_000, true); // scatter
                let _ = downsample_for_thumbnail(500); // thumbnail
                let _ = downsample_for_thumbnail(3_000); // hover
                let _ = downsample_stratified_by_rank(1_000, 5); // pcp
                let _ = downsample_smart(5_000, false); // data_points
                let _ = downsample_by_cluster(10_000); // cluster
            });
        },
    );
}

criterion_group!(
    benches,
    bench_downsample_smart,
    bench_downsample_for_thumbnail,
    bench_downsample_stratified_by_rank,
    bench_downsample_by_cluster,
    bench_all_six_keys,
);
criterion_main!(benches);
