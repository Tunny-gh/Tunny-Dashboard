use super::formatting::{escape_csv_field, format_f64, sanitize_csv_text, CSV_DELIMITER};

/// Generates a CSV string from the given row indices and column names (a JSON
/// array string).
///
/// References the values in the active DataFrame; returns an empty string if
/// the columns are empty or no DataFrame is set.
///
/// 🟢 REQ-150~REQ-153
pub fn serialize_csv(indices: &[u32], columns_json: &str) -> String {
    let columns: Vec<String> = parse_columns_json(columns_json);
    if columns.is_empty() {
        return String::new();
    }

    let result =
        crate::dataframe::with_active_df(|df| serialize_csv_from_df(df, indices, &columns));

    result.unwrap_or_default()
}

/// Generates a CSV string with a header from the given DataFrame, based on the
/// row indices and column name list.
pub(crate) fn serialize_csv_from_df(
    df: &crate::dataframe::DataFrame,
    indices: &[u32],
    columns: &[String],
) -> String {
    let n = df.row_count();
    let mut out = String::with_capacity(indices.len() * columns.len() * 10);

    // The header (column names) originates from journal text, so the formula guard is applied too.
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
            // String cells (category labels, user attrs) get the formula guard as
            // text; numeric cells are already safe strings from format_f64, so
            // only structural quoting is applied.
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

/// Stringifies the value of the given cell. trial_id gets dedicated handling,
/// numeric columns use `format_f64`, string columns use the value as-is, and
/// missing values return an empty string.
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

/// Parses a JSON array string in the form `["col1","col2"]` into a list of column names.
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
