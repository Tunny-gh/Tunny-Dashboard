//! Report header (title + metadata list) and table of contents.

use std::fmt::Write as _;

use super::*;
use crate::report::model::StudyReport;
use crate::report::text::format_unix_utc;
use crate::report::{format_number, ReportLang};

pub(super) fn render_header(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let ov = &report.overview;
    let _ = writeln!(s, "<h1>{}</h1>", esc(&ov.name));
    s.push_str("<ul class=\"meta\">\n");

    meta_row(
        s,
        tr(lang, "Storage", "ストレージ"),
        &report.source.storage_display,
    );
    if let Some(ts) = report.source.generated_at_unix {
        meta_row(
            s,
            tr(lang, "Generated at", "生成日時"),
            &format_unix_utc(ts),
        );
    }
    let dirs: Vec<String> = ov
        .objective_names
        .iter()
        .zip(ov.directions.iter())
        .map(|(name, d)| format!("{} ({})", name, dir_label(lang, *d)))
        .collect();
    meta_row(
        s,
        tr(lang, "Objectives", "目的"),
        &if dirs.is_empty() {
            "-".to_string()
        } else {
            dirs.join(", ")
        },
    );
    meta_row(
        s,
        tr(lang, "Trials", "試行数"),
        &format!(
            "{} COMPLETE / {} {}",
            ov.complete_trials,
            ov.total_trials,
            tr(lang, "total", "全体")
        ),
    );
    if !ov.state_counts.is_empty() {
        let states: Vec<String> = ov
            .state_counts
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect();
        meta_row(s, tr(lang, "States", "状態内訳"), &states.join(", "));
    }
    if let Some(w) = ov.wall_clock_seconds {
        meta_row(
            s,
            tr(lang, "Wall-clock", "実測所要時間"),
            &format!("{} s", format_number(w)),
        );
    }
    s.push_str("</ul>\n");
}

pub(super) fn render_toc(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let mut items: Vec<(&str, String)> = Vec::new();
    items.push((
        "key-findings",
        tr(lang, "Key Findings", "まとめ").to_string(),
    ));
    items.push(("outcome", tr(lang, "Outcome", "最適化結果").to_string()));
    items.push(("convergence", tr(lang, "Convergence", "収束").to_string()));
    if report.importance.is_some() {
        items.push((
            "importance",
            tr(lang, "Parameter Importance", "パラメータ重要度").to_string(),
        ));
    }
    if !report.objective_stats.is_empty() {
        items.push((
            "objective-stats",
            tr(lang, "Objective Statistics", "目的値の統計").to_string(),
        ));
    }
    if report
        .correlations
        .as_ref()
        .is_some_and(|c| !c.params.is_empty())
    {
        items.push(("correlations", tr(lang, "Correlations", "相関").to_string()));
    }
    if report.mcdm.is_some() {
        items.push((
            "mcdm",
            tr(lang, "Decision Analysis (MCDM)", "意思決定分析（MCDM）").to_string(),
        ));
    }
    if report.execution.is_some() {
        items.push(("execution", tr(lang, "Execution", "実行情報").to_string()));
    }
    items.push(("appendix", tr(lang, "Appendix", "付録").to_string()));

    s.push_str("<nav class=\"toc\" aria-label=\"Contents\">\n");
    let _ = writeln!(
        s,
        "<div class=\"toc-title\">{}</div>",
        esc(tr(lang, "Contents", "目次"))
    );
    s.push_str("<ol>\n");
    for (id, label) in items {
        let _ = writeln!(s, "<li><a href=\"#{id}\">{}</a></li>", esc(&label));
    }
    s.push_str("</ol>\n</nav>\n");
}
