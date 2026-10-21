//! Self-contained report output.
//!
//! Distills Optuna optimization results into a structured [`StudyReport`] and
//! renders it to JSON / Markdown / (HTML in a later phase). The report model
//! holds language-independent structured facts, and wording (en / ja) is
//! handled by the renderer templates. Markdown / JSON are meant to be passed
//! directly to an LLM by a future MCP server.
//!
//! ## Module layout
//!
//! - [`model`] — the [`StudyReport`] struct tree (`serde::Serialize`)
//! - [`builder`] — `(StudyMeta, DataFrame, StudyExtras)` → [`StudyReport`]
//! - [`findings`] — deterministic generation of Key Findings (summary)
//! - [`markdown`] — Markdown renderer (primary use case: LLM consumption)
//! - [`html`] — self-contained HTML renderer (embeds SVG charts)
//! - [`text`] — Key Finding wording templates (shared by markdown / html)

pub mod builder;
pub mod findings;
pub mod html;
pub mod markdown;
pub mod model;
pub mod svg;
pub mod text;
pub mod theme;

pub use builder::build_study_report;
pub use html::render_html;
pub use markdown::render_markdown;
pub use model::*;

/// Report and provenance acquired together; the wire rendering remains unchanged.
pub struct ComputedReport {
    pub key: crate::session::ResultKey,
    pub rows: Vec<tdashboard_study::TrialKey>,
    pub report: std::sync::Arc<StudyReport>,
}

pub fn generate_report(
    input: tdashboard_study::StudyInput,
    source: &ReportSource,
    opts: &ReportOptions,
) -> Result<ComputedReport, String> {
    let table = std::sync::Arc::new(crate::projection::project(&input));
    generate_prepared_report(input, table, source, opts)
}

/// Reuse the owner's prepared table, including its retained discovered schema.
/// Inputs and all-state extras must belong to the same acquired stamp.
pub fn generate_prepared_report(
    input: tdashboard_study::StudyInput,
    table: std::sync::Arc<crate::projection::Projection>,
    source: &ReportSource,
    opts: &ReportOptions,
) -> Result<ComputedReport, String> {
    if table.stamp != input.stamp()
        || table.dataframe.provenance() != Some(input.stamp())
        || table.dataframe.trial_keys() != Some(table.rows.as_slice())
    {
        return Err("Report table belongs to another input".into());
    }
    use crate::session::*;
    let mut session = AnalysisSession::default();
    let request = AnalysisRequest {
        kind: AnalysisKind::Report,
        conditions: vec![
            ("lang".into(), Condition::Text(format!("{:?}", opts.lang))),
            ("top_n".into(), Condition::Integer(opts.top_n as u64)),
            (
                "max_heatmap_params".into(),
                Condition::Integer(opts.max_heatmap_params as u64),
            ),
            (
                "skip_decision_sections".into(),
                Condition::Bool(opts.skip_decision_sections),
            ),
            (
                "storage_display".into(),
                Condition::Text(source.storage_display.clone()),
            ),
            (
                "generated_at".into(),
                Condition::Text(format!("{:?}", source.generated_at_unix)),
            ),
        ],
    };
    let mut prepared = session
        .prepare(vec![input], request, &[])
        .map_err(|e| format!("Report unavailable: {e:?}"))?;
    prepared.tables = vec![table];
    let WorkOrShared::Work(work) = session.start(prepared, ReuseOrRerun::Rerun)? else {
        unreachable!()
    };
    let completion = work.execute(|prepared| {
        let table = &prepared.tables[0];
        Ok(build_study_report(
            &table.meta,
            &table.dataframe,
            Some(&table.extras),
            source,
            opts,
        ))
    });
    let result = session.complete(completion).ok_or("Report superseded")?;
    Ok(ComputedReport {
        key: result.key().clone(),
        rows: result.rows()[0].clone(),
        report: result.payload::<StudyReport>().ok_or("Report missing")??,
    })
}

/// Rendering language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReportLang {
    /// English.
    #[default]
    En,
    /// Japanese.
    Ja,
}

/// Report generation options.
#[derive(Debug, Clone, Copy)]
pub struct ReportOptions {
    /// Default rendering language (`render_*` can override with an explicit argument).
    pub lang: ReportLang,
    /// Number of rows in the top-N table (default 10).
    pub top_n: usize,
    /// Max number of parameters in the correlation heatmap (default 15).
    pub max_heatmap_params: usize,
    /// If `true`, skip computing the MCDM section and the correlation section
    /// (for lightweight use cases such as study_summary). Key Findings and
    /// the Pareto table's TOPSIS ordering are still preserved (default `false`).
    pub skip_decision_sections: bool,
}

impl Default for ReportOptions {
    fn default() -> Self {
        ReportOptions {
            lang: ReportLang::En,
            top_n: 10,
            max_heatmap_params: 15,
            skip_decision_sections: false,
        }
    }
}

/// Source information supplied by the caller.
///
/// If `storage_display` is an RDB URL, it **must** be the masked
/// (`RdbUrl::masked()`) string (never leave a raw password in the report).
/// `generated_at_unix` is supplied by the caller since core has no clock;
/// `None` omits the timestamp field.
#[derive(Debug, Clone)]
pub struct ReportSource {
    /// Storage display name (RDBs are masked).
    pub storage_display: String,
    /// Generation timestamp (unix seconds). `None` omits it.
    pub generated_at_unix: Option<i64>,
}

/// Formats an f64 to 4 significant digits (shared formatter across renderers).
///
/// - Integer values are shown as plain integers (no trailing `.0`)
/// - Non-integers are rounded to 4 significant digits, with trailing zeros
///   and the trailing dot stripped
/// - `NaN` / `±inf` render as `"NaN"` / `"inf"` / `"-inf"` respectively
pub fn format_number(value: f64) -> String {
    format_sig(value, 4)
}

/// Formats a percentage value (rounded to an integer).
///
/// Shared by the three call sites — Markdown, HTML, and the Key Finding
/// template — that previously each duplicated the same logic.
pub(crate) fn pct(x: f64) -> String {
    format!("{x:.0}")
}

fn format_sig(value: f64, sig: usize) -> String {
    if value.is_nan() {
        return "NaN".to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    if value == 0.0 {
        return "0".to_string();
    }
    // Integer values are shown as plain integers.
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (sig as i32 - 1 - magnitude).max(0) as usize;
    let s = format!("{value:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod format_tests {
    use super::format_number;

    #[test]
    fn integers_render_without_decimals() {
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(-7.0), "-7");
        assert_eq!(format_number(1000.0), "1000");
    }

    #[test]
    fn four_significant_digits_no_trailing_zeros() {
        assert_eq!(format_number(1.23456), "1.235");
        assert_eq!(format_number(0.001234), "0.001234");
        assert_eq!(format_number(12345.6), "12346");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(0.5), "0.5");
    }

    #[test]
    fn non_finite() {
        assert_eq!(format_number(f64::NAN), "NaN");
        assert_eq!(format_number(f64::INFINITY), "inf");
        assert_eq!(format_number(f64::NEG_INFINITY), "-inf");
    }
}
