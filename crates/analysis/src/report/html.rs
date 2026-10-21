//! Self-contained HTML report renderer.
//!
//! Produces a single page with zero external resource references and no JS.
//! Styles reference the CSS custom properties defined by
//! [`super::theme::css_variables`], and automatically follow light/dark via
//! `prefers-color-scheme`. Charts are embedded as SVG by calling into the
//! primitives in [`super::svg`], and every chart is paired with a
//! corresponding numeric table (the table is primary, the chart supplementary).
//!
//! The renderer follows a "compute nothing, only draw" principle: series and
//! statistics are used exactly as pre-computed by the builder (no dependence
//! on `HashMap` iteration order, so output is byte-identical for identical
//! input). All wording supports both en / ja, and the Key Finding wording
//! template is shared with Markdown via [`super::text`].
//!
//! ## Submodule layout
//!
//! - [`helpers`]: shared escaping / i18n / table-building primitives.
//! - [`header`]: page header + table of contents.
//! - [`findings`]: Key Findings section.
//! - [`outcome`]: best trial / top trials / Pareto front + scatter chart.
//! - [`analysis`]: convergence, parameter importance, objective statistics,
//!   correlations.
//! - [`decision`]: multi-criteria decision analysis (MCDM).
//! - [`execution`]: execution info + appendix.

use std::fmt::Write as _;

use super::model::StudyReport;
use super::theme;
use super::ReportLang;

mod analysis;
mod decision;
mod execution;
mod findings;
mod header;
mod helpers;
mod outcome;

use analysis::*;
use decision::*;
use execution::*;
use findings::*;
use header::*;
use helpers::*;
use outcome::*;

/// viewBox width for full-page-width charts (responsive, so only the relative ratio matters).
const CHART_W: f64 = 880.0;
/// Max number of objectives to draw a histogram for.
const MAX_HISTOGRAMS: usize = 4;

/// Renders a [`StudyReport`] to self-contained HTML.
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

/// Page body CSS (embedded in `<style>` following `css_variables()`).
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

#[cfg(test)]
mod tests;
