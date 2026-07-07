use super::formatting::{escape_csv_field, format_f64, sanitize_csv_text, CSV_DELIMITER};

/// 指定した行インデックスと列名（JSON 配列文字列）から CSV 文字列を生成する。
///
/// アクティブな DataFrame の値を参照し、列が空または DataFrame 未設定の場合は空文字列を返す。
///
/// 🟢 REQ-150〜REQ-153
pub fn serialize_csv(indices: &[u32], columns_json: &str) -> String {
    let columns: Vec<String> = parse_columns_json(columns_json);
    if columns.is_empty() {
        return String::new();
    }

    let result =
        crate::dataframe::with_active_df(|df| serialize_csv_from_df(df, indices, &columns));

    result.unwrap_or_default()
}

/// 指定 DataFrame から行インデックス・列名リストに基づき、ヘッダ付き CSV 文字列を生成する。
pub(crate) fn serialize_csv_from_df(
    df: &crate::dataframe::DataFrame,
    indices: &[u32],
    columns: &[String],
) -> String {
    let n = df.row_count();
    let mut out = String::with_capacity(indices.len() * columns.len() * 10);

    // ヘッダ（列名）はジャーナル由来のテキストなので数式ガードも適用する。
    let header_fields: Vec<String> = columns.iter().map(|c| sanitize_csv_text(c)).collect();
    out.push_str(&header_fields.join(&CSV_DELIMITER.to_string()));
    out.push('\n');

    for &idx in indices {
        let row = idx as usize;
        if row >= n {
            continue;
        }

        let mut fields = Vec::with_capacity(columns.len());
        for col in columns {
            // 文字列セル（カテゴリラベル・user attr）はテキストとして数式ガード、
            // 数値セルは format_f64 済みの安全な文字列なので構造クオートのみ。
            if df.get_numeric_column(col).is_some() || col == "trial_id" {
                fields.push(escape_csv_field(&get_cell_value(df, row, col)));
            } else {
                fields.push(sanitize_csv_text(&get_cell_value(df, row, col)));
            }
        }
        out.push_str(&fields.join(&CSV_DELIMITER.to_string()));
        out.push('\n');
    }

    out
}

/// 指定セルの値を文字列化する。trial_id は専用処理、数値列は `format_f64`、
/// 文字列列はそのままの値、該当なしは空文字列を返す。
fn get_cell_value(df: &crate::dataframe::DataFrame, row: usize, col: &str) -> String {
    if col == "trial_id" {
        return df
            .get_trial_id(row)
            .map(|id| id.to_string())
            .unwrap_or_default();
    }

    if let Some(vals) = df.get_numeric_column(col) {
        if let Some(&v) = vals.get(row) {
            return format_f64(v);
        }
        return String::new();
    }

    if let Some(vals) = df.get_string_column(col) {
        return vals.get(row).cloned().unwrap_or_default();
    }

    String::new()
}

/// `["col1","col2"]` 形式の JSON 配列文字列を列名のリストにパースする。
pub(crate) fn parse_columns_json(json: &str) -> Vec<String> {
    let trimmed = json.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    if inner.trim().is_empty() {
        return vec![];
    }

    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in inner.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            ',' if !in_string => {
                let s = current.trim().to_string();
                if !s.is_empty() {
                    result.push(s);
                }
                current.clear();
            }
            _ if in_string => current.push(ch),
            _ => {}
        }
    }
    let s = current.trim().to_string();
    if !s.is_empty() {
        result.push(s);
    }

    result
}
