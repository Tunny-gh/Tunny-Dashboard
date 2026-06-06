use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tunny_core::io::journal::parser::{parse_single_study, scan_study_list};

/// 単一 Study・大量 Trial の合成 journal (`n_results.log` 相当) を生成する。
/// 1× op0(create_study) + N×{ op4(create_trial) + 複数 op5(set_trial_param) + op6(state_values) }。
/// op5 には実データ同様の `distribution` 文字列を持たせ、パース負荷を再現する。
fn build_n_results(n_trials: u32, n_params: u32) -> Vec<u8> {
    let mut lines: Vec<String> = Vec::with_capacity((n_trials * (n_params + 2) + 2) as usize);
    lines.push(
        r#"{"op_code":0,"worker_id":"w","study_name":"n_results","directions":[1,1]}"#.to_string(),
    );
    lines.push(
        r#"{"op_code":3,"worker_id":"w","study_id":0,"system_attr":{"study:metric_names":["Obj1","Obj2"]}}"#
            .to_string(),
    );
    for t in 0..n_trials {
        lines.push(
            r#"{"op_code":4,"worker_id":"w","study_id":0,"datetime_start":"2026-03-28T11:58:48.485367"}"#
                .to_string(),
        );
        for p in 0..n_params {
            let val = f64::from(t * n_params + p) * 0.001;
            lines.push(format!(
                r#"{{"op_code":5,"worker_id":"w","trial_id":{t},"param_name":"x{p}","param_value_internal":{val},"distribution":"{{\"name\": \"FloatDistribution\", \"attributes\": {{\"step\": null, \"low\": -32.77, \"high\": 32.77, \"log\": false}}}}"}}"#
            ));
        }
        let v0 = f64::from(t) * 0.01;
        let v1 = f64::from(t) * 0.02;
        lines.push(format!(
            r#"{{"op_code":6,"worker_id":"w","trial_id":{t},"state":1,"values":[{v0},{v1}],"datetime_complete":"2026-03-28T11:58:48.612043"}}"#
        ));
    }
    lines.join("\n").into_bytes()
}

fn bench_synthetic(c: &mut Criterion) {
    let mut group = c.benchmark_group("journal_synthetic");
    group.sample_size(20);
    for &n in &[20_000u32, 50_000u32] {
        let data = build_n_results(n, 5);

        group.bench_with_input(BenchmarkId::new("scan_study_list", n), &data, |b, data| {
            b.iter(|| scan_study_list(black_box(data)).expect("scan failed"));
        });

        group.bench_with_input(
            BenchmarkId::new("parse_single_study", n),
            &data,
            |b, data| {
                b.iter(|| parse_single_study(black_box(data), 0).expect("parse failed"));
            },
        );
    }
    group.finish();
}

fn bench_real_fixture(c: &mut Criterion) {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/test.log");
    let Ok(data) = std::fs::read(path) else {
        return;
    };

    let mut group = c.benchmark_group("journal_fixture_test_log");
    group.sample_size(20);
    group.bench_function("scan_study_list", |b| {
        b.iter(|| scan_study_list(black_box(&data)).expect("scan failed"));
    });
    group.bench_function("parse_single_study", |b| {
        b.iter(|| parse_single_study(black_box(&data), 0).expect("parse failed"));
    });
    group.finish();
}

criterion_group!(benches, bench_synthetic, bench_real_fixture);
criterion_main!(benches);
