//! Markdown renderer (primarily for LLM consumption).
//!
//! Structure: `# Optimization Report: {study}` -> `## Key Findings` (bullet
//! list) -> each section as `##` + a pipe table. No charts; tables are used
//! instead. Output is deterministic (identical input -> byte-identical
//! output) and does not depend on `HashMap` iteration order (the model
//! already holds data in `BTreeMap` / sorted `Vec`).
//!
//! The language (en / ja) is selected at render time. Templates avoid
//! speculation or exaggeration and only phrase facts already present in the
//! model.

use std::fmt::Write as _;

use super::builder::downsample;
use super::model::*;
use super::text::{self, format_unix_utc};
use super::{format_number, pct, ReportLang};

/// Renders a [`StudyReport`] to Markdown.
pub fn render_markdown(report: &StudyReport, lang: ReportLang) -> String {
    let mut s = String::new();

    // Title.
    let _ = writeln!(
        s,
        "# {}: {}",
        tr(lang, "Optimization Report", "最適化レポート"),
        esc(&report.overview.name)
    );
    s.push('\n');

    render_meta_line(&mut s, lang, report);
    render_key_findings(&mut s, lang, &report.key_findings);
    render_outcome(&mut s, lang, report);
    render_convergence(&mut s, lang, &report.convergence);
    render_importance(&mut s, lang, &report.importance);
    render_objective_stats(&mut s, lang, &report.objective_stats);
    render_correlations(&mut s, lang, &report.correlations);
    render_mcdm(&mut s, lang, &report.mcdm, &report.overview.objective_names);
    render_execution(&mut s, lang, &report.execution);
    render_reproduction(&mut s, lang, report);

    s
}

// =============================================================================
// Language / escape helpers
// =============================================================================

/// Returns either the en or ja string depending on the language.
fn tr(lang: ReportLang, en: &'static str, ja: &'static str) -> &'static str {
    match lang {
        ReportLang::En => en,
        ReportLang::Ja => ja,
    }
}

/// Escapes a user-derived string to be safe as a Markdown table cell / safe
/// for inline use (used for both table cells and inline body text).
///
/// - `\` -> `\\`, `|` -> `\|`: protects the table structure. Processed in a
///   single pass character by character, so it avoids the ordering issue
///   that a sequential `replace` chain would have (a later replacement
///   double-escaping a `\` inserted by an earlier one).
/// - `&` -> `&amp;`, `<` -> `&lt;`: prevents XSS / structure breakage when
///   the output Markdown is converted to HTML downstream (e.g. rendered via
///   MCP). In Markdown these still display as the literal character via the
///   entity reference.
/// - `* _ [ ] #` -> `\*` etc.: prevents misinterpretation as Markdown
///   inline syntax (emphasis, links, headings), including user-derived
///   spans in Key Findings.
/// - Newlines (`\n` / `\r`) -> space: protects cell / row structure.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '*' => out.push_str("\\*"),
            '_' => out.push_str("\\_"),
            '[' => out.push_str("\\["),
            ']' => out.push_str("\\]"),
            '#' => out.push_str("\\#"),
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// Formats a user-derived name as an inline code span.
///
/// Backslash escaping has no effect inside inline code, so [`esc`] is not
/// applied; instead embedded backticks are handled per the CommonMark
/// rule: wrap the content in a run of backticks one longer than the
/// longest backtick run it contains, and pad with a space if the content
/// starts or ends with a backtick (the parser strips one space from each
/// end). Newlines are replaced with spaces.
fn code_span(s: &str) -> String {
    let content: String = s
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let max_run = content.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(max_run + 1);
    if content.is_empty() {
        // An empty code span is not valid Markdown, so use a single space.
        return "` `".to_string();
    }
    if content.starts_with('`') || content.ends_with('`') {
        format!("{fence} {content} {fence}")
    } else {
        format!("{fence}{content}{fence}")
    }
}

/// Direction label.
fn dir_label(lang: ReportLang, d: Direction) -> &'static str {
    match d {
        Direction::Minimize => tr(lang, "Minimize", "最小化"),
        Direction::Maximize => tr(lang, "Maximize", "最大化"),
    }
}

fn param_val(v: &ParamValue) -> String {
    match v {
        ParamValue::Num(x) => format_number(*x),
        ParamValue::Cat(s) => esc(s),
    }
}

// =============================================================================
// Meta line
// =============================================================================

fn render_meta_line(s: &mut String, lang: ReportLang, report: &StudyReport) {
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

fn render_key_findings(s: &mut String, lang: ReportLang, findings: &[KeyFinding]) {
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
/// via [`super::text`]).
///
/// Emphasis spans are wrapped in Markdown `**...**`, and every span is made
/// inline-safe via [`esc`]. Markdown special characters (`* _ [ ] #` etc.)
/// in user-derived strings (parameter names, etc.) are made safe here. The
/// `#` inside template literals (e.g. `trial #N`) also becomes `\#`, but
/// this is harmless since it still renders as `#` in Markdown.
fn finding_sentence(lang: ReportLang, f: &KeyFinding) -> String {
    let mut out = String::new();
    for span in super::text::finding_spans(lang, f) {
        let body = esc(&span.text);
        if span.emphasis {
            let _ = write!(out, "**{body}**");
        } else {
            out.push_str(&body);
        }
    }
    out
}

// =============================================================================
// Outcome
// =============================================================================

fn render_outcome(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let _ = writeln!(s, "## {}\n", tr(lang, "Outcome", "最適化結果"));
    let obj_names = &report.overview.objective_names;
    let has_constraints = report.overview.has_constraints;
    match &report.outcome {
        Outcome::SingleObj { best_trial, top_n } => {
            if let Some(bt) = best_trial {
                let _ = writeln!(s, "{}\n", tr(lang, "Best trial:", "最良 trial:"));
                render_trial_table(
                    s,
                    lang,
                    std::slice::from_ref(bt),
                    obj_names,
                    has_constraints,
                );
            }
            let _ = writeln!(
                s,
                "{}\n",
                tr(
                    lang,
                    "Top trials (best first; objective and parameter columns):",
                    "上位 trial（最良順。目的とパラメータの列）:"
                )
            );
            render_trial_table(s, lang, top_n, obj_names, has_constraints);
        }
        Outcome::MultiObj {
            pareto_size,
            complete_count,
            objective_count,
            per_objective_extremes,
            pareto_table,
            pareto_infeasible_count,
            scatter,
            scatter_axes,
        } => {
            let _ = writeln!(
                s,
                "{} {} / {} COMPLETE.\n",
                tr(lang, "Pareto front size:", "パレート前面サイズ:"),
                pareto_size,
                complete_count
            );

            // Per-objective extremes.
            let _ = writeln!(
                s,
                "{}\n",
                tr(
                    lang,
                    "Per-objective extremes (best value respects each objective's direction):",
                    "目的ごとの極値（最良値は各目的の方向に従う）:"
                )
            );
            let _ = writeln!(
                s,
                "| {} | {} | {} | {} | {} |",
                tr(lang, "objective", "目的"),
                tr(lang, "direction", "方向"),
                tr(lang, "best", "最良"),
                tr(lang, "best trial", "最良 trial"),
                tr(lang, "worst", "最悪")
            );
            let _ = writeln!(s, "|---|---|---|---|---|");
            for e in per_objective_extremes {
                // Add an explicit mark when the best trial violates constraints.
                let infeasible_mark = if e.best_feasible {
                    ""
                } else {
                    tr(lang, " (infeasible)", "（違反）")
                };
                let _ = writeln!(
                    s,
                    "| {} | {} | {} | #{}{} | {} |",
                    esc(&e.objective_name),
                    dir_label(lang, e.direction),
                    format_number(e.best_value),
                    e.best_trial_number,
                    infeasible_mark,
                    format_number(e.worst_value)
                );
            }
            s.push('\n');

            // Pareto table (ordered by TOPSIS).
            let _ = writeln!(
                s,
                "{}\n",
                tr(
                    lang,
                    "Pareto-front trials, ordered by equal-weight TOPSIS (capped):",
                    "パレート前面の trial（等重み TOPSIS 順、cap 済み）:"
                )
            );
            render_trial_table(s, lang, pareto_table, obj_names, has_constraints);

            // The front is computed from feasible rows only, so an
            // infeasible trial appears in the table only in the fallback
            // case where there are no feasible solutions at all. The count
            // is already aggregated by the builder over the full
            // pre-cap front.
            if *pareto_infeasible_count > 0 {
                let _ = writeln!(
                    s,
                    "{}\n",
                    text::infeasible_fallback_note(lang, *pareto_infeasible_count)
                );
            }
            render_duplicate_note(s, lang, pareto_table);

            if *objective_count > 2 {
                let _ = writeln!(
                    s,
                    "{} (axes {}/{}, {} {}).\n",
                    tr(
                        lang,
                        "Note: scatter uses the first two objectives",
                        "注記: 散布図は先頭2目的を使用"
                    ),
                    scatter_axes.0,
                    scatter_axes.1,
                    objective_count,
                    tr(lang, "objectives total", "目的中")
                );
            }
            // If there are constraint-violating points, note the count
            // (the points themselves include all COMPLETE trials).
            let n_scatter_infeasible = scatter.iter().filter(|p| !p.feasible).count();
            if n_scatter_infeasible > 0 {
                let _ = writeln!(
                    s,
                    "{} {} ({} {}).\n",
                    tr(lang, "Scatter points:", "散布図の点数:"),
                    scatter.len(),
                    n_scatter_infeasible,
                    tr(lang, "infeasible", "件は制約違反")
                );
            } else {
                let _ = writeln!(
                    s,
                    "{} {}.\n",
                    tr(lang, "Scatter points:", "散布図の点数:"),
                    scatter.len()
                );
            }
        }
    }
}

/// Emits a one-line legend if the table contains duplicate solutions
/// (trials with `duplicate_of` set).
fn render_duplicate_note(s: &mut String, lang: ReportLang, trials: &[TrialSummary]) {
    if has_duplicate_marks(trials) {
        let _ = writeln!(s, "{}\n", text::duplicate_legend_note(lang));
    }
}

/// Emits a table of `TrialSummary`s (trial# + objectives + parameters
/// [+ max constraint value]).
///
/// `user_attrs` is intentionally not output here (to keep the LLM-facing
/// report concise). If user-attached info is needed, it can be found in the
/// HTML renderer's appendix.
fn render_trial_table(
    s: &mut String,
    lang: ReportLang,
    trials: &[TrialSummary],
    obj_names: &[String],
    show_constraint: bool,
) {
    if trials.is_empty() {
        let _ = writeln!(
            s,
            "{}\n",
            tr(lang, "_No trials._", "_該当する trial はありません。_")
        );
        return;
    }
    let param_names: Vec<String> = trials[0].params.iter().map(|(n, _)| n.clone()).collect();

    // Header.
    let mut header = format!("| {} |", tr(lang, "trial", "trial"));
    for o in obj_names {
        let _ = write!(header, " {} |", esc(o));
    }
    for p in &param_names {
        let _ = write!(header, " {} |", esc(p));
    }
    if show_constraint {
        let _ = write!(
            header,
            " {} |",
            tr(
                lang,
                "max constraint (≤0 = feasible)",
                "最大制約値（≤0 で充足）"
            )
        );
    }
    let _ = writeln!(s, "{header}");

    let cols = 1 + obj_names.len() + param_names.len() + usize::from(show_constraint);
    let sep: String = std::iter::repeat_n("---", cols)
        .collect::<Vec<_>>()
        .join("|");
    let _ = writeln!(s, "|{sep}|");

    for t in trials {
        let mut row = match t.duplicate_of {
            // For duplicate solutions with identical objective values, note the trial number of the first occurrence.
            Some(first) => format!("| #{} (= #{first}) |", t.trial_number),
            None => format!("| #{} |", t.trial_number),
        };
        for (i, _) in obj_names.iter().enumerate() {
            let v = t.objectives.get(i).copied().unwrap_or(f64::NAN);
            let _ = write!(row, " {} |", format_number(v));
        }
        for (_, v) in &t.params {
            let _ = write!(row, " {} |", param_val(v));
        }
        if show_constraint {
            let c = match t.max_constraint {
                // Positive value = constraint violation (the check is shared on the model side). Add an explicit mark.
                Some(v) if t.violates_constraints() => format!(
                    "{}{}",
                    format_number(v),
                    tr(lang, " (infeasible)", "（違反）")
                ),
                Some(v) => format_number(v),
                None => "-".to_string(),
            };
            let _ = write!(row, " {c} |");
        }
        let _ = writeln!(s, "{row}");
    }
    s.push('\n');
}

// =============================================================================
// Convergence
// =============================================================================

fn render_convergence(s: &mut String, lang: ReportLang, conv: &ConvergenceSection) {
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

fn yes_no(lang: ReportLang, b: bool) -> &'static str {
    if b {
        tr(lang, "yes", "はい")
    } else {
        tr(lang, "no", "いいえ")
    }
}

// =============================================================================
// Importance
// =============================================================================

fn render_importance(s: &mut String, lang: ReportLang, importance: &Option<ImportanceSection>) {
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

fn render_objective_stats(s: &mut String, lang: ReportLang, stats: &[ObjectiveStats]) {
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

fn render_correlations(
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

// =============================================================================
// MCDM
// =============================================================================

fn render_mcdm(s: &mut String, lang: ReportLang, mcdm: &Option<McdmSection>, obj_names: &[String]) {
    let Some(sec) = mcdm else {
        return;
    };
    let _ = writeln!(
        s,
        "## {}\n",
        tr(lang, "Decision Analysis (MCDM)", "意思決定分析（MCDM）")
    );
    let weights: Vec<String> = sec.weights.iter().map(|w| format_number(*w)).collect();
    let _ = writeln!(
        s,
        "{} {} ({}: {}). {}\n",
        tr(lang, "Weighting:", "重み付け:"),
        code_span(&sec.weight_scheme),
        tr(lang, "weights", "重み"),
        weights.join(", "),
        tr(
            lang,
            "Rankings are computed on the Pareto front only.",
            "ランキングはパレート前面のみを対象に計算する。"
        )
    );

    render_mcdm_table(s, lang, "TOPSIS", &sec.topsis_top, obj_names);
    render_mcdm_table(s, lang, "VIKOR", &sec.vikor_top, obj_names);
    render_mcdm_table(s, lang, "PROMETHEE II", &sec.promethee_top, obj_names);

    let consensus: Vec<String> = sec
        .consensus_trials
        .iter()
        .map(|t| format!("#{t}"))
        .collect();
    let _ = writeln!(
        s,
        "{} {}\n",
        tr(
            lang,
            "Consensus (trials in the top-10 of all three methods):",
            "コンセンサス（3手法すべての top10 に入る trial）:"
        ),
        if consensus.is_empty() {
            tr(lang, "none", "なし").to_string()
        } else {
            consensus.join(", ")
        }
    );
}

fn render_mcdm_table(
    s: &mut String,
    lang: ReportLang,
    method: &str,
    entries: &[McdmEntry],
    obj_names: &[String],
) {
    let _ = writeln!(
        s,
        "{} {} ({}):\n",
        tr(lang, "Top by", "上位:"),
        method,
        tr(lang, "rank / trial / objectives", "順位 / trial / 目的値")
    );
    if entries.is_empty() {
        let _ = writeln!(s, "{}\n", tr(lang, "_No entries._", "_該当なし。_"));
        return;
    }
    let mut header = format!(
        "| {} | {} |",
        tr(lang, "rank", "順位"),
        tr(lang, "trial", "trial")
    );
    for o in obj_names {
        let _ = write!(header, " {} |", esc(o));
    }
    let _ = writeln!(s, "{header}");
    let cols = 2 + obj_names.len();
    let sep: String = std::iter::repeat_n("---", cols)
        .collect::<Vec<_>>()
        .join("|");
    let _ = writeln!(s, "|{sep}|");
    for e in entries {
        let mut row = format!("| {} | #{} |", e.rank, e.trial_number);
        for v in &e.objectives {
            let _ = write!(row, " {} |", format_number(*v));
        }
        let _ = writeln!(s, "{row}");
    }
    s.push('\n');
}

// =============================================================================
// Execution
// =============================================================================

fn render_execution(s: &mut String, lang: ReportLang, execution: &Option<ExecutionSection>) {
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

fn render_reproduction(s: &mut String, lang: ReportLang, report: &StudyReport) {
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

#[cfg(test)]
mod esc_tests {
    use super::{code_span, esc};

    #[test]
    fn escapes_backslash_before_pipe() {
        // Because processing is single-pass, escaping `\` and `|` in
        // `a\|b` does not collide, and the table structure is not broken.
        assert_eq!(esc("a\\|b"), "a\\\\\\|b");
        assert_eq!(esc("trail\\"), "trail\\\\");
    }

    #[test]
    fn escapes_html_amp_and_lt() {
        // Guards against XSS / structure breakage when converted to HTML downstream.
        assert_eq!(esc("a&b"), "a&amp;b");
        assert_eq!(esc("<script>"), "&lt;script>");
        // `&` is turned into an entity reference as-is and is not
        // double-escaped (since processing is single-pass, `&amp;` is not
        // reprocessed).
        assert_eq!(esc("&lt;"), "&amp;lt;");
    }

    #[test]
    fn escapes_markdown_inline_specials() {
        // Not misinterpreted as emphasis, link, or heading syntax.
        assert_eq!(esc("*em*"), "\\*em\\*");
        assert_eq!(esc("snake_case_name"), "snake\\_case\\_name");
        assert_eq!(esc("[link]"), "\\[link\\]");
        assert_eq!(esc("#tag"), "\\#tag");
    }

    #[test]
    fn newlines_become_spaces() {
        assert_eq!(esc("a\nb\rc"), "a b c");
    }

    #[test]
    fn code_span_wraps_plain_names() {
        assert_eq!(code_span("spearman_abs"), "`spearman_abs`");
    }

    #[test]
    fn code_span_handles_embedded_backticks() {
        // Wraps with a fence one longer than the content's longest backtick run.
        assert_eq!(code_span("a`b"), "``a`b``");
        assert_eq!(code_span("a``b"), "```a``b```");
        // Pads with a space when the content starts/ends with a backtick.
        assert_eq!(code_span("`lead"), "`` `lead ``");
        assert_eq!(code_span("trail`"), "`` trail` ``");
        // An empty string becomes a code span containing a single space.
        assert_eq!(code_span(""), "` `");
    }
}
