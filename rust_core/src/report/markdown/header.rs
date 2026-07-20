//! Meta line and Key Findings section.

use std::fmt::Write as _;

use super::*;
use crate::report::model::{KeyFinding, StudyReport};
use crate::report::text::{self, format_unix_utc};
use crate::report::{format_number, ReportLang};

// =============================================================================
// Meta line
// =============================================================================

pub(super) fn render_meta_line(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let ov = &report.overview;
    let dirs: Vec<String> = ov
        .objective_names
        .iter()
        .zip(ov.directions.iter())
        .map(|(name, d)| format!("{} ({})", esc(name), dir_label(lang, *d)))
        .collect();
    let _ = writeln!(
        s,
        "- {}: {}",
        tr(lang, "Storage", "ストレージ"),
        esc(&report.source.storage_display)
    );
    if let Some(ts) = report.source.generated_at_unix {
        let _ = writeln!(
            s,
            "- {}: {} (unix {})",
            tr(lang, "Generated at", "生成日時"),
            format_unix_utc(ts),
            ts
        );
    }
    let _ = writeln!(
        s,
        "- {}: {}",
        tr(lang, "Objectives", "目的"),
        if dirs.is_empty() {
            "-".to_string()
        } else {
            dirs.join(", ")
        }
    );
    let _ = writeln!(
        s,
        "- {}: {} COMPLETE / {} {}",
        tr(lang, "Trials", "試行数"),
        ov.complete_trials,
        ov.total_trials,
        tr(lang, "total", "全体")
    );
    if let Some(w) = ov.wall_clock_seconds {
        let _ = writeln!(
            s,
            "- {}: {} s",
            tr(lang, "Wall-clock", "実測所要時間"),
            format_number(w)
        );
    }
    s.push('\n');
}

// =============================================================================
// Key Findings
// =============================================================================

pub(super) fn render_key_findings(s: &mut String, lang: ReportLang, findings: &[KeyFinding]) {
    let _ = writeln!(s, "## {}\n", tr(lang, "Key Findings", "まとめ"));
    if findings.is_empty() {
        let _ = writeln!(
            s,
            "{}\n",
            tr(lang, "_No findings._", "_まとめはありません。_")
        );
        return;
    }
    for f in findings {
        let _ = writeln!(s, "- {}", finding_sentence(lang, f));
    }
    s.push('\n');
}

/// Formats a Key Finding into a single sentence (the template is shared
/// via [`crate::report::text`]).
///
/// Emphasis spans are wrapped in Markdown `**...**`, and every span is made
/// inline-safe via [`esc`]. Markdown special characters (`* _ [ ] #` etc.)
/// in user-derived strings (parameter names, etc.) are made safe here. The
/// `#` inside template literals (e.g. `trial #N`) also becomes `\#`, but
/// this is harmless since it still renders as `#` in Markdown.
fn finding_sentence(lang: ReportLang, f: &KeyFinding) -> String {
    let mut out = String::new();
    for span in text::finding_spans(lang, f) {
        let body = esc(&span.text);
        if span.emphasis {
            let _ = write!(out, "**{body}**");
        } else {
            out.push_str(&body);
        }
    }
    out
}
