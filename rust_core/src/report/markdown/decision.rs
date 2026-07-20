//! Multi-criteria decision analysis (MCDM) section: TOPSIS / VIKOR /
//! PROMETHEE II ranking tables plus their consensus set.

use std::fmt::Write as _;

use super::*;
use crate::report::model::{McdmEntry, McdmSection};
use crate::report::{format_number, ReportLang};

pub(super) fn render_mcdm(
    s: &mut String,
    lang: ReportLang,
    mcdm: &Option<McdmSection>,
    obj_names: &[String],
) {
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
