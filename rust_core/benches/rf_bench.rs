use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tunny_core::RandomForest;

fn make_data(n: usize, p: usize) -> (Vec<Vec<f64>>, Vec<f64>) {
    let x: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..p).map(|j| ((i * p + j) as f64) * 0.01).collect())
        .collect();
    let y: Vec<f64> = (0..n)
        .map(|i| {
            let fi = i as f64;
            (fi * 0.13).sin() + fi * 0.01
        })
        .collect();
    (x, y)
}

fn bench_rf_train(c: &mut Criterion) {
    let mut group = c.benchmark_group("rf_train_n_trees");
    for &n_trees in &[10usize, 50, 100] {
        group.bench_with_input(BenchmarkId::new("train", n_trees), &n_trees, |b, &n_trees| {
            let (x, y) = make_data(200, 5);
            b.iter(|| RandomForest::train(&x, &y, n_trees, 5, 2, 42));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_rf_train);
criterion_main!(benches);
