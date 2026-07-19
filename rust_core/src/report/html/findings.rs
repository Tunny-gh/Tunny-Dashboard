//! Key Findings section.

use std::fmt::Write as _;

use super::*;
use crate::report::model::{FindingKind, KeyFinding};
use crate::report::text;
use crate::report::ReportLang;

pub(super) fn render_findings(s: &mut String, lang: ReportLang, findings: &[KeyFinding]) {
    let _ = writeln!(
        s,
        "<h2 id=\"key-findings\">{}</h2>",
        esc(tr(lang, "Key Findings", "まとめ"))
    );
    if findings.is_empty() {
        let _ = writeln!(
            s,
            "<p class=\"muted\">{}</p>",
            esc(tr(lang, "No findings.", "まとめはありません。"))
        );
        return;
    }
    for f in findings {
        let _ = writeln!(
            s,
            "<div class=\"finding\"><span class=\"badge\">{}</span>{}</div>",
            esc(finding_badge(lang, f.kind)),
            finding_html(lang, f)
        );
    }
}

/// Formats a Key Finding into an escaped HTML string (template shared with `text`).
fn finding_html(lang: ReportLang, f: &KeyFinding) -> String {
    let mut out = String::new();
    for span in text::finding_spans(lang, f) {
        let body = esc(&span.text);
        if span.emphasis {
            let _ = write!(out, "<strong>{body}</strong>");
        } else {
            out.push_str(&body);
        }
    }
    out
}

fn finding_badge(lang: ReportLang, kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::BestSingle => tr(lang, "Best", "最良"),
        FindingKind::ParetoSummary => tr(lang, "Pareto", "パレート"),
        FindingKind::ConvergenceStatus => tr(lang, "Convergence", "収束"),
        FindingKind::TopImportance => tr(lang, "Importance", "重要度"),
        FindingKind::TradeOff => tr(lang, "Trade-off", "トレードオフ"),
        FindingKind::Feasibility => tr(lang, "Feasibility", "実行可能性"),
        FindingKind::PruningEfficiency => tr(lang, "Pruning", "枝刈り"),
        FindingKind::DataQuality => tr(lang, "Data quality", "データ品質"),
    }
}
