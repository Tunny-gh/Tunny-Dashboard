/// Documentation.
pub(super) const CSV_DELIMITER: char = ',';

/// Documentation.
pub(super) const NEEDS_QUOTING_CHARS: [char; 3] = [',', '\n', '"'];

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
pub(super) fn escape_csv_field(s: &str) -> String {
    if s.chars().any(|c| NEEDS_QUOTING_CHARS.contains(&c)) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Documentation.
///
/// Design:
/// Documentation.
/// Documentation.
/// Documentation.
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
