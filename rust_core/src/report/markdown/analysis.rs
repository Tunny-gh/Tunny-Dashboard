//! Convergence, parameter importance, objective statistics, and
//! correlations sections.

use std::fmt::Write as _;

use super::*;
use crate::report::builder::downsample;
use crate::report::model::*;
use crate::report::{format_number, ReportLang};

// =============================================================================
// Convergence
// =============================================================================

pub(super) fn render_convergence(s: &mut String, lang: ReportLang, conv: &ConvergenceSection) {
    let _ = writeln!(s, "## {}\n", tr(lang, "Convergence", "収束"));
    let metric = match conv.metric {
        ConvergenceMetric::BestSoFar => tr(lang, "best-so-far objective", "best-so-far 目的値"),
        ConvergenceMetric::Hypervolume => tr(lang, "hypervolume", "ハイパーボリューム"),
    };
    let status = match conv.status {
        ConvergenceStatus::Converged => tr(lang, "converged", "収束"),
        ConvergenceStatus::StillImproving => tr(lang, "still improving", "改善中"),
        ConvergenceStatus::Insufficient => tr(lang, "insufficient data", "データ不足"),
    };
    let _ = writeln!(s, "- {}: {}", tr(lang, "Metric", "指標"), metric);
    let _ = writeln!(s, "- {}: {}", tr(lang, "Status", "判定"), status);
    if let Some(t) = conv.found_at_trial_number {
        let _ = writeln!(
            s,
            "- {}: #{}",
            tr(lang, "Best found at trial", "best 発見 trial"),
            t
        );
    }
    let _ = writeln!(
        s,
        "- {}: {}",
        tr(lang, "Improved in last 20%", "直近20%で改善"),
        yes_no(lang, conv.improved_in_last_20pct)
    );
    s.push('\n');

    if conv.series.is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        "{}\n",
        tr(
            lang,
            "Sampled convergence series (trial number → metric value):",
            "収束系列のサンプル（trial 番号 → 指標値）:"
        )
    );
    let sampled = downsample(&conv.series, 20);
    let _ = writeln!(
        s,
        "| {} | {} |",
        tr(lang, "trial", "trial"),
        tr(lang, "value", "値")
    );
    let _ = writeln!(s, "|---|---|");
    for p in &sampled {
        let _ = writeln!(s, "| #{} | {} |", p.trial_number, format_number(p.value));
    }
    s.push('\n');
}

// =============================================================================
// Importance
// =============================================================================

pub(super) fn render_importance(
    s: &mut String,
    lang: ReportLang,
    importance: &Option<ImportanceSection>,
) {
    let Some(sec) = importance else {
        return;
    };
    let _ = writeln!(
        s,
        "## {}\n",
        tr(lang, "Parameter Importance", "パラメータ重要度")
    );
    let _ = writeln!(
        s,
        "{} {} {} {}. {}\n",
        tr(lang, "Method:", "手法:"),
        code_span(&sec.method),
        tr(lang, "against objective", "評価対象の目的:"),
        code_span(&sec.objective_name),
        tr(
            lang,
            "Scores are sorted descending; higher means more influential.",
            "スコアは降順で、大きいほど影響が大きい。"
        )
    );
    let _ = writeln!(
        s,
        "| {} | {} |",
        tr(lang, "parameter", "パラメータ"),
        tr(lang, "score", "スコア")
    );
    let _ = writeln!(s, "|---|---|");
    for (name, score) in &sec.scores {
        let _ = writeln!(s, "| {} | {} |", esc(name), format_number(*score));
    }
    s.push('\n');
}

// =============================================================================
// Objective statistics
// =============================================================================

pub(super) fn render_objective_stats(s: &mut String, lang: ReportLang, stats: &[ObjectiveStats]) {
    if stats.is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        "## {}\n",
        tr(lang, "Objective Statistics", "目的値の統計")
    );
    let _ = writeln!(
        s,
        "{}\n",
        tr(
            lang,
            "Distribution of completed objective values (non-finite values excluded from n):",
            "COMPLETE の目的値分布（非有限値は n から除外）:"
        )
    );
    let _ = writeln!(
        s,
        "| {} | {} | n | mean | std | min | q1 | median | q3 | max |",
        tr(lang, "objective", "目的"),
        tr(lang, "direction", "方向")
    );
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|---|");
    for st in stats {
        let _ = writeln!(
            s,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            esc(&st.name),
            dir_label(lang, st.direction),
            st.n,
            format_number(st.mean),
            format_number(st.std),
            format_number(st.min),
            format_number(st.q1),
            format_number(st.median),
            format_number(st.q3),
            format_number(st.max),
        );
    }
    s.push('\n');
}

// =============================================================================
// Correlations
// =============================================================================

pub(super) fn render_correlations(
    s: &mut String,
    lang: ReportLang,
    correlations: &Option<CorrelationSection>,
) {
    let Some(sec) = correlations else {
        return;
    };
    if sec.params.is_empty() {
        return;
    }
    let _ = writeln!(s, "## {}\n", tr(lang, "Correlations", "相関"));
    let _ = writeln!(
        s,
        "{} {}. {}\n",
        tr(lang, "Method:", "手法:"),
        code_span(&sec.method),
        tr(
            lang,
            "Each cell is the rank correlation between a parameter (row) and an objective (column); parameters capped by max |ρ|.",
            "各セルはパラメータ（行）と目的（列）の順位相関。パラメータは max |ρ| で cap。"
        )
    );
    let mut header = format!("| {} |", tr(lang, "parameter", "パラメータ"));
    for o in &sec.objectives {
        let _ = write!(header, " {} |", esc(o));
    }
    let _ = writeln!(s, "{header}");
    let cols = 1 + sec.objectives.len();
    let sep: String = std::iter::repeat_n("---", cols)
        .collect::<Vec<_>>()
        .join("|");
    let _ = writeln!(s, "|{sep}|");
    for (i, name) in sec.params.iter().enumerate() {
        let mut row = format!("| {} |", esc(name));
        for v in &sec.matrix[i] {
            let _ = write!(row, " {} |", format_number(*v));
        }
        let _ = writeln!(s, "{row}");
    }
    s.push('\n');
}
