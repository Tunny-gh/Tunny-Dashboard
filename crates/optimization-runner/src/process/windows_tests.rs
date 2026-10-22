//! Windows counterparts to the Unix shell integration tests.
use super::*;
use crate::runner::{prepare_run, run_prepared, RunConfig};
use tdashboard_analysis::surrogate_opt::FitProgress;

fn command(script: &str, dir: &std::path::Path) -> CommandSpec {
    CommandSpec {
        program: "powershell.exe".into(),
        args: vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            script.into(),
        ],
        working_dir: Some(dir.to_string_lossy().into_owned()),
        timeout_secs: 10,
        retries: 0,
    }
}

fn definition(command: CommandSpec) -> ProcessDefinition {
    ProcessDefinition {
        param_names: vec!["x".into()],
        input: InputSpec::Env,
        command,
        objectives: vec![OutputSpec {
            name: "f".into(),
            source: OutputSource::Stdout,
            extractor: Extractor::Regex {
                pattern: r"f=([-0-9.]+)".into(),
            },
        }],
        constraints: vec![],
        pre_command: None,
        post_command: None,
    }
}

#[test]
fn windows_retry_counts_attempts_and_preserves_failure_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = command(
        "Add-Content attempts.txt attempt; [Console]::Error.WriteLine('boom'); exit 7",
        dir.path(),
    );
    cmd.retries = 2;
    let err = ProcessEvaluator::new(definition(cmd))
        .unwrap()
        .evaluate(&[1.0])
        .unwrap_err();
    assert!(
        err.contains("exited with 7") && err.contains("boom"),
        "{err}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("attempts.txt"))
            .unwrap()
            .lines()
            .count(),
        3
    );
}

#[test]
fn windows_timeout_kills_hanging_command() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = command("Start-Sleep -Seconds 10", dir.path());
    cmd.timeout_secs = 1;
    let start = std::time::Instant::now();
    let err = ProcessEvaluator::new(definition(cmd))
        .unwrap()
        .evaluate(&[1.0])
        .unwrap_err();
    assert!(err.contains("timed out"), "{err}");
    assert!(start.elapsed() < std::time::Duration::from_secs(8));
}

#[test]
fn windows_hooks_extraction_and_recorded_readback() {
    let dir = tempfile::tempdir().unwrap();
    let mut def = definition(command(
        "if (!(Test-Path pre.txt)) { exit 9 }; Write-Output ('f=' + $env:x)",
        dir.path(),
    ));
    def.pre_command = Some(command("Set-Content pre.txt ready", dir.path()));
    def.post_command = Some(command("Set-Content result.json '{\"g\":2.5}'", dir.path()));
    def.constraints.push(OutputSpec {
        name: "g".into(),
        source: OutputSource::File {
            path: "result.json".into(),
        },
        extractor: Extractor::JsonPath { path: "g".into() },
    });
    let ranges = std::collections::HashMap::from([(
        "x".into(),
        VarRange {
            low: 1.0,
            high: 5.0,
            digits: 0,
            is_integer: true,
        },
    )]);
    let problem = def.build_problem(&ranges).unwrap();
    let evaluator = ProcessEvaluator::new(def).unwrap();
    let cfg = RunConfig {
        sampler: crate::runner::Sampler::Random,
        n_trials: 2,
        directions: vec![tdashboard_io::journal::parser::OptimizationDirection::Minimize],
        ..Default::default()
    };
    let path = dir.path().join("run.log");
    let prep = prepare_run(&path, &problem, &cfg).unwrap();
    // The shared hook files deliberately require serial evaluation.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let summary = pool
        .install(|| run_prepared(&prep, &problem, &evaluator, &cfg, &FitProgress::new()))
        .unwrap();
    assert_eq!(summary.completed, 2);
    let (_, df, _) =
        crate::test_support::parse_single_study(&std::fs::read(path).unwrap(), 0).unwrap();
    assert_eq!(df.get_numeric_column("x"), df.get_numeric_column("f"));
    assert_eq!(df.get_numeric_column("c1").unwrap(), &[2.5, 2.5]);
}

#[test]
fn windows_hook_failure_and_extraction_failure_are_reported() {
    let dir = tempfile::tempdir().unwrap();
    let mut def = definition(command("Write-Output no_objective", dir.path()));
    def.pre_command = Some(command("exit 8", dir.path()));
    assert!(ProcessEvaluator::new(def.clone())
        .unwrap()
        .evaluate(&[1.0])
        .unwrap_err()
        .contains("8"));
    def.pre_command = None;
    assert!(ProcessEvaluator::new(def)
        .unwrap()
        .evaluate(&[1.0])
        .unwrap_err()
        .contains("match"));
}
