/// CSV のフィールド区切り文字。
pub(super) const CSV_DELIMITER: char = ',';

/// クオートが必要になる文字（区切り文字・改行・ダブルクオート）。
pub(super) const NEEDS_QUOTING_CHARS: [char; 3] = [',', '\n', '"'];

/// 構造クオート（カンマ・改行・ダブルクオートを含む場合のみ `"` で囲み、内部の `"` は `""` にエスケープ）を適用する。
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

/// f64 を CSV 出力用の文字列に整形する。整数値は小数点なし、非有限値（NaN/inf）は空欄、
/// それ以外は小数点以下10桁で丸めた上で末尾の余分な0を除去する。
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
