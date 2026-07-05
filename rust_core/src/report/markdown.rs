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

use super::model::*;
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

/// テーブルセル用にパイプと改行をエスケープする。
fn esc(s: &str) -> String {
    s.replace('|', "\\|").replace(['\n', '\r'], " ")
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
            "- {} (unix): {}",
            tr(lang, "Generated at", "生成日時"),
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

fn finding_sentence(lang: ReportLang, f: &KeyFinding) -> String {
    let num = |k: &str| f.metrics.get(k).copied().unwrap_or(f64::NAN);
    let lab = |k: &str| f.labels.get(k).cloned().unwrap_or_default();
    match f.kind {
        FindingKind::BestSingle => {
            let best = format_number(num("best"));
            let trial = format_number(num("trial"));
            let fp = pct(num("found_pct"));
            match lang {
                ReportLang::En => format!(
                    "Best objective **{best}** at trial #{trial} (found at {fp}% of the run)."
                ),
                ReportLang::Ja => format!(
                    "最良値 **{best}**（trial #{trial}、全体の {fp}% 時点で発見）。"
                ),
            }
        }
        FindingKind::ParetoSummary => {
            let front = format_number(num("front_size"));
            let complete = format_number(num("complete"));
            match lang {
                ReportLang::En => format!(
                    "Pareto front holds **{front}** non-dominated trials out of {complete} completed."
                ),
                ReportLang::Ja => format!(
                    "パレート前面は COMPLETE {complete} 件中 **{front}** 件の非支配解で構成される。"
                ),
            }
        }
        FindingKind::ConvergenceStatus => match lab("status").as_str() {
            "converged" => tr(
                lang,
                "Optimization appears **converged** — no best-value updates in the final 20% of trials.",
                "最適化は**収束**したとみられる（後半20%の試行で best 更新なし）。",
            )
            .to_string(),
            "still_improving" => tr(
                lang,
                "Optimization is **still improving** — best value was updated within the final 20% of trials; more trials may help.",
                "最適化はなお改善中（直近20%の試行でも best が更新されており、追加試行で改善の余地がある）。",
            )
            .to_string(),
            _ => tr(
                lang,
                "**Insufficient data** for a convergence verdict (fewer than 10 completed trials).",
                "収束判定に十分なデータがない（COMPLETE が10件未満）。",
            )
            .to_string(),
        },
        FindingKind::TopImportance => {
            let method = lab("method");
            let mut parts = Vec::new();
            for i in 1..=3 {
                let name = lab(&format!("param{i}"));
                if name.is_empty() {
                    continue;
                }
                let score = format_number(num(&format!("score{i}")));
                parts.push(format!("{} ({})", esc(&name), score));
            }
            let list = parts.join(", ");
            match lang {
                ReportLang::En => {
                    format!("Most influential parameters ({method}): {list}.")
                }
                ReportLang::Ja => {
                    format!("影響の大きいパラメータ（{method}）: {list}。")
                }
            }
        }
        FindingKind::TradeOff => {
            let a = esc(&lab("obj_a"));
            let b = esc(&lab("obj_b"));
            let rho = format_number(num("rho"));
            match lang {
                ReportLang::En => {
                    format!("Objectives **{a}** and **{b}** trade off (Spearman ρ = {rho}).")
                }
                ReportLang::Ja => {
                    format!("目的 **{a}** と **{b}** はトレードオフの関係にある（Spearman ρ = {rho}）。")
                }
            }
        }
        FindingKind::Feasibility => {
            let feasible = format_number(num("feasible"));
            let total = format_number(num("total"));
            let rate = pct(num("rate") * 100.0);
            let tail = if f.labels.contains_key("has_best") {
                let bt = format_number(num("best_trial"));
                match lang {
                    ReportLang::En => format!("; best feasible at trial #{bt}"),
                    ReportLang::Ja => format!("、最良は trial #{bt}"),
                }
            } else {
                String::new()
            };
            match lang {
                ReportLang::En => {
                    format!("Feasible trials: **{feasible}/{total}** ({rate}%){tail}.")
                }
                ReportLang::Ja => {
                    format!("実行可能な試行: **{feasible}/{total}**（{rate}%）{tail}。")
                }
            }
        }
        FindingKind::PruningEfficiency => {
            let pruned = format_number(num("pruned"));
            let rate = pct(num("rate") * 100.0);
            let tail = if f.labels.contains_key("has_step") {
                let step = format_number(num("median_step"));
                match lang {
                    ReportLang::En => format!("; median prune step {step}"),
                    ReportLang::Ja => format!("、中央値 step {step}"),
                }
            } else {
                String::new()
            };
            match lang {
                ReportLang::En => {
                    format!("Pruning removed **{pruned}** trials ({rate}% of finished){tail}.")
                }
                ReportLang::Ja => {
                    format!("枝刈りにより **{pruned}** 件の試行が早期終了（終了試行の {rate}%）{tail}。")
                }
            }
        }
        FindingKind::DataQuality => {
            let nan = format_number(num("nan_count"));
            let fail = format_number(num("fail_count"));
            match lang {
                ReportLang::En => format!(
                    "Data quality note: {nan} trial(s) with non-finite objective values, {fail} FAILED trial(s)."
                ),
                ReportLang::Ja => format!(
                    "データ品質の注意: 目的値が非有限の試行 {nan} 件、FAIL 試行 {fail} 件。"
                ),
            }
        }
    }
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
                let _ = writeln!(
                    s,
                    "| {} | {} | {} | #{} | {} |",
                    esc(&e.objective_name),
                    dir_label(lang, e.direction),
                    format_number(e.best_value),
                    e.best_trial_number,
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

/// TrialSummary の表を出力する（trial# + 目的 + パラメータ [+ 制約違反]）。
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
        let _ = write!(header, " {} |", tr(lang, "violation", "制約違反"));
    }
    let _ = writeln!(s, "{header}");

    let cols = 1 + obj_names.len() + param_names.len() + usize::from(show_constraint);
    let sep: String = std::iter::repeat_n("---", cols)
        .collect::<Vec<_>>()
        .join("|");
    let _ = writeln!(s, "|{sep}|");

    for t in trials {
        let mut row = format!("| #{} |", t.trial_number);
        for (i, _) in obj_names.iter().enumerate() {
            let v = t.objectives.get(i).copied().unwrap_or(f64::NAN);
            let _ = write!(row, " {} |", format_number(v));
        }
        for (_, v) in &t.params {
            let _ = write!(row, " {} |", param_val(v));
        }
        if show_constraint {
            let c = t
                .constraint_violation
                .map(format_number)
                .unwrap_or_else(|| "-".to_string());
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
    let sampled = downsample_points(&conv.series, 20);
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

fn downsample_points(pts: &[ConvergencePoint], max: usize) -> Vec<ConvergencePoint> {
    if pts.len() <= max || max < 2 {
        return pts.to_vec();
    }
    let last = pts.len() - 1;
    (0..max)
        .map(|k| pts[k * last / (max - 1)].clone())
        .collect()
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
