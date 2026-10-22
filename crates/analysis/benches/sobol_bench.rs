use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use tdashboard_analysis::dataframe::{DataFrame, TrialRow};
use tdashboard_analysis::sensitivity::compute_sobol;

fn setup_sobol_df(n: usize, n_params: usize, n_objectives: usize) -> DataFrame {
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
                .map(|k| fi * (k as f64 + 1.0) * 0.03 + (k as f64))
                .collect();

            TrialRow {
                trial_id: i as u32,
                trial_number: i as u32,
                param_display,
                distribution_metadata: Default::default(),
                param_category_label: HashMap::new(),
                objective_values,
                user_attrs_numeric: HashMap::new(),
                user_attrs_string: HashMap::new(),
                user_attrs_json: HashMap::new(),
                constraint_values: vec![],
            }
        })
        .collect();

    DataFrame::from_trials(&rows, &param_names, &objective_names, &[], &[], 0)
}

fn bench_sobol(c: &mut Criterion) {
    let mut group = c.benchmark_group("sobol_objectives");
    for &n_obj in &[1usize, 2, 4] {
        group.bench_with_input(
            BenchmarkId::new("compute_sobol", n_obj),
            &n_obj,
            |b, &n_obj| {
                let df = setup_sobol_df(200, 5, n_obj);
                b.iter(|| compute_sobol(&df, 512));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_sobol);
criterion_main!(benches);
