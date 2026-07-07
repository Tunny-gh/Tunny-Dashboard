//! 自己完結 HTML レポートレンダラ。
//!
//! 外部リソース参照ゼロ・JS 不使用の 1 枚ページを生成する。スタイルは
//! [`super::theme::css_variables`] が定義する CSS custom property を参照し、
//! `prefers-color-scheme` によりライト/ダークへ自動追従する。チャートは
//! [`super::svg`] のプリミティブを呼び出して SVG として埋め込み、すべての
//! チャートには対応する数表を併置する（表がプライマリ、チャートは補助）。
//!
//! レンダラは計算しない・描くだけの原則に従い、系列や統計は builder が
//! 事前に計算したモデル値をそのまま用いる（`HashMap` 反復に依存せず、
//! 同一入力に対しバイト同一の出力を返す）。文言はすべて en / ja 対応で、
//! Key Finding の文章テンプレートは [`super::text`] を Markdown と共有する。

use std::fmt::Write as _;

use super::model::*;
use super::svg::{self, HBarItem, HistBin, LinePoint, ScatterPoint};
use super::text::{self, format_unix_utc};
use super::theme;
use super::{format_number, pct, ReportLang};

/// フルページ幅チャートの viewBox 幅（レスポンシブなので相対比のみ意味を持つ）。
const CHART_W: f64 = 880.0;
/// ヒストグラムを描画する目的の最大数。
const MAX_HISTOGRAMS: usize = 4;

/// [`StudyReport`] を自己完結 HTML へレンダリングする。
pub fn render_html(report: &StudyReport, lang: ReportLang) -> String {
    let mut s = String::new();
    let lang_attr = match lang {
        ReportLang::En => "en",
        ReportLang::Ja => "ja",
    };

    s.push_str("<!DOCTYPE html>\n");
    let _ = writeln!(s, "<html lang=\"{lang_attr}\">");
    s.push_str("<head>\n");
    s.push_str("<meta charset=\"utf-8\">\n");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    let _ = writeln!(
        s,
        "<title>{}: {}</title>",
        tr(lang, "Optimization Report", "最適化レポート"),
        esc(&report.overview.name)
    );
    s.push_str("<style>\n");
    s.push_str(&theme::css_variables());
    s.push_str(PAGE_CSS);
    s.push_str("</style>\n");
    s.push_str("</head>\n<body>\n");

    render_header(&mut s, lang, report);
    render_toc(&mut s, lang, report);
    render_findings(&mut s, lang, &report.key_findings);
    render_outcome(&mut s, lang, report);
    render_convergence(&mut s, lang, &report.convergence);
    render_importance(&mut s, lang, report.importance.as_ref());
    render_objective_stats(&mut s, lang, &report.objective_stats);
    render_correlations(&mut s, lang, report.correlations.as_ref());
    render_mcdm(
        &mut s,
        lang,
        report.mcdm.as_ref(),
        &report.overview.objective_names,
    );
    render_execution(&mut s, lang, report.execution.as_ref());
    render_appendix(&mut s, lang, report);

    s.push_str("</body>\n</html>\n");
    s
}

/// ページ本体の CSS（`css_variables()` に続けて `<style>` に埋め込む）。
const PAGE_CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; }
body {
  background: var(--surface);
  color: var(--ink-primary);
  font-family: system-ui, -apple-system, "Segoe UI", "Hiragino Kaku Gothic ProN", "Noto Sans JP", sans-serif;
  max-width: 960px;
  margin: 0 auto;
  padding: 28px 22px 72px;
  line-height: 1.55;
}
h1 { font-size: 1.75rem; margin: 0 0 6px; line-height: 1.25; }
h2 {
  font-size: 1.3rem;
  margin: 44px 0 12px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--grid);
  scroll-margin-top: 12px;
}
h3 { font-size: 1.02rem; margin: 22px 0 8px; color: var(--ink-secondary); }
p { margin: 8px 0; }
a { color: var(--series-1); text-decoration: none; }
a:hover { text-decoration: underline; }
.meta { list-style: none; padding: 0; margin: 8px 0 4px; color: var(--ink-secondary); font-size: 0.9rem; }
.meta li { display: flex; gap: 8px; padding: 1px 0; }
.meta .k { color: var(--ink-muted); min-width: 140px; }
.meta .v { font-variant-numeric: tabular-nums; }
.desc { color: var(--ink-secondary); font-size: 0.88rem; margin: 6px 0 10px; }
nav.toc {
  margin: 20px 0 8px;
  padding: 12px 16px;
  border: 1px solid var(--grid);
  border-radius: 8px;
  background: color-mix(in srgb, var(--surface) 92%, var(--ink-muted));
}
nav.toc .toc-title { font-weight: 600; font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.04em; color: var(--ink-muted); margin-bottom: 6px; }
nav.toc ol { margin: 0; padding-left: 20px; columns: 2; column-gap: 28px; font-size: 0.92rem; }
nav.toc li { margin: 2px 0; break-inside: avoid; }
.finding {
  border: 1px solid var(--grid);
  border-left: 3px solid var(--series-1);
  border-radius: 6px;
  padding: 9px 14px;
  margin: 9px 0;
  background: color-mix(in srgb, var(--surface) 94%, var(--ink-muted));
}
.finding .badge {
  display: inline-block;
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-weight: 700;
  color: var(--series-1);
  margin-right: 8px;
}
.finding strong { font-variant-numeric: tabular-nums; }
ul.facts { list-style: none; padding: 0; margin: 8px 0; font-size: 0.9rem; }
ul.facts li { display: flex; gap: 8px; padding: 1px 0; }
ul.facts .k { color: var(--ink-muted); min-width: 200px; }
ul.facts .v { font-variant-numeric: tabular-nums; }
.table-wrap { overflow-x: auto; margin: 12px 0; }
table { border-collapse: collapse; width: 100%; font-size: 0.85rem; font-variant-numeric: tabular-nums; }
th, td { text-align: left; padding: 5px 11px; border-bottom: 1px solid var(--grid); white-space: nowrap; }
th { color: var(--ink-secondary); font-weight: 600; border-bottom: 2px solid var(--axis); position: sticky; top: 0; background: var(--surface); }
td.num, th.num { text-align: right; }
tbody tr:hover { background: color-mix(in srgb, var(--surface) 90%, var(--ink-muted)); }
figure { margin: 14px 0; }
figcaption { color: var(--ink-muted); font-size: 0.8rem; margin-top: 4px; text-align: center; }
details { margin: 8px 0; border: 1px solid var(--grid); border-radius: 6px; padding: 4px 12px; }
summary { cursor: pointer; color: var(--ink-secondary); font-size: 0.9rem; padding: 4px 0; }
.callout {
  border-left: 3px solid var(--series-2);
  background: color-mix(in srgb, var(--surface) 94%, var(--ink-muted));
  padding: 8px 14px;
  border-radius: 4px;
  margin: 12px 0;
  font-size: 0.9rem;
}
.muted { color: var(--ink-muted); }
td.infeasible { color: var(--series-6); font-weight: 600; }
@media print {
  body { max-width: none; padding: 0; }
  nav.toc { break-inside: avoid; }
  th { position: static; }
  details { border: none; }
  details[open] > summary { display: none; }
  * { position: static !important; }
}
"#;

// =============================================================================
// エスケープ・言語ヘルパー
// =============================================================================

/// HTML テキストノード / 属性値用エスケープ（`svg` と共有）。
fn esc(s: &str) -> String {
    svg::escape_xml(s)
}

/// 言語に応じて en / ja を返す。
fn tr(lang: ReportLang, en: &'static str, ja: &'static str) -> &'static str {
    match lang {
        ReportLang::En => en,
        ReportLang::Ja => ja,
    }
}

fn dir_label(lang: ReportLang, d: Direction) -> &'static str {
    match d {
        Direction::Minimize => tr(lang, "Minimize", "最小化"),
        Direction::Maximize => tr(lang, "Maximize", "最大化"),
    }
}

fn yes_no(lang: ReportLang, b: bool) -> &'static str {
    if b {
        tr(lang, "yes", "はい")
    } else {
        tr(lang, "no", "いいえ")
    }
}

/// 数値パラメータ値を `(表示文字列, 数値か)` として返す。
fn param_value(v: &ParamValue) -> (String, bool) {
    match v {
        ParamValue::Num(x) => (format_number(*x), true),
        ParamValue::Cat(s) => (s.clone(), false),
    }
}

// =============================================================================
// テーブル出力ヘルパー
// =============================================================================

fn open_table(s: &mut String) {
    s.push_str("<div class=\"table-wrap\"><table>\n");
}

fn close_table(s: &mut String) {
    s.push_str("</table></div>\n");
}

/// `<th>` セルを書き出す。
fn th(s: &mut String, text: &str, numeric: bool) {
    let cls = if numeric { " class=\"num\"" } else { "" };
    let _ = write!(s, "<th{cls}>{}</th>", esc(text));
}

/// `<td>` セルを書き出す。
fn td(s: &mut String, text: &str, numeric: bool) {
    let cls = if numeric { " class=\"num\"" } else { "" };
    let _ = write!(s, "<td{cls}>{}</td>", esc(text));
}

// =============================================================================
// ヘッダ・目次
// =============================================================================

fn render_header(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let ov = &report.overview;
    let _ = writeln!(s, "<h1>{}</h1>", esc(&ov.name));
    s.push_str("<ul class=\"meta\">\n");

    meta_row(
        s,
        tr(lang, "Storage", "ストレージ"),
        &report.source.storage_display,
    );
    if let Some(ts) = report.source.generated_at_unix {
        meta_row(
            s,
            tr(lang, "Generated at", "生成日時"),
            &format_unix_utc(ts),
        );
    }
    let dirs: Vec<String> = ov
        .objective_names
        .iter()
        .zip(ov.directions.iter())
        .map(|(name, d)| format!("{} ({})", name, dir_label(lang, *d)))
        .collect();
    meta_row(
        s,
        tr(lang, "Objectives", "目的"),
        &if dirs.is_empty() {
            "-".to_string()
        } else {
            dirs.join(", ")
        },
    );
    meta_row(
        s,
        tr(lang, "Trials", "試行数"),
        &format!(
            "{} COMPLETE / {} {}",
            ov.complete_trials,
            ov.total_trials,
            tr(lang, "total", "全体")
        ),
    );
    if !ov.state_counts.is_empty() {
        let states: Vec<String> = ov
            .state_counts
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect();
        meta_row(s, tr(lang, "States", "状態内訳"), &states.join(", "));
    }
    if let Some(w) = ov.wall_clock_seconds {
        meta_row(
            s,
            tr(lang, "Wall-clock", "実測所要時間"),
            &format!("{} s", format_number(w)),
        );
    }
    s.push_str("</ul>\n");
}

fn meta_row(s: &mut String, key: &str, value: &str) {
    let _ = writeln!(
        s,
        "<li><span class=\"k\">{}</span><span class=\"v\">{}</span></li>",
        esc(key),
        esc(value)
    );
}

fn render_toc(s: &mut String, lang: ReportLang, report: &StudyReport) {
    let mut items: Vec<(&str, String)> = Vec::new();
    items.push((
        "key-findings",
        tr(lang, "Key Findings", "まとめ").to_string(),
    ));
    items.push(("outcome", tr(lang, "Outcome", "最適化結果").to_string()));
    items.push(("convergence", tr(lang, "Convergence", "収束").to_string()));
    if report.importance.is_some() {
        items.push((
            "importance",
            tr(lang, "Parameter Importance", "パラメータ重要度").to_string(),
        ));
    }
    if !report.objective_stats.is_empty() {
        items.push((
            "objective-stats",
            tr(lang, "Objective Statistics", "目的値の統計").to_string(),
        ));
    }
    if report
        .correlations
        .as_ref()
        .is_some_and(|c| !c.params.is_empty())
    {
        items.push(("correlations", tr(lang, "Correlations", "相関").to_string()));
    }
    if report.mcdm.is_some() {
        items.push((
            "mcdm",
            tr(lang, "Decision Analysis (MCDM)", "意思決定分析（MCDM）").to_string(),
        ));
    }
    if report.execution.is_some() {
        items.push(("execution", tr(lang, "Execution", "実行情報").to_string()));
    }
    items.push(("appendix", tr(lang, "Appendix", "付録").to_string()));

    s.push_str("<nav class=\"toc\" aria-label=\"Contents\">\n");
    let _ = writeln!(
        s,
        "<div class=\"toc-title\">{}</div>",
        esc(tr(lang, "Contents", "目次"))
    );
    s.push_str("<ol>\n");
    for (id, label) in items {
        let _ = writeln!(s, "<li><a href=\"#{id}\">{}</a></li>", esc(&label));
    }
    s.push_str("</ol>\n</nav>\n");
}

// =============================================================================
// Key Findings
// =============================================================================

fn render_findings(s: &mut String, lang: ReportLang, findings: &[KeyFinding]) {
    let _ = writeln!(
        s,
        "<h2 id=\"key-findings\">{}</h2>",
        esc(tr(lang, "Key Findings", "まとめ"))
    );
    if findings.is_empty() {
        let _ = writeln!(
            s,
            "<p class=\"muted\">{}</p>",
            esc(tr(lang, "No findings.", "まとめはありません。"))
        );
        return;
    }
    for f in findings {
        let _ = writeln!(
            s,
            "<div class=\"finding\"><span class=\"badge\">{}</span>{}</div>",
            esc(finding_badge(lang, f.kind)),
            finding_html(lang, f)
        );
    }
}

/// Key Finding をエスケープ済み HTML 文へ整形する（テンプレートは `text` 共有）。
fn finding_html(lang: ReportLang, f: &KeyFinding) -> String {
    let mut out = String::new();
    for span in super::text::finding_spans(lang, f) {
        let body = esc(&span.text);
        if span.emphasis {
            let _ = write!(out, "<strong>{body}</strong>");
        } else {
            out.push_str(&body);
        }
    }
    out
}

fn finding_badge(lang: ReportLang, kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::BestSingle => tr(lang, "Best", "最良"),
        FindingKind::ParetoSummary => tr(lang, "Pareto", "パレート"),
        FindingKind::ConvergenceStatus => tr(lang, "Convergence", "収束"),
        FindingKind::TopImportance => tr(lang, "Importance", "重要度"),
        FindingKind::TradeOff => tr(lang, "Trade-off", "トレードオフ"),
        FindingKind::Feasibility => tr(lang, "Feasibility", "実行可能性"),
        FindingKind::PruningEfficiency => tr(lang, "Pruning", "枝刈り"),
        FindingKind::DataQuality => tr(lang, "Data quality", "データ品質"),
    }
}

// =============================================================================
// Outcome
// =============================================================================

fn render_outcome(s: &mut String, lang: ReportLang, report: &StudyReport) {
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

/// 単目的の Outcome（最良 trial + 上位 trial 表）。
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

/// 目的ごとの極値表。
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
            // 制約違反 trial が最良の場合は赤字 + ✗ で明示する。
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

/// 目的空間の散布図（先頭2目的軸、front / dominated 2系列）。
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

/// パレート表（TOPSIS 順）と、フォールバック注記・重複解凡例。
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

    // 前面は feasible 行のみから計算されるため、違反 trial が表に
    // 現れるのは「feasible 解が 1 件も無い」フォールバック時のみ。
    // 件数は builder が cap 前の front 全体から集計済み。
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

/// TrialSummary の表（trial# + 目的 + パラメータ [+ 最大制約値]）。
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
    // パラメータ列の数値/カテゴリ性を先頭行から確定する。
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
            // 同一目的値の重複解は初出 trial 番号を併記する（muted で控えめに）。
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
                // 正値 = 制約違反（判定は model 側で共有）。赤字 + ✗ で明示する。
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

// =============================================================================
// Convergence
// =============================================================================

fn render_convergence(s: &mut String, lang: ReportLang, conv: &ConvergenceSection) {
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

    // 折れ線チャート（最良更新点にマーカー）。
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

    // 末尾10点の表（<details> 内）。
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

fn fact_row(s: &mut String, key: &str, value: &str) {
    let _ = writeln!(
        s,
        "<li><span class=\"k\">{}</span><span class=\"v\">{}</span></li>",
        esc(key),
        esc(value)
    );
}

// =============================================================================
// Importance
// =============================================================================

fn render_importance(s: &mut String, lang: ReportLang, importance: Option<&ImportanceSection>) {
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

fn render_objective_stats(s: &mut String, lang: ReportLang, stats: &[ObjectiveStats]) {
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

    // ヒストグラム（最大 MAX_HISTOGRAMS 枚）。
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

fn render_correlations(
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

// =============================================================================
// MCDM
// =============================================================================

fn render_mcdm(s: &mut String, lang: ReportLang, mcdm: Option<&McdmSection>, obj_names: &[String]) {
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

// =============================================================================
// Execution
// =============================================================================

fn render_execution(s: &mut String, lang: ReportLang, execution: Option<&ExecutionSection>) {
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

// =============================================================================
// Appendix（Reproduction + 全パラメータ）
// =============================================================================

fn render_appendix(s: &mut String, lang: ReportLang, report: &StudyReport) {
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

    // 再現情報。
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

    // 代表 trial の全パラメータ。
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

// =============================================================================
// テスト
// =============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::render_html;
    use crate::data::dataframe::{DataFrame, TrialRow};
    use crate::data::extras::{StudyExtras, TrialExtra, TrialState};
    use crate::io::journal::parser::{OptimizationDirection, StudyMeta};
    use crate::report::model::Outcome;
    use crate::report::{
        build_study_report, render_markdown, ReportLang, ReportOptions, ReportSource, StudyReport,
    };

    fn source() -> ReportSource {
        ReportSource {
            storage_display: "sqlite:///demo.db".to_string(),
            generated_at_unix: Some(1_700_000_000),
        }
    }

    fn opts() -> ReportOptions {
        ReportOptions::default()
    }

    fn row(id: u32, params: &[(&str, f64)], objs: &[f64]) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            param_display: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            param_category_label: HashMap::new(),
            objective_values: objs.to_vec(),
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }
    }

    fn meta_single() -> StudyMeta {
        StudyMeta {
            study_id: 7,
            name: "single_study".to_string(),
            directions: vec![OptimizationDirection::Minimize],
            completed_trials: 12,
            total_trials: 12,
            param_names: vec!["a".to_string(), "b".to_string()],
            objective_names: vec!["obj0".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: HashMap::new(),
        }
    }

    fn df_single() -> DataFrame {
        let b = [5.0, 3.0, 8.0, 1.0, 9.0, 2.0, 7.0, 0.0, 6.0, 4.0, 10.0, 11.0];
        let rows: Vec<TrialRow> = (0..12)
            .map(|i| row(i, &[("a", i as f64), ("b", b[i as usize])], &[i as f64]))
            .collect();
        DataFrame::from_trials(
            &rows,
            &["a".to_string(), "b".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        )
    }

    fn meta_multi() -> StudyMeta {
        StudyMeta {
            study_id: 9,
            name: "multi_study".to_string(),
            directions: vec![
                OptimizationDirection::Minimize,
                OptimizationDirection::Minimize,
            ],
            completed_trials: 6,
            total_trials: 6,
            param_names: vec!["p".to_string(), "q".to_string()],
            objective_names: vec!["obj0".to_string(), "obj1".to_string()],
            user_attr_names: vec![],
            has_constraints: false,
            param_bounds: HashMap::new(),
        }
    }

    fn df_multi() -> DataFrame {
        // front = {trial0(1,4), trial1(2,2), trial2(4,1)}、他は支配される。
        let pts = [
            (1.0, 4.0),
            (2.0, 2.0),
            (4.0, 1.0),
            (3.0, 3.0),
            (5.0, 5.0),
            (2.0, 3.0),
        ];
        let rows: Vec<TrialRow> = pts
            .iter()
            .enumerate()
            .map(|(i, &(o0, o1))| {
                row(
                    i as u32,
                    &[("p", i as f64), ("q", (6 - i) as f64)],
                    &[o0, o1],
                )
            })
            .collect();
        DataFrame::from_trials(
            &rows,
            &["p".to_string(), "q".to_string()],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            0,
        )
    }

    /// extras（COMPLETE + PRUNED + FAIL）を組み立てて実行情報セクションを有効化する。
    fn extras_multi() -> StudyExtras {
        let mut extras = StudyExtras::default();
        for i in 0..6u32 {
            extras.trials.push(TrialExtra {
                trial_id: i,
                trial_number: i,
                state: TrialState::Complete,
                datetime_start: Some(i as f64),
                datetime_complete: Some(i as f64 + 1.0),
                intermediate_values: vec![],
            });
        }
        extras.trials.push(TrialExtra {
            trial_id: 6,
            trial_number: 6,
            state: TrialState::Pruned,
            datetime_start: Some(0.0),
            datetime_complete: Some(2.0),
            intermediate_values: vec![(0, 1.0), (5, 0.5)],
        });
        extras.trials.push(TrialExtra {
            trial_id: 7,
            trial_number: 7,
            state: TrialState::Fail,
            datetime_start: None,
            datetime_complete: None,
            intermediate_values: vec![],
        });
        extras
    }

    fn count(html: &str, needle: &str) -> usize {
        html.matches(needle).count()
    }

    /// 生成 HTML が自己完結（外部リソース参照ゼロ・JS 不使用）であることを確認する。
    ///
    /// SVG の `xmlns="http://www.w3.org/2000/svg"` は名前空間識別子であり
    /// ネットワーク取得を伴わないため、外部フェッチのパターン（`href="http`・
    /// `src=`・`url(`・`@import`）のみを検査する。
    fn assert_self_contained(html: &str) {
        assert!(html.starts_with("<!DOCTYPE html>"), "DOCTYPE 先頭");
        assert!(!html.contains("<script"), "JS を含まない");
        assert!(!html.contains("href=\"http"), "外部リンク href を含まない");
        assert!(!html.contains("src="), "外部リソース src を含まない");
        assert!(!html.contains("url("), "CSS url() 参照を含まない");
        assert!(!html.contains("@import"), "外部 CSS import を含まない");
    }

    #[test]
    fn multi_objective_full_section_render() {
        let report = build_study_report(
            &meta_multi(),
            &df_multi(),
            Some(&extras_multi()),
            &source(),
            &opts(),
        );
        let html = render_html(&report, ReportLang::En);

        assert_self_contained(&html);
        // 主要セクションのアンカーが揃っている（多目的 + extras の全部入り）。
        for id in [
            "id=\"key-findings\"",
            "id=\"outcome\"",
            "id=\"convergence\"",
            "id=\"importance\"",
            "id=\"objective-stats\"",
            "id=\"correlations\"",
            "id=\"mcdm\"",
            "id=\"execution\"",
            "id=\"appendix\"",
        ] {
            assert!(html.contains(id), "セクション欠落: {id}");
        }
        // 目次にも MCDM へのリンクがある。
        assert!(html.contains("<nav class=\"toc\""));
        assert!(html.contains("href=\"#mcdm\""));

        // チャート: 収束 line + 散布図 + importance hbar + ヒストグラム + heatmap。
        let n_svg = count(&html, "<svg");
        assert!(
            n_svg >= 4,
            "多目的レポートは複数チャートを埋め込む: {n_svg}"
        );
        // 散布図（front / dominated の凡例）が Outcome にある。
        assert!(html.contains("Pareto front"), "散布図の凡例");
        // MCDM 3手法の見出し。
        assert!(html.contains(">TOPSIS<"));
        assert!(html.contains(">VIKOR<"));
        assert!(html.contains(">PROMETHEE II<"));
    }

    #[test]
    fn single_objective_skips_mcdm_and_scatter() {
        let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
        let html = render_html(&report, ReportLang::En);

        assert_self_contained(&html);
        // 単目的では MCDM / 散布図 / 実行情報（extras なし）を出さない。
        assert!(!html.contains("id=\"mcdm\""), "単目的は MCDM を出さない");
        assert!(!html.contains("href=\"#mcdm\""), "目次に MCDM を出さない");
        assert!(!html.contains("Pareto front"), "単目的は散布図を出さない");
        assert!(
            !html.contains("id=\"execution\""),
            "extras 無しは実行情報なし"
        );
        // 単目的の主要セクションは存在する。
        assert!(html.contains("id=\"outcome\""));
        assert!(html.contains("id=\"convergence\""));
        assert!(html.contains("id=\"importance\""));
    }

    #[test]
    fn user_strings_are_escaped() {
        let mut meta = meta_single();
        meta.name = "<script>alert('x')</script>".to_string();
        meta.param_names = vec!["<b>a</b>".to_string(), "b".to_string()];
        let rows: Vec<TrialRow> = (0..12)
            .map(|i| row(i, &[("<b>a</b>", i as f64), ("b", 0.0)], &[i as f64]))
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &["<b>a</b>".to_string(), "b".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );
        let report = build_study_report(&meta, &df, None, &source(), &opts());
        let html = render_html(&report, ReportLang::En);

        // 生の <script> / <b> はエスケープされ、実体参照として現れる。
        assert!(
            !html.contains("<script>alert"),
            "スクリプトが素通りしている"
        );
        assert!(!html.contains("<b>a</b>"), "パラメータ名の生タグが素通り");
        assert!(html.contains("&lt;script&gt;alert"));
        assert!(html.contains("&lt;b&gt;a&lt;/b&gt;"));
    }

    #[test]
    fn output_is_deterministic() {
        let report = build_study_report(
            &meta_multi(),
            &df_multi(),
            Some(&extras_multi()),
            &source(),
            &opts(),
        );
        let a = render_html(&report, ReportLang::En);
        let b = render_html(&report, ReportLang::En);
        assert_eq!(a, b, "同一入力→バイト同一");
    }

    #[test]
    fn japanese_headings_smoke() {
        let report = build_study_report(
            &meta_multi(),
            &df_multi(),
            Some(&extras_multi()),
            &source(),
            &opts(),
        );
        let html = render_html(&report, ReportLang::Ja);
        assert!(html.contains("<html lang=\"ja\">"));
        for heading in [
            "まとめ",
            "最適化結果",
            "収束",
            "意思決定分析",
            "実行情報",
            "付録",
        ] {
            assert!(html.contains(heading), "日本語見出し欠落: {heading}");
        }
    }

    /// 制約付き multi の行を組み立てる。
    fn row_c(id: u32, params: &[(&str, f64)], objs: &[f64], cons: &[f64]) -> TrialRow {
        let mut r = row(id, params, objs);
        r.constraint_values = cons.to_vec();
        r
    }

    fn meta_multi_constrained() -> StudyMeta {
        let mut m = meta_multi();
        m.has_constraints = true;
        m
    }

    /// 目的空間の front = {trial0, trial1, trial2}。
    ///
    /// `all_infeasible = true` では全行が c=[0.4, -1.0]（合計は -0.6 と負だが
    /// 最大は 0.4 で違反）となり、feasible 解ゼロ → 目的空間前面への
    /// フォールバック + 違反マーク/注記の回帰ケースになる。
    fn df_multi_constrained(all_infeasible: bool) -> DataFrame {
        let pts = [
            (1.0, 4.0),
            (2.0, 2.0),
            (4.0, 1.0),
            (3.0, 3.0),
            (5.0, 5.0),
            (2.0, 3.0),
        ];
        let rows: Vec<TrialRow> = pts
            .iter()
            .enumerate()
            .map(|(i, &(o0, o1))| {
                let cons: &[f64] = if all_infeasible {
                    &[0.4, -1.0]
                } else {
                    &[-0.5, -0.25]
                };
                row_c(
                    i as u32,
                    &[("p", i as f64), ("q", (6 - i) as f64)],
                    &[o0, o1],
                    cons,
                )
            })
            .collect();
        DataFrame::from_trials(
            &rows,
            &["p".to_string(), "q".to_string()],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            2,
        )
    }

    #[test]
    fn all_infeasible_falls_back_and_marks_violations() {
        // feasible 解ゼロ → 目的空間前面へフォールバックし、違反マークと
        // フォールバック注記が出る。
        let report = build_study_report(
            &meta_multi_constrained(),
            &df_multi_constrained(true),
            None,
            &source(),
            &opts(),
        );
        let html = render_html(&report, ReportLang::En);

        // 列ヘッダは意味論込み。
        assert!(html.contains("max constraint (≤0 = feasible)"), "列ヘッダ");
        // 各行は sum なら -0.6（無印に見える）だが max は 0.4 → 違反マーク。
        assert!(
            html.contains("<td class=\"num infeasible\">0.4 ✗</td>"),
            "違反セルの赤字 + ✗ マーク（sum でなく max を表示）"
        );
        // フォールバックの注記が出る。
        assert!(
            html.contains("no trial satisfies all constraints"),
            "Pareto 表直下のフォールバック注記"
        );
        // 極値表の最良 trial も違反マーク付き。
        assert!(
            html.contains("<td class=\"num infeasible\">#"),
            "極値表の違反 trial マーク"
        );

        let ja = render_html(&report, ReportLang::Ja);
        assert!(ja.contains("最大制約値（≤0 で充足）"), "ja 列ヘッダ");
        assert!(ja.contains("件は制約違反です"), "ja 注記");
        assert!(ja.contains("フォールバック"), "ja フォールバック注記");

        // Markdown も同じ意味論。
        let md = render_markdown(&report, ReportLang::En);
        assert!(md.contains("max constraint (≤0 = feasible)"));
        assert!(md.contains("0.4 (infeasible)"));
        assert!(md.contains("no trial satisfies all constraints"));
        // 極値表: 両目的とも最小化で best は #0 (obj0=1.0) / #2 (obj1=1.0)。
        assert!(md.contains("#0 (infeasible)"), "極値表の違反マーク: {md}");
        let md_ja = render_markdown(&report, ReportLang::Ja);
        assert!(md_ja.contains("0.4（違反）"));
        assert!(md_ja.contains("件は制約違反です"));
    }

    #[test]
    fn infeasible_trial_is_excluded_from_front_when_feasible_exist() {
        // trial1 (2,2) は目的空間では front だが制約違反 → 前面から除外され、
        // 残る feasible 行で前面が再計算される（違反マーク・注記なし）。
        let pts = [(1.0, 4.0), (2.0, 2.0), (4.0, 1.0), (3.0, 3.0)];
        let rows: Vec<TrialRow> = pts
            .iter()
            .enumerate()
            .map(|(i, &(o0, o1))| {
                let cons: &[f64] = if i == 1 { &[0.4] } else { &[-0.5] };
                row_c(i as u32, &[("p", i as f64), ("q", 1.0)], &[o0, o1], cons)
            })
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &["p".to_string(), "q".to_string()],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            2,
        );
        let report = build_study_report(&meta_multi_constrained(), &df, None, &source(), &opts());

        let Outcome::MultiObj {
            pareto_table,
            scatter,
            ..
        } = &report.outcome
        else {
            panic!("multi-objective outcome expected");
        };
        let front_trials: Vec<u32> = pareto_table.iter().map(|t| t.trial_number).collect();
        assert!(
            !front_trials.contains(&1),
            "違反 trial1 は前面から除外: {front_trials:?}"
        );
        // trial1 除外後は (3,3) も (1,4)/(4,1) に支配されないため front 入り。
        assert!(front_trials.contains(&0) && front_trials.contains(&2));
        // 散布図では trial1 は feasible=false / on_front=false の点として残る。
        let p1 = scatter.iter().find(|p| p.trial_number == 1).unwrap();
        assert!(!p1.feasible && !p1.on_front);

        let html = render_html(&report, ReportLang::En);
        assert!(!html.contains("class=\"num infeasible\""), "違反マークなし");
        assert!(
            !html.contains("no trial satisfies all constraints"),
            "注記なし"
        );
    }

    #[test]
    fn duplicate_objective_trials_are_marked() {
        // trial1 と trial3 は同一目的値 (2,2) → 若い trial1 が正、trial3 に
        // duplicate_of = 1 が付き、両レンダラに凡例と併記が出る。
        let pts = [(1.0, 4.0), (2.0, 2.0), (4.0, 1.0), (2.0, 2.0)];
        let rows: Vec<TrialRow> = pts
            .iter()
            .enumerate()
            .map(|(i, &(o0, o1))| row(i as u32, &[("p", i as f64), ("q", 1.0)], &[o0, o1]))
            .collect();
        let df = DataFrame::from_trials(
            &rows,
            &["p".to_string(), "q".to_string()],
            &["obj0".to_string(), "obj1".to_string()],
            &[],
            &[],
            2,
        );
        let report = build_study_report(&meta_multi(), &df, None, &source(), &opts());

        let Outcome::MultiObj { pareto_table, .. } = &report.outcome else {
            panic!("multi-objective outcome expected");
        };
        let dup = pareto_table.iter().find(|t| t.trial_number == 3).unwrap();
        assert_eq!(dup.duplicate_of, Some(1));
        let first = pareto_table.iter().find(|t| t.trial_number == 1).unwrap();
        assert_eq!(first.duplicate_of, None);

        let html = render_html(&report, ReportLang::En);
        assert!(html.contains("(= #1)"), "HTML の併記");
        assert!(html.contains("identical to trial #N"), "HTML の凡例");
        let md = render_markdown(&report, ReportLang::Ja);
        assert!(md.contains("#3 (= #1)"), "MD の併記: {md}");
        assert!(md.contains("重複解"), "MD の凡例");
    }

    #[test]
    fn feasible_only_front_has_no_infeasible_mark_or_note() {
        let report = build_study_report(
            &meta_multi_constrained(),
            &df_multi_constrained(false),
            None,
            &source(),
            &opts(),
        );
        let html = render_html(&report, ReportLang::En);

        // 列自体は出る（制約ありスタディ）。
        assert!(html.contains("max constraint (≤0 = feasible)"));
        // 全 trial feasible → 違反マークも注記も出ない
        // （PAGE_CSS の `td.infeasible` 定義は常在のためセル class で判定）。
        assert!(!html.contains("class=\"num infeasible\""), "違反マークなし");
        assert!(!html.contains("trials violate constraints"), "注記なし");

        let md = render_markdown(&report, ReportLang::En);
        assert!(!md.contains("(infeasible)"));
        assert!(!md.contains("trials violate constraints"));
    }

    #[test]
    fn markdown_generated_at_includes_iso_utc() {
        let report = build_study_report(&meta_single(), &df_single(), None, &source(), &opts());
        let md = render_markdown(&report, ReportLang::En);
        // unix 秒だけでなく ISO UTC を併記する（決定論的変換）。
        assert!(
            md.contains("Generated at: 2023-11-14T22:13:20Z (unix 1700000000)"),
            "ISO UTC + unix 併記"
        );
    }

    #[test]
    fn zero_trials_does_not_panic() {
        let meta = meta_single();
        let df = DataFrame::from_trials(
            &[],
            &["a".to_string(), "b".to_string()],
            &["obj0".to_string()],
            &[],
            &[],
            0,
        );
        let report: StudyReport = build_study_report(&meta, &df, None, &source(), &opts());
        let en = render_html(&report, ReportLang::En);
        let ja = render_html(&report, ReportLang::Ja);
        assert_self_contained(&en);
        assert!(en.contains("id=\"key-findings\""));
        assert!(ja.contains("<html lang=\"ja\">"));
    }
}
