//! Multi-criteria decision analysis (MCDM) section: TOPSIS / VIKOR /
//! PROMETHEE II ranking tables plus their consensus set.

use std::fmt::Write as _;

use super::*;
use crate::report::model::{McdmEntry, McdmSection};
use crate::report::{format_number, ReportLang};

pub(super) fn render_mcdm(
    s: &mut String,
    lang: ReportLang,
    mcdm: Option<&McdmSection>,
    obj_names: &[String],
) {
    let Some(sec) = mcdm else {
        return;
    };
    let _ = writeln!(
        s,
        "<h2 id=\"mcdm\">{}</h2>",
        esc(tr(lang, "Decision Analysis (MCDM)", "意思決定分析（MCDM）"))
    );
    let weights: Vec<String> = sec.weights.iter().map(|w| format_number(*w)).collect();
    let _ = writeln!(
        s,
        "<p class=\"desc\">{} <code>{}</code> ({}: {}). {}</p>",
        esc(tr(lang, "Weighting:", "重み付け:")),
        esc(&sec.weight_scheme),
        esc(tr(lang, "weights", "重み")),
        esc(&weights.join(", ")),
        esc(tr(
            lang,
            "Rankings are computed on the Pareto front only.",
            "ランキングはパレート前面のみを対象に計算する。"
        ))
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
        "<div class=\"callout\"><strong>{}</strong> {}</div>",
        esc(tr(
            lang,
            "Consensus (top-10 of all three methods):",
            "コンセンサス（3手法すべての top10）:"
        )),
        if consensus.is_empty() {
            esc(tr(lang, "none", "なし"))
        } else {
            esc(&consensus.join(", "))
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
    let _ = writeln!(s, "<h3>{}</h3>", esc(method));
    if entries.is_empty() {
        let _ = writeln!(
            s,
            "<p class=\"muted\">{}</p>",
            esc(tr(lang, "No entries.", "該当なし。"))
        );
        return;
    }
    open_table(s);
    s.push_str("<thead><tr>");
    th(s, tr(lang, "rank", "順位"), true);
    th(s, tr(lang, "trial", "trial"), true);
    for o in obj_names {
        th(s, o, true);
    }
    s.push_str("</tr></thead>\n<tbody>\n");
    for e in entries {
        s.push_str("<tr>");
        td(s, &e.rank.to_string(), true);
        td(s, &format!("#{}", e.trial_number), true);
        for v in &e.objectives {
            td(s, &format_number(*v), true);
        }
        s.push_str("</tr>\n");
    }
    s.push_str("</tbody>\n");
    close_table(s);
}
