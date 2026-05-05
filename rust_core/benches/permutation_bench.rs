use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tunny_core::sensitivity::compute_permutation_importances;

fn bench_permutation(c: &mut Criterion) {
    let mut group = c.benchmark_group("permutation_n_features");
    for &p in &[3usize, 10, 20] {
        group.bench_with_input(BenchmarkId::new("compute", p), &p, |b, &p| {
            let n = 200;
            let x: Vec<Vec<f64>> = (0..n)
                .map(|i| (0..p).map(|j| ((i * p + j) as f64) * 0.01).collect())
                .collect();
            let y: Vec<f64> = (0..n)
                .map(|i| {
                    let fi = i as f64;
                    (fi * 0.07).cos() + fi * 0.02
                })
                .collect();
            b.iter(|| compute_permutation_importances(&x, &y));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_permutation);
criterion_main!(benches);
