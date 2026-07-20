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
//!
//! ## Submodule layout
//!
//! - [`helpers`]: shared escaping / i18n text primitives.
//! - [`header`]: meta line + Key Findings section.
//! - [`outcome`]: best trial / top trials / Pareto front table.
//! - [`analysis`]: convergence, parameter importance, objective statistics,
//!   correlations.
//! - [`decision`]: multi-criteria decision analysis (MCDM).
//! - [`execution`]: execution info + reproduction info.

use std::fmt::Write as _;

use super::model::StudyReport;
use super::ReportLang;

mod analysis;
mod decision;
mod execution;
mod header;
mod helpers;
mod outcome;

use analysis::*;
use decision::*;
use execution::*;
use header::*;
use helpers::*;
use outcome::*;

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
