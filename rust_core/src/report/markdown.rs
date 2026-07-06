//! Markdown レンダラ（LLM 向け主用途）。
//!
//! 構造: `# Optimization Report: {study}` → `## Key Findings`（箇条書き）→
//! 各セクション `##` + パイプテーブル。チャートは出さず数表で代替する。
//! 出力は決定論的（同一入力→バイト同一）で、`HashMap` 反復順に依存しない
//! （モデルは BTreeMap / ソート済み Vec で保持済み）。
//!
//! 言語（en / ja）はレンダリング時に選択する。テンプレートは推測や誇張を入れず、
//! モデルのファクトのみを文章化する。

use std::fmt::Write as _;

use super::builder::downsample;
use super::model::*;
use super::text::{self, format_unix_utc};
use super::{format_number, ReportLang};

/// [`StudyReport`] を Markdown へレンダリングする。
pub fn render_markdown(report: &StudyReport, lang: ReportLang) -> String {
    let mut s = String::new();

    // タイトル。
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
// 言語・エスケープヘルパー
// =============================================================================

/// 言語に応じて en / ja のいずれかを返す。
fn tr(lang: ReportLang, en: &'static str, ja: &'static str) -> &'static str {
    match lang {
        ReportLang::En => en,
        ReportLang::Ja => ja,
    }
}

/// テーブルセル用にバックスラッシュ・パイプ・改行をエスケープする。
///
/// バックスラッシュは `|` のエスケープより先に処理する必要がある。
/// 先に `|` → `\|` を行うと、元の文字列に含まれるバックスラッシュ
/// （例: `a\|b`）が「エスケープ済みパイプ」と区別できず、後段のバック
/// スラッシュ置換でパイプ側の `\` まで二重にエスケープされて表構造が
/// 崩れる（あるいはその逆で `\|` がバックスラッシュ+パイプに読めてしまう）。
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(['\n', '\r'], " ")
}

/// 方向ラベル。
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

fn pct(x: f64) -> String {
    format!("{:.0}", x)
}

// =============================================================================
// メタ行
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

/// Key Finding を 1 文へ整形する（テンプレートは [`super::text`] で共有）。
///
/// 強調 span は Markdown の `**...**` で囲み、全 span を [`esc`] でセルセーフに
/// する。リテラルへの `esc` はパイプ・改行を含まないため実質無変換で、
/// ユーザー由来文字列（param 名等）のみが安全化される。
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

            // 目的ごとの極値。
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
                // 制約違反 trial が最良の場合は明示マークを付ける。
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

            // パレート表（TOPSIS 順）。
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

            // 前面は feasible 行のみから計算されるため、違反 trial が表に
            // 現れるのは「feasible 解が 1 件も無い」フォールバック時のみ。
            let n_infeasible = pareto_table
                .iter()
                .filter(|t| t.max_constraint.is_some_and(|v| v > 0.0))
                .count();
            if n_infeasible > 0 {
                let _ = writeln!(
                    s,
                    "{}\n",
                    text::infeasible_fallback_note(lang, n_infeasible)
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
            let _ = writeln!(
                s,
                "{} {}.\n",
                tr(lang, "Scatter points:", "散布図の点数:"),
                scatter.len()
            );
        }
    }
}

/// 表内に重複解（`duplicate_of` 付き trial）があれば凡例を 1 行出力する。
fn render_duplicate_note(s: &mut String, lang: ReportLang, trials: &[TrialSummary]) {
    if trials.iter().any(|t| t.duplicate_of.is_some()) {
        let _ = writeln!(s, "{}\n", text::duplicate_legend_note(lang));
    }
}

/// TrialSummary の表を出力する（trial# + 目的 + パラメータ [+ 最大制約値]）。
///
/// `user_attrs` はここでは意図的に出力しない（LLM 向けレポートを簡潔に保つ
/// ため）。ユーザー付帯情報が必要な場合は HTML レンダラの付録（appendix）
/// 側で確認できる。
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

    // ヘッダ。
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
            // 同一目的値の重複解は初出 trial 番号を併記する。
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
                // 正値 = 制約違反。値の後に明示マークを付ける。
                Some(v) if v > 0.0 => format!(
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
        "{} `{}` {} `{}`. {}\n",
        tr(lang, "Method:", "手法:"),
        esc(&sec.method),
        tr(lang, "against objective", "評価対象の目的:"),
        esc(&sec.objective_name),
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
        "{} `{}`. {}\n",
        tr(lang, "Method:", "手法:"),
        esc(&sec.method),
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
        "{} `{}` ({}: {}). {}\n",
        tr(lang, "Weighting:", "重み付け:"),
        esc(&sec.weight_scheme),
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
    use super::esc;

    #[test]
    fn escapes_backslash_before_pipe() {
        // バックスラッシュを先にエスケープしないと `a\|b` の `\` が
        // 後続の `|` エスケープと衝突し、表構造が壊れ得る。
        assert_eq!(esc("a\\|b"), "a\\\\\\|b");
        assert_eq!(esc("trail\\"), "trail\\\\");
    }
}
