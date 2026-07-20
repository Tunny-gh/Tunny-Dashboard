//! Outcome section: best trial / top trials (single-objective) or Pareto
//! front table (multi-objective).

use std::fmt::Write as _;

use super::*;
use crate::report::model::*;
use crate::report::text;
use crate::report::{format_number, ReportLang};

pub(super) fn render_outcome(s: &mut String, lang: ReportLang, report: &StudyReport) {
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
