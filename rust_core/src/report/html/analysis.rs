//! Convergence, parameter importance, objective statistics, and
//! correlations sections.

use std::fmt::Write as _;

use super::*;
use crate::report::model::*;
use crate::report::svg::{self, HBarItem, HistBin, LinePoint};
use crate::report::{format_number, ReportLang};

// =============================================================================
// Convergence
// =============================================================================

pub(super) fn render_convergence(s: &mut String, lang: ReportLang, conv: &ConvergenceSection) {
    let _ = writeln!(
        s,
        "<h2 id=\"convergence\">{}</h2>",
        esc(tr(lang, "Convergence", "収束"))
    );
    let metric = match conv.metric {
        ConvergenceMetric::BestSoFar => tr(lang, "best-so-far objective", "best-so-far 目的値"),
        ConvergenceMetric::Hypervolume => tr(lang, "hypervolume", "ハイパーボリューム"),
    };
    let status = match conv.status {
        ConvergenceStatus::Converged => tr(lang, "converged", "収束"),
        ConvergenceStatus::StillImproving => tr(lang, "still improving", "改善中"),
        ConvergenceStatus::Insufficient => tr(lang, "insufficient data", "データ不足"),
    };

    s.push_str("<ul class=\"facts\">\n");
    fact_row(s, tr(lang, "Metric", "指標"), metric);
    fact_row(s, tr(lang, "Status", "判定"), status);
    if let Some(t) = conv.found_at_trial_number {
        fact_row(
            s,
            tr(lang, "Best found at trial", "best 発見 trial"),
            &format!("#{t}"),
        );
    }
    fact_row(
        s,
        tr(lang, "Improved in last 20%", "直近20%で改善"),
        yes_no(lang, conv.improved_in_last_20pct),
    );
    s.push_str("</ul>\n");

    if conv.series.is_empty() {
        return;
    }

    // Line chart (markers at best-update points).
    let points: Vec<LinePoint> = conv
        .series
        .iter()
        .map(|p| LinePoint {
            trial_number: p.trial_number as i64,
            value: p.value,
        })
        .collect();
    let mut marks = Vec::new();
    for i in 0..points.len() {
        if i == 0 || points[i].value != points[i - 1].value {
            marks.push(i);
        }
    }
    let chart = svg::line_chart(&points, &marks, CHART_W, 260.0);
    let _ = writeln!(
        s,
        "<figure>{chart}<figcaption>{} ({})</figcaption></figure>",
        esc(tr(lang, "Convergence curve", "収束カーブ")),
        esc(metric)
    );

    // Table of the last 10 points (inside <details>).
    let tail = if conv.series.len() > 10 {
        &conv.series[conv.series.len() - 10..]
    } else {
        &conv.series[..]
    };
    let _ = writeln!(
        s,
        "<details><summary>{}</summary>",
        esc(tr(lang, "Last 10 sampled points", "収束系列の末尾10点"))
    );
    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "trial", "trial"), true);
    th(s, tr(lang, "value", "値"), true);
    s.push_str("</tr></thead>\n<tbody>\n");
    for p in tail {
        s.push_str("<tr>");
        td(s, &format!("#{}", p.trial_number), true);
        td(s, &format_number(p.value), true);
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);
    s.push_str("</details>\n");
}

// =============================================================================
// Importance
// =============================================================================

pub(super) fn render_importance(
    s: &mut String,
    lang: ReportLang,
    importance: Option<&ImportanceSection>,
) {
    let Some(sec) = importance else {
        return;
    };
    let _ = writeln!(
        s,
        "<h2 id=\"importance\">{}</h2>",
        esc(tr(lang, "Parameter Importance", "パラメータ重要度"))
    );
    let _ = writeln!(
        s,
        "<p class=\"desc\">{} <code>{}</code> {} <code>{}</code>. {}</p>",
        esc(tr(lang, "Method:", "手法:")),
        esc(&sec.method),
        esc(tr(lang, "against objective", "評価対象の目的:")),
        esc(&sec.objective_name),
        esc(tr(
            lang,
            "Higher score means more influential.",
            "スコアが大きいほど影響が大きい。"
        ))
    );

    if !sec.scores.is_empty() {
        let items: Vec<HBarItem> = sec
            .scores
            .iter()
            .map(|(name, score)| HBarItem {
                label: name.clone(),
                value: *score,
            })
            .collect();
        let chart = svg::hbar_chart(&items, CHART_W);
        let _ = writeln!(s, "<figure>{chart}</figure>");
    }

    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "parameter", "パラメータ"), false);
    th(s, tr(lang, "score", "スコア"), true);
    s.push_str("</tr></thead>\n<tbody>\n");
    for (name, score) in &sec.scores {
        s.push_str("<tr>");
        td(s, name, false);
        td(s, &format_number(*score), true);
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);
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
        "<h2 id=\"objective-stats\">{}</h2>",
        esc(tr(lang, "Objective Statistics", "目的値の統計"))
    );
    let _ = writeln!(
        s,
        "<p class=\"desc\">{}</p>",
        esc(tr(
            lang,
            "Distribution of completed objective values (non-finite values excluded from n).",
            "COMPLETE の目的値分布（非有限値は n から除外）。"
        ))
    );

    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "objective", "目的"), false);
    th(s, tr(lang, "direction", "方向"), false);
    for h in ["n", "mean", "std", "min", "q1", "median", "q3", "max"] {
        th(s, h, true);
    }
    s.push_str("</tr></thead>\n<tbody>\n");
    for st in stats {
        s.push_str("<tr>");
        td(s, &st.name, false);
        td(s, dir_label(lang, st.direction), false);
        td(s, &st.n.to_string(), true);
        td(s, &format_number(st.mean), true);
        td(s, &format_number(st.std), true);
        td(s, &format_number(st.min), true);
        td(s, &format_number(st.q1), true);
        td(s, &format_number(st.median), true);
        td(s, &format_number(st.q3), true);
        td(s, &format_number(st.max), true);
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);

    // Histograms (up to MAX_HISTOGRAMS charts).
    let mut shown = 0usize;
    for st in stats {
        if shown >= MAX_HISTOGRAMS {
            break;
        }
        let Some(h) = &st.histogram else {
            continue;
        };
        if h.counts.is_empty() || h.bin_edges.len() != h.counts.len() + 1 {
            continue;
        }
        let bins: Vec<HistBin> = (0..h.counts.len())
            .map(|i| HistBin {
                lower: h.bin_edges[i],
                upper: h.bin_edges[i + 1],
                count: h.counts[i] as u64,
            })
            .collect();
        let chart = svg::histogram(&bins, CHART_W, 220.0);
        let _ = writeln!(
            s,
            "<figure>{chart}<figcaption>{}: {}</figcaption></figure>",
            esc(tr(lang, "Distribution", "分布")),
            esc(&st.name)
        );
        shown += 1;
    }
}

// =============================================================================
// Correlations
// =============================================================================

pub(super) fn render_correlations(
    s: &mut String,
    lang: ReportLang,
    correlations: Option<&CorrelationSection>,
) {
    let Some(sec) = correlations else {
        return;
    };
    if sec.params.is_empty() {
        return;
    }
    let _ = writeln!(
        s,
        "<h2 id=\"correlations\">{}</h2>",
        esc(tr(lang, "Correlations", "相関"))
    );
    let _ = writeln!(
        s,
        "<p class=\"desc\">{} <code>{}</code>. {}</p>",
        esc(tr(lang, "Method:", "手法:")),
        esc(&sec.method),
        esc(tr(
            lang,
            "Rank correlation between each parameter (row) and objective (column); parameters capped by max |ρ|.",
            "各セルはパラメータ（行）と目的（列）の順位相関。パラメータは max |ρ| で cap。"
        ))
    );

    let chart = svg::heatmap(&sec.matrix, &sec.params, &sec.objectives, CHART_W);
    let _ = writeln!(s, "<figure>{chart}</figure>");

    let _ = writeln!(
        s,
        "<details><summary>{}</summary>",
        esc(tr(lang, "Full correlation table", "相関表（全体）"))
    );
    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "parameter", "パラメータ"), false);
    for o in &sec.objectives {
        th(s, o, true);
    }
    s.push_str("</tr></thead>\n<tbody>\n");
    for (i, name) in sec.params.iter().enumerate() {
        s.push_str("<tr>");
        td(s, name, false);
        for v in &sec.matrix[i] {
            td(s, &format_number(*v), true);
        }
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);
    s.push_str("</details>\n");
}
