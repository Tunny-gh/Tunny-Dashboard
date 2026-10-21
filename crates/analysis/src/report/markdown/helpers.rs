//! Shared escaping / i18n text helpers used across the `markdown` report
//! submodules.

use crate::report::model::{Direction, ParamValue};
use crate::report::{format_number, ReportLang};

// =============================================================================
// Language / escape helpers
// =============================================================================

/// Returns either the en or ja string depending on the language.
pub(super) fn tr(lang: ReportLang, en: &'static str, ja: &'static str) -> &'static str {
    match lang {
        ReportLang::En => en,
        ReportLang::Ja => ja,
    }
}

/// Escapes a user-derived string to be safe as a Markdown table cell / safe
/// for inline use (used for both table cells and inline body text).
///
/// - `\` -> `\\`, `|` -> `\|`: protects the table structure. Processed in a
///   single pass character by character, so it avoids the ordering issue
///   that a sequential `replace` chain would have (a later replacement
///   double-escaping a `\` inserted by an earlier one).
/// - `&` -> `&amp;`, `<` -> `&lt;`: prevents XSS / structure breakage when
///   the output Markdown is converted to HTML downstream (e.g. rendered via
///   MCP). In Markdown these still display as the literal character via the
///   entity reference.
/// - `* _ [ ] #` -> `\*` etc.: prevents misinterpretation as Markdown
///   inline syntax (emphasis, links, headings), including user-derived
///   spans in Key Findings.
/// - Newlines (`\n` / `\r`) -> space: protects cell / row structure.
pub(super) fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '|' => out.push_str("\\|"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '*' => out.push_str("\\*"),
            '_' => out.push_str("\\_"),
            '[' => out.push_str("\\["),
            ']' => out.push_str("\\]"),
            '#' => out.push_str("\\#"),
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// Formats a user-derived name as an inline code span.
///
/// Backslash escaping has no effect inside inline code, so [`esc`] is not
/// applied; instead embedded backticks are handled per the CommonMark
/// rule: wrap the content in a run of backticks one longer than the
/// longest backtick run it contains, and pad with a space if the content
/// starts or ends with a backtick (the parser strips one space from each
/// end). Newlines are replaced with spaces.
pub(super) fn code_span(s: &str) -> String {
    let content: String = s
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let max_run = content.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(max_run + 1);
    if content.is_empty() {
        // An empty code span is not valid Markdown, so use a single space.
        return "` `".to_string();
    }
    if content.starts_with('`') || content.ends_with('`') {
        format!("{fence} {content} {fence}")
    } else {
        format!("{fence}{content}{fence}")
    }
}

/// Direction label.
pub(super) fn dir_label(lang: ReportLang, d: Direction) -> &'static str {
    match d {
        Direction::Minimize => tr(lang, "Minimize", "最小化"),
        Direction::Maximize => tr(lang, "Maximize", "最大化"),
    }
}

pub(super) fn param_val(v: &ParamValue) -> String {
    match v {
        ParamValue::Num(x) => format_number(*x),
        ParamValue::Cat(s) => esc(s),
    }
}

pub(super) fn yes_no(lang: ReportLang, b: bool) -> &'static str {
    if b {
        tr(lang, "yes", "はい")
    } else {
        tr(lang, "no", "いいえ")
    }
}

#[cfg(test)]
mod esc_tests {
    use super::{code_span, esc};

    #[test]
    fn escapes_backslash_before_pipe() {
        // Because processing is single-pass, escaping `\` and `|` in
        // `a\|b` does not collide, and the table structure is not broken.
        assert_eq!(esc("a\\|b"), "a\\\\\\|b");
        assert_eq!(esc("trail\\"), "trail\\\\");
    }

    #[test]
    fn escapes_html_amp_and_lt() {
        // Guards against XSS / structure breakage when converted to HTML downstream.
        assert_eq!(esc("a&b"), "a&amp;b");
        assert_eq!(esc("<script>"), "&lt;script>");
        // `&` is turned into an entity reference as-is and is not
        // double-escaped (since processing is single-pass, `&amp;` is not
        // reprocessed).
        assert_eq!(esc("&lt;"), "&amp;lt;");
    }

    #[test]
    fn escapes_markdown_inline_specials() {
        // Not misinterpreted as emphasis, link, or heading syntax.
        assert_eq!(esc("*em*"), "\\*em\\*");
        assert_eq!(esc("snake_case_name"), "snake\\_case\\_name");
        assert_eq!(esc("[link]"), "\\[link\\]");
        assert_eq!(esc("#tag"), "\\#tag");
    }

    #[test]
    fn newlines_become_spaces() {
        assert_eq!(esc("a\nb\rc"), "a b c");
    }

    #[test]
    fn code_span_wraps_plain_names() {
        assert_eq!(code_span("spearman_abs"), "`spearman_abs`");
    }

    #[test]
    fn code_span_handles_embedded_backticks() {
        // Wraps with a fence one longer than the content's longest backtick run.
        assert_eq!(code_span("a`b"), "``a`b``");
        assert_eq!(code_span("a``b"), "```a``b```");
        // Pads with a space when the content starts/ends with a backtick.
        assert_eq!(code_span("`lead"), "`` `lead ``");
        assert_eq!(code_span("trail`"), "`` trail` ``");
        // An empty string becomes a code span containing a single space.
        assert_eq!(code_span(""), "` `");
    }
}
