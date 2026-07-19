//! Execution info section and the Appendix (reproduction info + full
//! parameters of the representative trial).

use std::fmt::Write as _;

use super::*;
use crate::report::model::{ExecutionSection, Outcome, StudyReport};
use crate::report::{format_number, pct, ReportLang};

pub(super) fn render_execution(
    s: &mut String,
    lang: ReportLang,
    execution: Option<&ExecutionSection>,
) {
    let Some(sec) = execution else {
        return;
    };
    let _ = writeln!(
        s,
        "<h2 id=\"execution\">{}</h2>",
        esc(tr(lang, "Execution", "実行情報"))
    );

    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "state", "state"), false);
    th(s, tr(lang, "count", "件数"), true);
    s.push_str("</tr></thead>\n<tbody>\n");
    for (state, count) in &sec.state_counts {
        s.push_str("<tr>");
        td(s, state, false);
        td(s, &count.to_string(), true);
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);

    s.push_str("<ul class=\"facts\">\n");
    fact_row(
        s,
        tr(lang, "Pruned rate", "枝刈り率"),
        &format!("{}%", pct(sec.pruned_rate * 100.0)),
    );
    if let Some(step) = sec.median_prune_step {
        fact_row(
            s,
            tr(lang, "Median prune step", "枝刈り step 中央値"),
            &format_number(step),
        );
    }
    if let (Some(mean), Some(std)) = (sec.mean_trial_seconds, sec.std_trial_seconds) {
        fact_row(
            s,
            tr(lang, "Mean trial time", "平均 trial 時間"),
            &format!("{} ± {} s", format_number(mean), format_number(std)),
        );
    }
    if let Some(total) = sec.total_seconds {
        fact_row(
            s,
            tr(lang, "Total time", "総所要時間"),
            &format!("{} s", format_number(total)),
        );
    }
    s.push_str("</ul>\n");
}

pub(super) fn render_appendix(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let _ = writeln!(
        s,
        "<h2 id=\"appendix\">{}</h2>",
        esc(tr(lang, "Appendix", "付録"))
    );
    let _ = writeln!(
        s,
        "<details><summary>{}</summary>",
        esc(tr(
            lang,
            "Reproduction & full parameters",
            "再現情報・全パラメータ"
        ))
    );

    // Reproduction info.
    let r = &report.reproduction;
    let _ = writeln!(s, "<h3>{}</h3>", esc(tr(lang, "Reproduction", "再現情報")));
    s.push_str("<ul class=\"facts\">\n");
    fact_row(s, "study_id", &r.study_id.to_string());
    fact_row(
        s,
        tr(lang, "storage (masked)", "ストレージ（マスク済み）"),
        &r.storage_display,
    );
    fact_row(s, "top_n", &r.top_n.to_string());
    fact_row(s, "max_heatmap_params", &r.max_heatmap_params.to_string());
    fact_row(s, "schema_version", &r.schema_version.to_string());
    s.push_str("</ul>\n");

    // All parameters of the representative trial.
    let representative = match &report.outcome {
        Outcome::SingleObj { best_trial, .. } => best_trial.as_ref(),
        Outcome::MultiObj { pareto_table, .. } => pareto_table.first(),
    };
    if let Some(t) = representative {
        let _ = writeln!(
            s,
            "<h3>{} (#{})</h3>",
            esc(tr(
                lang,
                "Representative trial parameters",
                "代表 trial の全パラメータ"
            )),
            t.trial_number
        );
        if t.params.is_empty() && t.user_attrs.is_empty() {
            let _ = writeln!(
                s,
                "<p class=\"muted\">{}</p>",
                esc(tr(lang, "No parameters.", "パラメータはありません。"))
            );
        } else {
            open_table(s);
            s.push_str("<thead><tr>");
            th(s, tr(lang, "key", "項目"), false);
            th(s, tr(lang, "value", "値"), false);
            s.push_str("</tr></thead>\n<tbody>\n");
            for (name, v) in &t.params {
                let (text, is_num) = param_value(v);
                s.push_str("<tr>");
                td(s, name, false);
                td(s, &text, is_num);
                s.push_str("</tr>\n");
            }
            for (name, value) in &t.user_attrs {
                s.push_str("<tr>");
                td(s, &format!("user_attr: {name}"), false);
                td(s, value, false);
                s.push_str("</tr>\n");
            }
            s.push_str("</tbody>\n");
            close_table(s);
        }
    }

    s.push_str("</details>\n");
}
