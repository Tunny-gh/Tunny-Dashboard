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

/// 数式インジェクションを誘発する先頭文字（Excel 等が式として解釈する）。
pub(super) const FORMULA_LEADING_CHARS: [char; 4] = ['=', '+', '-', '@'];

/// テキストフィールド用のサニタイズ。先頭が数式文字なら `'` を前置して
/// スプレッドシートの式解釈を無効化した上で、構造クオートを適用する。
/// 数値をフォーマットした文字列には使わないこと（負数の `-` に `'` が付くため）。
pub(super) fn sanitize_csv_text(s: &str) -> String {
    if s.starts_with(FORMULA_LEADING_CHARS) {
        escape_csv_field(&format!("'{s}"))
    } else {
        escape_csv_field(s)
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
