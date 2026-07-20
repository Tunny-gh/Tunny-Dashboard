//! Execution info and reproduction info sections.

use std::fmt::Write as _;

use super::*;
use crate::report::model::{ExecutionSection, StudyReport};
use crate::report::{format_number, pct, ReportLang};

// =============================================================================
// Execution
// =============================================================================

pub(super) fn render_execution(
    s: &mut String,
    lang: ReportLang,
    execution: &Option<ExecutionSection>,
) {
    let Some(sec) = execution else {
        return;
    };
    let _ = writeln!(s, "## {}\n", tr(lang, "Execution", "実行情報"));

    let _ = writeln!(
        s,
        "| {} | {} |",
        tr(lang, "state", "state"),
        tr(lang, "count", "件数")
    );
    let _ = writeln!(s, "|---|---|");
    for (state, count) in &sec.state_counts {
        let _ = writeln!(s, "| {} | {} |", esc(state), count);
    }
    s.push('\n');

    let _ = writeln!(
        s,
        "- {}: {}%",
        tr(lang, "Pruned rate", "枝刈り率"),
        pct(sec.pruned_rate * 100.0)
    );
    if let Some(step) = sec.median_prune_step {
        let _ = writeln!(
            s,
            "- {}: {}",
            tr(lang, "Median prune step", "枝刈り step 中央値"),
            format_number(step)
        );
    }
    if let (Some(mean), Some(std)) = (sec.mean_trial_seconds, sec.std_trial_seconds) {
        let _ = writeln!(
            s,
            "- {}: {} ± {} s",
            tr(lang, "Mean trial time", "平均 trial 時間"),
            format_number(mean),
            format_number(std)
        );
    }
    if let Some(total) = sec.total_seconds {
        let _ = writeln!(
            s,
            "- {}: {} s",
            tr(lang, "Total time", "総所要時間"),
            format_number(total)
        );
    }
    s.push('\n');
}

// =============================================================================
// Reproduction
// =============================================================================

pub(super) fn render_reproduction(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let r = &report.reproduction;
    let _ = writeln!(s, "## {}\n", tr(lang, "Reproduction", "再現情報"));
    let _ = writeln!(s, "- study_id: {}", r.study_id);
    let _ = writeln!(
        s,
        "- {}: {}",
        tr(lang, "storage (masked)", "ストレージ（マスク済み）"),
        esc(&r.storage_display)
    );
    let _ = writeln!(s, "- top_n: {}", r.top_n);
    let _ = writeln!(s, "- max_heatmap_params: {}", r.max_heatmap_params);
    let _ = writeln!(s, "- schema_version: {}", r.schema_version);
    s.push('\n');
}
