use super::formatting::{escape_csv_field, format_f64, CSV_DELIMITER};

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
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

/// Documentation.
///
/// Documentation.
pub(crate) fn serialize_csv_from_df(
    df: &crate::dataframe::DataFrame,
    indices: &[u32],
    columns: &[String],
) -> String {
    let n = df.row_count();
    let mut out = String::with_capacity(indices.len() * columns.len() * 10);

    let header_fields: Vec<String> = columns.iter().map(|c| escape_csv_field(c)).collect();
    out.push_str(&header_fields.join(&CSV_DELIMITER.to_string()));
    out.push('\n');

    for &idx in indices {
        let row = idx as usize;
        if row >= n {
            continue;
        }

        let mut fields = Vec::with_capacity(columns.len());
        for col in columns {
            let field = get_cell_value(df, row, col);
            fields.push(escape_csv_field(&field));
        }
        out.push_str(&fields.join(&CSV_DELIMITER.to_string()));
        out.push('\n');
    }

    out
}

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
/// Documentation.
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

/// Documentation.
///
/// Documentation.
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
