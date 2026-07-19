use std::collections::HashMap;

use super::render_html;
use crate::data::dataframe::{DataFrame, TrialRow};
use crate::data::extras::{StudyExtras, TrialExtra, TrialState};
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};
use crate::report::model::Outcome;
use crate::report::{
    build_study_report, render_markdown, ReportLang, ReportOptions, ReportSource, StudyReport,
};

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
        param_names: vec!["p".to_string(), "q".to_string()],
        objective_names: vec!["obj0".to_string(), "obj1".to_string()],
        user_attr_names: vec![],
        has_constraints: false,
        param_bounds: HashMap::new(),
    }
}

fn df_multi() -> DataFrame {
    // front = {trial0(1,4), trial1(2,2), trial2(4,1)}, others are dominated.
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
        .map(|(i, &(o0, o1))| {
            row(
                i as u32,
                &[("p", i as f64), ("q", (6 - i) as f64)],
                &[o0, o1],
            )
        })
        .collect();
    DataFrame::from_trials(
        &rows,
        &["p".to_string(), "q".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        0,
    )
}

/// Builds extras (COMPLETE + PRUNED + FAIL) to enable the execution info section.
fn extras_multi() -> StudyExtras {
    let mut extras = StudyExtras::default();
    for i in 0..6u32 {
        extras.trials.push(TrialExtra {
            trial_id: i,
            trial_number: i,
            state: TrialState::Complete,
            datetime_start: Some(i as f64),
            datetime_complete: Some(i as f64 + 1.0),
            intermediate_values: vec![],
        });
    }
    extras.trials.push(TrialExtra {
        trial_id: 6,
        trial_number: 6,
        state: TrialState::Pruned,
        datetime_start: Some(0.0),
        datetime_complete: Some(2.0),
        intermediate_values: vec![(0, 1.0), (5, 0.5)],
    });
    extras.trials.push(TrialExtra {
        trial_id: 7,
        trial_number: 7,
        state: TrialState::Fail,
        datetime_start: None,
        datetime_complete: None,
        intermediate_values: vec![],
    });
    extras
}

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

/// Confirms the generated HTML is self-contained (zero external resource
/// references, no JS).
///
/// The SVG `xmlns="http://www.w3.org/2000/svg"` is a namespace
/// identifier and does not trigger a network fetch, so we only check for
/// external-fetch patterns (`href="http`, `src=`, `url(`, `@import`).
fn assert_self_contained(html: &str) {
    assert!(html.starts_with("<!DOCTYPE html>"), "DOCTYPE 先頭");
    assert!(!html.contains("<script"), "JS を含まない");
    assert!(!html.contains("href=\"http"), "外部リンク href を含まない");
    assert!(!html.contains("src="), "外部リソース src を含まない");
    assert!(!html.contains("url("), "CSS url() 参照を含まない");
    assert!(!html.contains("@import"), "外部 CSS import を含まない");
}

#[test]
fn multi_objective_full_section_render() {
    let report = build_study_report(
        &meta_multi(),
        &df_multi(),
        Some(&extras_multi()),
        &source(),
        &opts(),
    );
    let html = render_html(&report, ReportLang::En);

    assert_self_contained(&html);
    // All primary section anchors are present (multi-objective + full extras).
    for id in [
        "id=\"key-findings\"",
        "id=\"outcome\"",
        "id=\"convergence\"",
        "id=\"importance\"",
        "id=\"objective-stats\"",
        "id=\"correlations\"",
        "id=\"mcdm\"",
        "id=\"execution\"",
        "id=\"appendix\"",
    ] {
        assert!(html.contains(id), "セクション欠落: {id}");
    }
    // The table of contents also links to MCDM.
    assert!(html.contains("<nav class=\"toc\""));
    assert!(html.contains("href=\"#mcdm\""));

    // Charts: convergence line + scatter + importance hbar + histograms + heatmap.
    let n_svg = count(&html, "<svg");
    assert!(
        n_svg >= 4,
        "多目的レポートは複数チャートを埋め込む: {n_svg}"
    );
    // The scatter plot (front / dominated legend) is in Outcome.
    assert!(html.contains("Pareto front"), "散布図の凡例");
    // Headings for the three MCDM methods.
    assert!(html.contains(">TOPSIS<"));
    assert!(html.contains(">VIKOR<"));
    assert!(html.contains(">PROMETHEE II<"));
}

#[test]
fn single_objective_skips_mcdm_and_scatter() {
    let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
    let html = render_html(&report, ReportLang::En);

    assert_self_contained(&html);
    // Single-objective doesn't emit MCDM / scatter plot / execution info (no extras).
    assert!(!html.contains("id=\"mcdm\""), "単目的は MCDM を出さない");
    assert!(!html.contains("href=\"#mcdm\""), "目次に MCDM を出さない");
    assert!(!html.contains("Pareto front"), "単目的は散布図を出さない");
    assert!(
        !html.contains("id=\"execution\""),
        "extras 無しは実行情報なし"
    );
    // The primary single-objective sections are still present.
    assert!(html.contains("id=\"outcome\""));
    assert!(html.contains("id=\"convergence\""));
    assert!(html.contains("id=\"importance\""));
}

#[test]
fn user_strings_are_escaped() {
    let mut meta = meta_single();
    meta.name = "<script>alert('x')</script>".to_string();
    meta.param_names = vec!["<b>a</b>".to_string(), "b".to_string()];
    let rows: Vec<TrialRow> = (0..12)
        .map(|i| row(i, &[("<b>a</b>", i as f64), ("b", 0.0)], &[i as f64]))
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &["<b>a</b>".to_string(), "b".to_string()],
        &["obj0".to_string()],
        &[],
        &[],
        0,
    );
    let report = build_study_report(&meta, &df, None, &source(), &opts());
    let html = render_html(&report, ReportLang::En);

    // Raw <script> / <b> are escaped and appear as entity references.
    assert!(
        !html.contains("<script>alert"),
        "スクリプトが素通りしている"
    );
    assert!(!html.contains("<b>a</b>"), "パラメータ名の生タグが素通り");
    assert!(html.contains("&lt;script&gt;alert"));
    assert!(html.contains("&lt;b&gt;a&lt;/b&gt;"));
}

#[test]
fn output_is_deterministic() {
    let report = build_study_report(
        &meta_multi(),
        &df_multi(),
        Some(&extras_multi()),
        &source(),
        &opts(),
    );
    let a = render_html(&report, ReportLang::En);
    let b = render_html(&report, ReportLang::En);
    assert_eq!(a, b, "同一入力→バイト同一");
}

#[test]
fn japanese_headings_smoke() {
    let report = build_study_report(
        &meta_multi(),
        &df_multi(),
        Some(&extras_multi()),
        &source(),
        &opts(),
    );
    let html = render_html(&report, ReportLang::Ja);
    assert!(html.contains("<html lang=\"ja\">"));
    for heading in [
        "まとめ",
        "最適化結果",
        "収束",
        "意思決定分析",
        "実行情報",
        "付録",
    ] {
        assert!(html.contains(heading), "日本語見出し欠落: {heading}");
    }
}

/// Builds a constrained multi-objective row.
fn row_c(id: u32, params: &[(&str, f64)], objs: &[f64], cons: &[f64]) -> TrialRow {
    let mut r = row(id, params, objs);
    r.constraint_values = cons.to_vec();
    r
}

fn meta_multi_constrained() -> StudyMeta {
    let mut m = meta_multi();
    m.has_constraints = true;
    m
}

/// Objective-space front = {trial0, trial1, trial2}.
///
/// With `all_infeasible = true`, every row has c=[0.4, -1.0] (the sum is
/// -0.6, negative, but the max is 0.4, a violation), so there are zero
/// feasible solutions. This becomes the regression case for the
/// objective-space-front fallback plus the violation mark/note.
fn df_multi_constrained(all_infeasible: bool) -> DataFrame {
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
        .map(|(i, &(o0, o1))| {
            let cons: &[f64] = if all_infeasible {
                &[0.4, -1.0]
            } else {
                &[-0.5, -0.25]
            };
            row_c(
                i as u32,
                &[("p", i as f64), ("q", (6 - i) as f64)],
                &[o0, o1],
                cons,
            )
        })
        .collect();
    DataFrame::from_trials(
        &rows,
        &["p".to_string(), "q".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        2,
    )
}

#[test]
fn all_infeasible_falls_back_and_marks_violations() {
    // Zero feasible solutions → falls back to the objective-space front,
    // and the violation mark and fallback note are emitted.
    let report = build_study_report(
        &meta_multi_constrained(),
        &df_multi_constrained(true),
        None,
        &source(),
        &opts(),
    );
    let html = render_html(&report, ReportLang::En);

    // The column header carries the semantics.
    assert!(html.contains("max constraint (≤0 = feasible)"), "列ヘッダ");
    // Each row's sum would be -0.6 (looking unmarked), but the max is 0.4 → violation mark.
    assert!(
        html.contains("<td class=\"num infeasible\">0.4 ✗</td>"),
        "違反セルの赤字 + ✗ マーク（sum でなく max を表示）"
    );
    // The fallback note is emitted.
    assert!(
        html.contains("no trial satisfies all constraints"),
        "Pareto 表直下のフォールバック注記"
    );
    // The best trial in the extremes table also carries a violation mark.
    assert!(
        html.contains("<td class=\"num infeasible\">#"),
        "極値表の違反 trial マーク"
    );

    let ja = render_html(&report, ReportLang::Ja);
    assert!(ja.contains("最大制約値（≤0 で充足）"), "ja 列ヘッダ");
    assert!(ja.contains("件は制約違反です"), "ja 注記");
    assert!(ja.contains("フォールバック"), "ja フォールバック注記");

    // Markdown carries the same semantics.
    let md = render_markdown(&report, ReportLang::En);
    assert!(md.contains("max constraint (≤0 = feasible)"));
    assert!(md.contains("0.4 (infeasible)"));
    assert!(md.contains("no trial satisfies all constraints"));
    // Extremes table: both objectives are minimize, so best is #0 (obj0=1.0) / #2 (obj1=1.0).
    assert!(md.contains("#0 (infeasible)"), "極値表の違反マーク: {md}");
    let md_ja = render_markdown(&report, ReportLang::Ja);
    assert!(md_ja.contains("0.4（違反）"));
    assert!(md_ja.contains("件は制約違反です"));
}

#[test]
fn infeasible_trial_is_excluded_from_front_when_feasible_exist() {
    // trial1 (2,2) is on the objective-space front but violates
    // constraints → excluded from the front, which is recomputed from
    // the remaining feasible rows (no violation mark or note).
    let pts = [(1.0, 4.0), (2.0, 2.0), (4.0, 1.0), (3.0, 3.0)];
    let rows: Vec<TrialRow> = pts
        .iter()
        .enumerate()
        .map(|(i, &(o0, o1))| {
            let cons: &[f64] = if i == 1 { &[0.4] } else { &[-0.5] };
            row_c(i as u32, &[("p", i as f64), ("q", 1.0)], &[o0, o1], cons)
        })
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &["p".to_string(), "q".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        2,
    );
    let report = build_study_report(&meta_multi_constrained(), &df, None, &source(), &opts());

    let Outcome::MultiObj {
        pareto_table,
        scatter,
        ..
    } = &report.outcome
    else {
        panic!("multi-objective outcome expected");
    };
    let front_trials: Vec<u32> = pareto_table.iter().map(|t| t.trial_number).collect();
    assert!(
        !front_trials.contains(&1),
        "違反 trial1 は前面から除外: {front_trials:?}"
    );
    // After excluding trial1, (3,3) is also not dominated by (1,4)/(4,1), so it joins the front.
    assert!(front_trials.contains(&0) && front_trials.contains(&2));
    // In the scatter plot, trial1 remains as a point with feasible=false / on_front=false.
    let p1 = scatter.iter().find(|p| p.trial_number == 1).unwrap();
    assert!(!p1.feasible && !p1.on_front);

    let html = render_html(&report, ReportLang::En);
    assert!(!html.contains("class=\"num infeasible\""), "違反マークなし");
    assert!(
        !html.contains("no trial satisfies all constraints"),
        "注記なし"
    );
}

#[test]
fn duplicate_objective_trials_are_marked() {
    // trial1 and trial3 share identical objective values (2,2) → the
    // lower-numbered trial1 is canonical, trial3 gets duplicate_of = 1,
    // and both renderers emit the legend and the annotation.
    let pts = [(1.0, 4.0), (2.0, 2.0), (4.0, 1.0), (2.0, 2.0)];
    let rows: Vec<TrialRow> = pts
        .iter()
        .enumerate()
        .map(|(i, &(o0, o1))| row(i as u32, &[("p", i as f64), ("q", 1.0)], &[o0, o1]))
        .collect();
    let df = DataFrame::from_trials(
        &rows,
        &["p".to_string(), "q".to_string()],
        &["obj0".to_string(), "obj1".to_string()],
        &[],
        &[],
        2,
    );
    let report = build_study_report(&meta_multi(), &df, None, &source(), &opts());

    let Outcome::MultiObj { pareto_table, .. } = &report.outcome else {
        panic!("multi-objective outcome expected");
    };
    let dup = pareto_table.iter().find(|t| t.trial_number == 3).unwrap();
    assert_eq!(dup.duplicate_of, Some(1));
    let first = pareto_table.iter().find(|t| t.trial_number == 1).unwrap();
    assert_eq!(first.duplicate_of, None);

    let html = render_html(&report, ReportLang::En);
    assert!(html.contains("(= #1)"), "HTML の併記");
    assert!(html.contains("identical to trial #N"), "HTML の凡例");
    let md = render_markdown(&report, ReportLang::Ja);
    assert!(md.contains("#3 (= #1)"), "MD の併記: {md}");
    assert!(md.contains("重複解"), "MD の凡例");
}

#[test]
fn feasible_only_front_has_no_infeasible_mark_or_note() {
    let report = build_study_report(
        &meta_multi_constrained(),
        &df_multi_constrained(false),
        None,
        &source(),
        &opts(),
    );
    let html = render_html(&report, ReportLang::En);

    // The column itself is still present (constrained study).
    assert!(html.contains("max constraint (≤0 = feasible)"));
    // All trials feasible → neither the violation mark nor the note is
    // emitted (checked via the cell class, since `td.infeasible` is
    // always defined in PAGE_CSS).
    assert!(!html.contains("class=\"num infeasible\""), "違反マークなし");
    assert!(!html.contains("trials violate constraints"), "注記なし");

    let md = render_markdown(&report, ReportLang::En);
    assert!(!md.contains("(infeasible)"));
    assert!(!md.contains("trials violate constraints"));
}

#[test]
fn markdown_generated_at_includes_iso_utc() {
    let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
    let md = render_markdown(&report, ReportLang::En);
    // Includes ISO UTC alongside the unix seconds (deterministic conversion).
    assert!(
        md.contains("Generated at: 2023-11-14T22:13:20Z (unix 1700000000)"),
        "ISO UTC + unix 併記"
    );
}

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
    let report: StudyReport = build_study_report(&meta, &df, None, &source(), &opts());
    let en = render_html(&report, ReportLang::En);
    let ja = render_html(&report, ReportLang::Ja);
    assert_self_contained(&en);
    assert!(en.contains("id=\"key-findings\""));
    assert!(ja.contains("<html lang=\"ja\">"));
}
