//! Shared escaping / i18n / table-building primitives used across the
//! `html` report submodules.

use std::fmt::Write as _;

use crate::report::model::{Direction, ParamValue};
use crate::report::svg;
use crate::report::{format_number, ReportLang};

// =============================================================================
// Escape / language helpers
// =============================================================================

/// Escapes for HTML text nodes / attribute values (shared with `svg`).
pub(super) fn esc(s: &str) -> String {
    svg::escape_xml(s)
}

/// Returns the en or ja variant depending on the language.
pub(super) fn tr(lang: ReportLang, en: &'static str, ja: &'static str) -> &'static str {
    match lang {
        ReportLang::En => en,
        ReportLang::Ja => ja,
    }
}

pub(super) fn dir_label(lang: ReportLang, d: Direction) -> &'static str {
    match d {
        Direction::Minimize => tr(lang, "Minimize", "最小化"),
        Direction::Maximize => tr(lang, "Maximize", "最大化"),
    }
}

pub(super) fn yes_no(lang: ReportLang, b: bool) -> &'static str {
    if b {
        tr(lang, "yes", "はい")
    } else {
        tr(lang, "no", "いいえ")
    }
}

/// Returns a parameter value as `(display string, is numeric)`.
pub(super) fn param_value(v: &ParamValue) -> (String, bool) {
    match v {
        ParamValue::Num(x) => (format_number(*x), true),
        ParamValue::Cat(s) => (s.clone(), false),
    }
}

// =============================================================================
// Table output helpers
// =============================================================================

pub(super) fn open_table(s: &mut String) {
    s.push_str("<div class=\"table-wrap\"><table>\n");
}

pub(super) fn close_table(s: &mut String) {
    s.push_str("</table></div>\n");
}

/// Writes out a `<th>` cell.
pub(super) fn th(s: &mut String, text: &str, numeric: bool) {
    let cls = if numeric { " class=\"num\"" } else { "" };
    let _ = write!(s, "<th{cls}>{}</th>", esc(text));
}

/// Writes out a `<td>` cell.
pub(super) fn td(s: &mut String, text: &str, numeric: bool) {
    let cls = if numeric { " class=\"num\"" } else { "" };
    let _ = write!(s, "<td{cls}>{}</td>", esc(text));
}

pub(super) fn meta_row(s: &mut String, key: &str, value: &str) {
    let _ = writeln!(
        s,
        "<li><span class=\"k\">{}</span><span class=\"v\">{}</span></li>",
        esc(key),
        esc(value)
    );
}

pub(super) fn fact_row(s: &mut String, key: &str, value: &str) {
    let _ = writeln!(
        s,
        "<li><span class=\"k\">{}</span><span class=\"v\">{}</span></li>",
        esc(key),
        esc(value)
    );
}
