use super::formatting::{escape_csv_field, format_f64, sanitize_csv_text, CSV_DELIMITER};

/// Generates a CSV string from the given row indices and column names (a JSON
/// array string).
///
/// Uses the supplied table; returns an empty string when the columns are empty.
///
/// 🟢 REQ-150~REQ-153
pub fn serialize_csv(
    df: &crate::dataframe::DataFrame,
    indices: &[u32],
    columns_json: &str,
) -> String {
    let columns: Vec<String> = parse_columns_json(columns_json);
    if columns.is_empty() {
        return String::new();
    }

    serialize_csv_from_df(df, indices, &columns)
}

/// Generates a CSV string with a header from the given DataFrame, based on the
/// row indices and column name list.
pub(crate) fn serialize_csv_from_df(
    df: &crate::dataframe::DataFrame,
    indices: &[u32],
    columns: &[String],
) -> String {
    let mut out = String::with_capacity(indices.len() * columns.len() * 10);

    // The header (column names) originates from journal text, so the formula guard is applied too.
    let header_fields: Vec<String> = columns.iter().map(|c| sanitize_csv_text(c)).collect();
    out.push_str(&header_fields.join(&CSV_DELIMITER.to_string()));
    out.push('\n');

    for row in tdashboard_analysis::export::select_rows(df, indices, columns) {
        let mut fields = Vec::with_capacity(columns.len());
        for cell in row {
            match cell {
                tdashboard_analysis::export::ExportCell::Numeric(value) => {
                    fields.push(escape_csv_field(&format_f64(value)))
                }
                tdashboard_analysis::export::ExportCell::Text(value) => {
                    fields.push(sanitize_csv_text(&value))
                }
                tdashboard_analysis::export::ExportCell::Empty => fields.push(String::new()),
            }
        }
        out.push_str(&fields.join(&CSV_DELIMITER.to_string()));
        out.push('\n');
    }

    out
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
