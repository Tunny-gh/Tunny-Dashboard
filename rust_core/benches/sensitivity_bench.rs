use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use tunny_core::dataframe::{select_study, store_dataframes, DataFrame, TrialRow};
use tunny_core::sensitivity::compute_sensitivity;

fn setup_sensitivity_df(n: usize, n_params: usize, n_objectives: usize) {
    let param_names: Vec<String> = (0..n_params).map(|i| format!("x{}", i)).collect();
    let objective_names: Vec<String> = (0..n_objectives).map(|i| format!("obj{}", i)).collect();

    let rows: Vec<TrialRow> = (0..n)
        .map(|i| {
            let fi = i as f64;
            let param_display: HashMap<String, f64> = param_names
                .iter()
                .enumerate()
                .map(|(j, name)| (name.clone(), fi * (j as f64 + 1.0) * 0.01))
                .collect();

            let objective_values: Vec<f64> = (0..n_objectives)
                .map(|k| fi * (k as f64 + 1.0) * 0.05 + (k as f64))
                .collect();

            TrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display,
                param_category_label: HashMap::new(),
                objective_values,
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                constraint_values: vec![],
            }
        })
        .collect();

    let df = DataFrame::from_trials(&rows, &param_names, &objective_names, &[], &[], 0);
    store_dataframes(vec![df]);
    select_study(0).expect("select_study failed");
}

fn bench_sensitivity(c: &mut Criterion) {
    let mut group = c.benchmark_group("sensitivity_objectives");
    for &n_obj in &[1usize, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("compute_sensitivity", n_obj),
            &n_obj,
            |b, &n_obj| {
                setup_sensitivity_df(200, 5, n_obj);
                b.iter(compute_sensitivity);
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_sensitivity);
criterion_main!(benches);
