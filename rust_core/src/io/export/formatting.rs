/// CSV field delimiter character.
pub(super) const CSV_DELIMITER: char = ',';

/// Characters that require quoting (delimiter, newline, double quote).
pub(super) const NEEDS_QUOTING_CHARS: [char; 3] = [',', '\n', '"'];

/// Applies structural quoting (wraps in `"` only if the value contains a comma,
/// newline, or double quote, escaping any inner `"` as `""`).
pub(super) fn escape_csv_field(s: &str) -> String {
    if s.chars().any(|c| NEEDS_QUOTING_CHARS.contains(&c)) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Leading characters that can trigger formula injection (interpreted as a
/// formula by Excel and similar spreadsheet applications).
pub(super) const FORMULA_LEADING_CHARS: [char; 4] = ['=', '+', '-', '@'];

/// Sanitization for text fields. If the value starts with a formula character,
/// prefixes it with `'` to disable spreadsheet formula interpretation, then
/// applies structural quoting.
/// Do not use this on formatted numeric strings (a negative number's leading
/// `-` would get a `'` prefixed).
pub(super) fn sanitize_csv_text(s: &str) -> String {
    if s.starts_with(FORMULA_LEADING_CHARS) {
        escape_csv_field(&format!("'{s}"))
    } else {
        escape_csv_field(s)
    }
}

/// Formats an f64 as a string for CSV output. Integer values are written without
/// a decimal point, non-finite values (NaN/inf) become an empty field, and other
/// values are rounded to 10 decimal places with trailing zeros trimmed.
pub(super) fn format_f64(v: f64) -> String {
    if v.is_nan() || v.is_infinite() {
        return String::new();
    }
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{}", v as i64);
    }
    let s = format!("{:.10}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}
