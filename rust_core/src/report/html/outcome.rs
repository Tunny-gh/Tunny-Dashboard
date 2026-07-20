//! Outcome section: best trial / top trials (single-objective) or Pareto
//! front (multi-objective), including the objective-space scatter chart.

use std::fmt::Write as _;

use super::*;
use crate::report::model::*;
use crate::report::svg::{self, ScatterPoint};
use crate::report::text;
use crate::report::{format_number, ReportLang};

pub(super) fn render_outcome(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let _ = writeln!(
        s,
        "<h2 id=\"outcome\">{}</h2>",
        esc(tr(lang, "Outcome", "最適化結果"))
    );
    let obj_names = &report.overview.objective_names;
    let has_constraints = report.overview.has_constraints;

    match &report.outcome {
        Outcome::SingleObj { best_trial, top_n } => {
            render_outcome_single(
                s,
                lang,
                best_trial.as_ref(),
                top_n,
                obj_names,
                has_constraints,
            );
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
                "<p>{} <strong>{}</strong> / {} COMPLETE.</p>",
                esc(tr(lang, "Pareto front size:", "パレート前面サイズ:")),
                pareto_size,
                complete_count
            );
            render_extremes_table(s, lang, per_objective_extremes);
            render_outcome_scatter(s, lang, scatter, *scatter_axes, *objective_count, obj_names);
            render_pareto_table_block(
                s,
                lang,
                pareto_table,
                *pareto_infeasible_count,
                obj_names,
                has_constraints,
            );
        }
    }
}

/// Single-objective Outcome (best trial + top-trials table).
fn render_outcome_single(
    s: &mut String,
    lang: ReportLang,
    best_trial: Option<&TrialSummary>,
    top_n: &[TrialSummary],
    obj_names: &[String],
    has_constraints: bool,
) {
    if let Some(bt) = best_trial {
        let _ = writeln!(s, "<h3>{}</h3>", esc(tr(lang, "Best trial", "最良 trial")));
        render_trial_table(
            s,
            lang,
            std::slice::from_ref(bt),
            obj_names,
            has_constraints,
        );
    }
    let _ = writeln!(s, "<h3>{}</h3>", esc(tr(lang, "Top trials", "上位 trial")));
    let _ = writeln!(
        s,
        "<p class=\"desc\">{}</p>",
        esc(tr(
            lang,
            "Best first; objective and parameter columns.",
            "最良順。目的とパラメータの列。"
        ))
    );
    render_trial_table(s, lang, top_n, obj_names, has_constraints);
}

/// Per-objective extremes table.
fn render_extremes_table(s: &mut String, lang: ReportLang, extremes: &[ObjectiveExtreme]) {
    let _ = writeln!(
        s,
        "<h3>{}</h3>",
        esc(tr(lang, "Per-objective extremes", "目的ごとの極値"))
    );
    let _ = writeln!(
        s,
        "<p class=\"desc\">{}</p>",
        esc(tr(
            lang,
            "Best value respects each objective's direction.",
            "最良値は各目的の方向に従う。"
        ))
    );
    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "objective", "目的"), false);
    th(s, tr(lang, "direction", "方向"), false);
    th(s, tr(lang, "best", "最良"), true);
    th(s, tr(lang, "best trial", "最良 trial"), true);
    th(s, tr(lang, "worst", "最悪"), true);
    s.push_str("</tr></thead>\n<tbody>\n");
    for e in extremes {
        s.push_str("<tr>");
        td(s, &e.objective_name, false);
        td(s, dir_label(lang, e.direction), false);
        td(s, &format_number(e.best_value), true);
        if e.best_feasible {
            td(s, &format!("#{}", e.best_trial_number), true);
        } else {
            // If the best trial violates constraints, flag it in red with a ✗.
            let _ = write!(
                s,
                "<td class=\"num infeasible\">#{} ✗</td>",
                e.best_trial_number
            );
        }
        td(s, &format_number(e.worst_value), true);
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);
}

/// Scatter plot of the objective space (first two objective axes, front / dominated series).
fn render_outcome_scatter(
    s: &mut String,
    lang: ReportLang,
    scatter: &[ParetoPoint],
    scatter_axes: (usize, usize),
    objective_count: usize,
    obj_names: &[String],
) {
    if scatter.is_empty() || objective_count < 2 {
        return;
    }
    let (xi, yi) = scatter_axes;
    let x_label = obj_names.get(xi).map(String::as_str).unwrap_or("obj x");
    let y_label = obj_names.get(yi).map(String::as_str).unwrap_or("obj y");
    let background: Vec<ScatterPoint> = scatter
        .iter()
        .filter(|p| !p.on_front)
        .map(scatter_pt)
        .collect();
    let front: Vec<ScatterPoint> = scatter
        .iter()
        .filter(|p| p.on_front)
        .map(scatter_pt)
        .collect();
    let chart = svg::scatter_chart(&background, &front, x_label, y_label, CHART_W, 440.0);
    let _ = writeln!(
        s,
        "<figure>{chart}<figcaption>{}</figcaption></figure>",
        esc(tr(
            lang,
            "Objective space: Pareto front vs dominated trials.",
            "目的空間: パレート前面と被支配解。"
        ))
    );
    if objective_count > 2 {
        let _ = writeln!(
            s,
            "<p class=\"desc\">{} ({} {}).</p>",
            esc(tr(
                lang,
                "Scatter uses the first two objectives",
                "散布図は先頭2目的を使用"
            )),
            objective_count,
            esc(tr(lang, "objectives total", "目的中"))
        );
    }
}

/// Pareto table (TOPSIS order), plus the fallback note and duplicate-solution legend.
fn render_pareto_table_block(
    s: &mut String,
    lang: ReportLang,
    pareto_table: &[TrialSummary],
    pareto_infeasible_count: usize,
    obj_names: &[String],
    has_constraints: bool,
) {
    let _ = writeln!(
        s,
        "<h3>{}</h3>",
        esc(tr(lang, "Pareto-front trials", "パレート前面の trial"))
    );
    let _ = writeln!(
        s,
        "<p class=\"desc\">{}</p>",
        esc(tr(
            lang,
            "Ordered by equal-weight TOPSIS (capped).",
            "等重み TOPSIS 順（cap 済み）。"
        ))
    );
    render_trial_table(s, lang, pareto_table, obj_names, has_constraints);

    // The front is computed only from feasible rows, so a violating trial
    // appears in the table only during the "no feasible solution exists"
    // fallback. The count is already tallied by the builder from the full
    // pre-cap front.
    if pareto_infeasible_count > 0 {
        let note = text::infeasible_fallback_note(lang, pareto_infeasible_count);
        let _ = writeln!(s, "<p class=\"desc\">{}</p>", esc(&note));
    }
    if has_duplicate_marks(pareto_table) {
        let _ = writeln!(
            s,
            "<p class=\"desc\">{}</p>",
            esc(text::duplicate_legend_note(lang))
        );
    }
}

fn scatter_pt(p: &ParetoPoint) -> ScatterPoint {
    ScatterPoint {
        trial_number: p.trial_number as i64,
        x: p.x,
        y: p.y,
        feasible: p.feasible,
    }
}

/// Table of TrialSummary rows (trial# + objectives + parameters [+ max constraint value]).
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
            "<p class=\"muted\">{}</p>",
            esc(tr(lang, "No trials.", "該当する trial はありません。"))
        );
        return;
    }
    // Determine whether each parameter column is numeric or categorical from the first row.
    let param_cols: Vec<(String, bool)> = trials[0]
        .params
        .iter()
        .map(|(name, v)| (name.clone(), matches!(v, ParamValue::Num(_))))
        .collect();

    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "trial", "trial"), true);
    for o in obj_names {
        th(s, o, true);
    }
    for (name, is_num) in &param_cols {
        th(s, name, *is_num);
    }
    if show_constraint {
        th(
            s,
            tr(
                lang,
                "max constraint (≤0 = feasible)",
                "最大制約値（≤0 で充足）",
            ),
            true,
        );
    }
    s.push_str("</tr></thead>\n<tbody>\n");

    for t in trials {
        s.push_str("<tr>");
        match t.duplicate_of {
            // For duplicate solutions with identical objective values, note the first-occurrence trial number (muted, understated).
            Some(first) => {
                let _ = write!(
                    s,
                    "<td class=\"num\">#{} <span class=\"muted\">(= #{first})</span></td>",
                    t.trial_number
                );
            }
            None => td(s, &format!("#{}", t.trial_number), true),
        }
        for i in 0..obj_names.len() {
            let v = t.objectives.get(i).copied().unwrap_or(f64::NAN);
            td(s, &format_number(v), true);
        }
        for (idx, (_, v)) in t.params.iter().enumerate() {
            let (text, _) = param_value(v);
            let is_num = param_cols.get(idx).map(|c| c.1).unwrap_or(false);
            td(s, &text, is_num);
        }
        if show_constraint {
            match t.max_constraint {
                // Positive value = constraint violation (the check is shared with the model side). Flag in red with a ✗.
                Some(v) if t.violates_constraints() => {
                    let _ = write!(
                        s,
                        "<td class=\"num infeasible\">{} ✗</td>",
                        esc(&format_number(v))
                    );
                }
                Some(v) => td(s, &format_number(v), true),
                None => td(s, "-", true),
            }
        }
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);
}
