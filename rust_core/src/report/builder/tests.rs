use std::collections::HashMap;

use crate::data::dataframe::{DataFrame, TrialRow};
use crate::data::extras::{StudyExtras, TrialExtra, TrialState};
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};
use crate::report::model::*;
use crate::report::{build_study_report, render_markdown, ReportLang, ReportOptions, ReportSource};

fn source() -> ReportSource {
    ReportSource {
        storage_display: "sqlite:///demo.db".to_string(),
        generated_at_unix: Some(1_700_000_000),
    }
}

fn opts() -> ReportOptions {
    ReportOptions::default()
}

fn row(id: u32, params: &[(&str, f64)], objs: &[f64]) -> TrialRow {
    TrialRow {
        trial_id: id,
        trial_number: id,
        param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        param_category_label: HashMap::new(),
        objective_values: objs.to_vec(),
        user_attrs_numeric: HashMap::new(),
        user_attrs_string: HashMap::new(),
        constraint_values: vec![],
    }
}

fn meta_single() -> StudyMeta {
    StudyMeta {
        study_id: 7,
        name: "single_study".to_string(),
        directions: vec![OptimizationDirection::Minimize],
        completed_trials: 12,
        total_trials: 12,
        param_names: vec!["a".to_string(), "b".to_string()],
        objective_names: vec!["obj0".to_string()],
        user_attr_names: vec![],
        has_constraints: false,
        param_bounds: HashMap::new(),
    }
}

fn df_single() -> DataFrame {
    // obj0 = a (monotonically increasing) -> best is settled at trial0 (no
    // update in the latter half = Converged).
    // a is perfectly correlated with obj0 (|ρ|=1), giving it the highest
    // importance. b is weakly correlated.
    let b = [5.0, 3.0, 8.0, 1.0, 9.0, 2.0, 7.0, 0.0, 6.0, 4.0, 10.0, 11.0];
    let rows: Vec<TrialRow> = (0..12)
        .map(|i| row(i, &[("a", i as f64), ("b", b[i as usize])], &[i as f64]))
        .collect();
    DataFrame::from_trials(
        &rows,
        &["a".to_string(), "b".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        0,
    )
}

fn meta_multi() -> StudyMeta {
    StudyMeta {
        study_id: 9,
        name: "multi_study".to_string(),
        directions: vec![
            OptimizationDirection::Minimize,
            OptimizationDirection::Minimize,
        ],
        completed_trials: 6,
        total_trials: 6,
        param_names: vec!["p".to_string()],
        objective_names: vec!["obj0".to_string(), "obj1".to_string()],
        user_attr_names: vec![],
        has_constraints: false,
        param_bounds: HashMap::new(),
    }
}

fn df_multi() -> DataFrame {
    // front = {trial0(1,4), trial1(2,2), trial2(4,1)}, the rest are
    // dominated.
    let pts = [
        (1.0, 4.0),
        (2.0, 2.0),
        (4.0, 1.0),
        (3.0, 3.0),
        (5.0, 5.0),
        (2.0, 3.0),
    ];
    let rows: Vec<TrialRow> = pts
        .iter()
        .enumerate()
        .map(|(i, &(o0, o1))| row(i as u32, &[("p", i as f64)], &[o0, o1]))
        .collect();
    DataFrame::from_trials(
        &rows,
        &["p".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    )
}

// =============================================================================
// Single-objective
// =============================================================================

#[test]
fn single_objective_outcome_and_convergence() {
    let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());

    match &report.outcome {
        Outcome::SingleObj { best_trial, top_n } => {
            let bt = best_trial.as_ref().expect("best trial present");
            assert_eq!(bt.trial_number, 0, "obj0=a 最小は trial0");
            assert_eq!(bt.objectives, vec![0.0]);
            assert!(!top_n.is_empty());
            // Top entries are ordered best (smallest) first.
            let first = top_n[0].objectives[0];
            let last = top_n[top_n.len() - 1].objectives[0];
            assert!(first <= last, "top_n は最良順で並ぶ");
        }
        _ => panic!("single-objective study must yield SingleObj"),
    }

    // best-so-far is non-increasing (monotonic) for minimization.
    let vals: Vec<f64> = report.convergence.series.iter().map(|p| p.value).collect();
    for w in vals.windows(2) {
        assert!(w[1] <= w[0], "best-so-far は非増加であるべき: {w:?}");
    }
    assert_eq!(report.convergence.metric, ConvergenceMetric::BestSoFar);
    assert_eq!(report.convergence.status, ConvergenceStatus::Converged);
    assert_eq!(report.convergence.found_at_trial_number, Some(0));
    assert!(!report.convergence.improved_in_last_20pct);
}

#[test]
fn single_objective_importance_orders_a_first() {
    let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
    let imp = report.importance.expect("importance computed");
    assert_eq!(imp.method, "spearman_abs");
    assert_eq!(imp.objective_name, "obj0");
    assert_eq!(imp.scores[0].0, "a", "a は obj0 と完全相関で最重要");
    // Descending order.
    for w in imp.scores.windows(2) {
        assert!(w[0].1 >= w[1].1, "importance は降順");
    }
}

#[test]
fn single_objective_key_findings_present() {
    let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
    let kinds: Vec<FindingKind> = report.key_findings.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&FindingKind::BestSingle));
    assert!(kinds.contains(&FindingKind::ConvergenceStatus));
    assert!(kinds.contains(&FindingKind::TopImportance));
    assert!(!kinds.contains(&FindingKind::ParetoSummary));
}

// =============================================================================
// Multi-objective
// =============================================================================

#[test]
fn multi_objective_pareto_and_mcdm_consensus() {
    let report = build_study_report(&meta_multi(), &df_multi(), None, &source(), &opts());

    match &report.outcome {
        Outcome::MultiObj {
            pareto_size,
            complete_count,
            objective_count,
            per_objective_extremes,
            scatter,
            ..
        } => {
            assert_eq!(*pareto_size, 3, "front = trial0,1,2");
            assert_eq!(*complete_count, 6);
            assert_eq!(*objective_count, 2);
            // obj0's best is trial0 (=1), obj1's best is trial2 (=1).
            let e0 = &per_objective_extremes[0];
            assert_eq!(e0.best_trial_number, 0);
            assert_eq!(e0.best_value, 1.0);
            let e1 = &per_objective_extremes[1];
            assert_eq!(e1.best_trial_number, 2);
            assert_eq!(e1.best_value, 1.0);
            // The scatter plot covers all COMPLETE trials, with 3 front points.
            assert_eq!(scatter.len(), 6);
            assert_eq!(scatter.iter().filter(|p| p.on_front).count(), 3);
        }
        _ => panic!("multi-objective study must yield MultiObj"),
    }

    let mcdm = report.mcdm.expect("mcdm present for multi-objective");
    assert_eq!(mcdm.weight_scheme, "equal");
    assert_eq!(mcdm.weights, vec![0.5, 0.5]);
    // The front has 3 entries, so the top10 of all three methods is the
    // entire front -> consensus is {0,1,2}.
    assert_eq!(mcdm.consensus_trials, vec![0, 1, 2]);

    // HV is non-decreasing.
    let vals: Vec<f64> = report.convergence.series.iter().map(|p| p.value).collect();
    assert_eq!(report.convergence.metric, ConvergenceMetric::Hypervolume);
    for w in vals.windows(2) {
        assert!(w[1] >= w[0], "HV は非減少: {w:?}");
    }
}

#[test]
fn skip_decision_sections_omits_mcdm_and_correlations_but_keeps_ordering() {
    let mut skip_opts = opts();
    skip_opts.skip_decision_sections = true;
    let report = build_study_report(&meta_multi(), &df_multi(), None, &source(), &skip_opts);

    assert!(
        report.mcdm.is_none(),
        "skip_decision_sections なら mcdm は None"
    );
    assert!(
        report.correlations.is_none(),
        "skip_decision_sections なら correlations も None"
    );

    // pareto_table's TOPSIS ordering keeps working even when mcdm is skipped.
    let baseline = build_study_report(&meta_multi(), &df_multi(), None, &source(), &opts());
    match (&report.outcome, &baseline.outcome) {
        (
            Outcome::MultiObj {
                pareto_table: skipped_table,
                ..
            },
            Outcome::MultiObj {
                pareto_table: full_table,
                ..
            },
        ) => {
            let skipped_order: Vec<u32> = skipped_table.iter().map(|t| t.trial_number).collect();
            let full_order: Vec<u32> = full_table.iter().map(|t| t.trial_number).collect();
            assert_eq!(
                skipped_order, full_order,
                "TOPSIS 順は mcdm 省略の有無に関わらず一致するべき"
            );
        }
        _ => panic!("multi-objective study must yield MultiObj"),
    }

    // Key Findings should not change.
    let skipped_kinds: Vec<FindingKind> = report.key_findings.iter().map(|f| f.kind).collect();
    let baseline_kinds: Vec<FindingKind> = baseline.key_findings.iter().map(|f| f.kind).collect();
    assert_eq!(skipped_kinds, baseline_kinds);
}

#[test]
fn multi_objective_trade_off_finding() {
    // obj0 ascending, obj1 descending (Spearman ρ = -1 across all COMPLETE
    // points) -> TradeOff finding.
    let pts = [
        (1.0, 6.0),
        (2.0, 5.0),
        (3.0, 4.0),
        (4.0, 3.0),
        (5.0, 2.0),
        (6.0, 1.0),
    ];
    let rows: Vec<TrialRow> = pts
        .iter()
        .enumerate()
        .map(|(i, &(o0, o1))| row(i as u32, &[("p", i as f64)], &[o0, o1]))
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &["p".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    );
    let report = build_study_report(&meta_multi(), &df, None, &source(), &opts());
    let kinds: Vec<FindingKind> = report.key_findings.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&FindingKind::ParetoSummary));
    assert!(kinds.contains(&FindingKind::TradeOff));
    let to = report
        .key_findings
        .iter()
        .find(|f| f.kind == FindingKind::TradeOff)
        .unwrap();
    assert!(to.metrics.get("rho").copied().unwrap() < -0.3);
}

// =============================================================================
// Robustness (0 / 1 trial, NaN, constraints, extras)
// =============================================================================

#[test]
fn zero_trials_does_not_panic() {
    let meta = meta_single();
    let df = DataFrame::from_trials(
        &[],
        &["a".to_string(), "b".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        0,
    );
    let report = build_study_report(&meta, &df, None, &source(), &opts());

    assert_eq!(report.overview.complete_trials, 0);
    assert_eq!(report.convergence.status, ConvergenceStatus::Insufficient);
    assert!(report.convergence.series.is_empty());
    match &report.outcome {
        Outcome::SingleObj { best_trial, top_n } => {
            assert!(best_trial.is_none());
            assert!(top_n.is_empty());
        }
        _ => panic!("expected SingleObj"),
    }
    assert!(report.importance.is_none());
    // Rendering must not panic either.
    let _ = render_markdown(&report, ReportLang::En);
    let _ = render_markdown(&report, ReportLang::Ja);
}

#[test]
fn single_trial_robustness() {
    let meta = meta_single();
    let rows = vec![row(0, &[("a", 1.0), ("b", 2.0)], &[3.0])];
    let df = DataFrame::from_trials(
        &rows,
        &["a".to_string(), "b".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        0,
    );
    let report = build_study_report(&meta, &df, None, &source(), &opts());
    assert_eq!(report.overview.complete_trials, 1);
    assert_eq!(report.convergence.status, ConvergenceStatus::Insufficient);
    match &report.outcome {
        Outcome::SingleObj { best_trial, .. } => {
            assert_eq!(best_trial.as_ref().unwrap().trial_number, 0);
        }
        _ => panic!("expected SingleObj"),
    }
    // importance/correlations are not computed since n<2.
    assert!(report.importance.is_none());
    assert!(report.correlations.is_none());
}

#[test]
fn nan_objective_triggers_data_quality() {
    let meta = meta_single();
    let mut rows: Vec<TrialRow> = (0..11)
        .map(|i| row(i, &[("a", i as f64), ("b", 0.0)], &[i as f64]))
        .collect();
    // Just one NaN objective value.
    rows.push(row(11, &[("a", 11.0), ("b", 0.0)], &[f64::NAN]));
    let df = DataFrame::from_trials(
        &rows,
        &["a".to_string(), "b".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        0,
    );
    let report = build_study_report(&meta, &df, None, &source(), &opts());
    let dq = report
        .key_findings
        .iter()
        .find(|f| f.kind == FindingKind::DataQuality)
        .expect("DataQuality finding for NaN objective");
    assert_eq!(dq.metrics.get("nan_count").copied(), Some(1.0));
}

#[test]
fn extras_produce_execution_and_pruning() {
    let meta = meta_single();
    let df = df_single();
    let mut extras = StudyExtras::default();
    for i in 0..12u32 {
        extras.trials.push(TrialExtra {
            trial_id: i,
            trial_number: i,
            state: TrialState::Complete,
            datetime_start: Some(i as f64),
            datetime_complete: Some(i as f64 + 1.0),
            intermediate_values: vec![],
        });
    }
    // Add 2 PRUNED trials (with intermediate-value steps) and 1 FAIL trial.
    extras.trials.push(TrialExtra {
        trial_id: 12,
        trial_number: 12,
        state: TrialState::Pruned,
        datetime_start: Some(0.0),
        datetime_complete: Some(2.0),
        intermediate_values: vec![(0, 1.0), (5, 0.5)],
    });
    extras.trials.push(TrialExtra {
        trial_id: 13,
        trial_number: 13,
        state: TrialState::Pruned,
        datetime_start: Some(0.0),
        datetime_complete: Some(2.0),
        intermediate_values: vec![(0, 1.0), (3, 0.5)],
    });
    extras.trials.push(TrialExtra {
        trial_id: 14,
        trial_number: 14,
        state: TrialState::Fail,
        datetime_start: None,
        datetime_complete: None,
        intermediate_values: vec![],
    });

    let report = build_study_report(&meta, &df, Some(&extras), &source(), &opts());
    let exec = report.execution.expect("execution present with extras");
    assert_eq!(exec.state_counts.get("PRUNED").copied(), Some(2));
    assert_eq!(exec.state_counts.get("FAIL").copied(), Some(1));
    assert!(exec.pruned_rate > 0.0);
    assert!(exec.median_prune_step.is_some());
    assert!(exec.mean_trial_seconds.is_some());

    let kinds: Vec<FindingKind> = report.key_findings.iter().map(|f| f.kind).collect();
    assert!(kinds.contains(&FindingKind::PruningEfficiency));
    assert!(kinds.contains(&FindingKind::DataQuality)); // FAIL>0
}

// =============================================================================
// Markdown renderer
// =============================================================================

#[test]
fn markdown_contains_all_sections_for_full_study() {
    let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
    let md = render_markdown(&report, ReportLang::En);
    for heading in [
        // The `_` in the study name is escaped to `\_` so it isn't
        // misinterpreted as Markdown emphasis (it still renders as `_`).
        "# Optimization Report: single\\_study",
        "## Key Findings",
        "## Outcome",
        "## Convergence",
        "## Parameter Importance",
        "## Objective Statistics",
        "## Correlations",
        "## Reproduction",
    ] {
        assert!(
            md.contains(heading),
            "markdown must contain heading: {heading}"
        );
    }
}

#[test]
fn markdown_is_deterministic() {
    let report = build_study_report(&meta_multi(), &df_multi(), None, &source(), &opts());
    let a = render_markdown(&report, ReportLang::En);
    let b = render_markdown(&report, ReportLang::En);
    assert_eq!(a, b, "同一入力→バイト同一");
}

#[test]
fn markdown_escapes_pipe_in_names() {
    let mut meta = meta_single();
    meta.name = "a|b".to_string();
    let report = build_study_report(&meta, &df_single(), None, &source(), &opts());
    let md = render_markdown(&report, ReportLang::En);
    assert!(
        md.contains("a\\|b"),
        "study 名のパイプはエスケープされるべき"
    );
}

#[test]
fn markdown_escapes_backslash_before_pipe() {
    // Backslash escaping must happen before pipe escaping (otherwise the
    // `\` in `a\|b` could break the table structure). Verify with a name
    // that includes `\|` and a trailing `\`.
    let mut meta = meta_single();
    meta.name = r"a\|b\".to_string();
    let report = build_study_report(&meta, &df_single(), None, &source(), &opts());
    let md = render_markdown(&report, ReportLang::En);
    assert!(
        md.contains(r"a\\\|b\\"),
        "バックスラッシュは `|` エスケープより前に二重化されるべき: {md}"
    );
}

#[test]
fn markdown_japanese_smoke() {
    let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
    let md = render_markdown(&report, ReportLang::Ja);
    assert!(md.contains("## まとめ"));
    assert!(md.contains("## 最適化結果"));
    assert!(md.contains("## 再現情報"));
}

#[test]
fn json_serialization_is_deterministic() {
    let report = build_study_report(&meta_multi(), &df_multi(), None, &source(), &opts());
    let a = serde_json::to_string_pretty(&report).unwrap();
    let b = serde_json::to_string_pretty(&report).unwrap();
    assert_eq!(a, b);
    assert!(a.contains("\"schema_version\""));
}
