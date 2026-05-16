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
        downsample_stratified_by_rank, init_sampling, SamplingContext,
    },
};

/// Build a DataFrame with `n` rows (2 objectives, 3 params) and return a SamplingContext.
fn setup_df(n: usize) -> SamplingContext {
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

    let pareto_indices: Vec<u32> = (0..n.min(500) as u32).collect();
    let all_ranks: Vec<u32> = (0..n as u32).map(|i| if i < 500 { 0 } else { 1 }).collect();
    let mut ctx = init_sampling(vec![true, true], pareto_indices, all_ranks);

    let labels: Vec<i32> = (0..n).map(|i| (i % 5) as i32).collect();
    ctx.cluster_labels = Some(labels);
    ctx
}

fn bench_downsample_smart(c: &mut Criterion) {
    let n = 50_000usize;
    let ctx = setup_df(n);
    c.bench_with_input(BenchmarkId::new("downsample_smart", n), &n, |b, _| {
        b.iter(|| downsample_smart(&ctx, 10_000, true));
    });
}

fn bench_downsample_for_thumbnail(c: &mut Criterion) {
    let n = 50_000usize;
    let ctx = setup_df(n);
    c.bench_with_input(
        BenchmarkId::new("downsample_for_thumbnail", n),
        &n,
        |b, _| {
            b.iter(|| downsample_for_thumbnail(&ctx, 500));
        },
    );
}

fn bench_downsample_stratified_by_rank(c: &mut Criterion) {
    let n = 50_000usize;
    let ctx = setup_df(n);
    c.bench_with_input(
        BenchmarkId::new("downsample_stratified_by_rank", n),
        &n,
        |b, _| {
            b.iter(|| downsample_stratified_by_rank(&ctx, 1_000, 5));
        },
    );
}

fn bench_downsample_by_cluster(c: &mut Criterion) {
    let n = 50_000usize;
    let ctx = setup_df(n);
    c.bench_with_input(BenchmarkId::new("downsample_by_cluster", n), &n, |b, _| {
        b.iter(|| downsample_by_cluster(&ctx, 10_000));
    });
}

fn bench_all_six_keys(c: &mut Criterion) {
    let n = 50_000usize;
    let ctx = setup_df(n);
    c.bench_with_input(BenchmarkId::new("all_six_keys_combined", n), &n, |b, _| {
        b.iter(|| {
            let _ = downsample_smart(&ctx, 10_000, true);
            let _ = downsample_for_thumbnail(&ctx, 500);
            let _ = downsample_for_thumbnail(&ctx, 3_000);
            let _ = downsample_stratified_by_rank(&ctx, 1_000, 5);
            let _ = downsample_smart(&ctx, 5_000, false);
            let _ = downsample_by_cluster(&ctx, 10_000);
        });
    });
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
