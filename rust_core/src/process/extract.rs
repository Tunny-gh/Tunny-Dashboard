//! Output extraction: pulling numeric objective / constraint values out of a
//! command's captured text (stdout or a file) via regex, a JSON path, or a CSV
//! cell.

use serde_json::Value;

use super::definition::{CsvColumn, CsvRow, Extractor};

/// Extracts a numeric value from `text` using `extractor`. The result must be
/// finite — a non-finite output (`inf` / `nan`) is an extraction error, so a
/// diverged solver is treated as a failed evaluation rather than recorded as a
/// finite-looking success.
pub fn extract_value(extractor: &Extractor, text: &str) -> Result<f64, String> {
    let value = match extractor {
        Extractor::Regex { pattern } => extract_regex(pattern, text),
        Extractor::JsonPath { path } => extract_json_path(path, text),
        Extractor::Csv {
            row,
            column,
            has_header,
        } => extract_csv(row, column, *has_header, text),
    }?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("extracted value is not finite: {value}"))
    }
}

/// First capture group (or the whole match if there is no group) of `pattern`,
/// parsed as `f64`.
fn extract_regex(pattern: &str, text: &str) -> Result<f64, String> {
    let re = regex::Regex::new(pattern).map_err(|e| format!("invalid regex \"{pattern}\": {e}"))?;
    let caps = re
        .captures(text)
        .ok_or_else(|| format!("regex \"{pattern}\" did not match the output"))?;
    // Group 1 if present, else the whole match (group 0).
    let matched = caps
        .get(1)
        .or_else(|| caps.get(0))
        .ok_or_else(|| format!("regex \"{pattern}\" matched nothing"))?;
    parse_number(matched.as_str().trim())
        .ok_or_else(|| format!("regex match \"{}\" is not a number", matched.as_str()))
}

/// Follows a dotted `path` (object keys and array indices) into the JSON in
/// `text` and parses the addressed value as a number.
fn extract_json_path(path: &str, text: &str) -> Result<f64, String> {
    let root: Value =
        serde_json::from_str(text).map_err(|e| format!("output is not valid JSON: {e}"))?;
    let mut cur = &root;
    for segment in path.split('.').filter(|s| !s.is_empty()) {
        cur = match cur {
            Value::Object(map) => map
                .get(segment)
                .ok_or_else(|| format!("JSON path segment \"{segment}\" not found"))?,
            Value::Array(arr) => {
                let idx: usize = segment.parse().map_err(|_| {
                    format!("JSON path segment \"{segment}\" is not an array index")
                })?;
                arr.get(idx)
                    .ok_or_else(|| format!("JSON array index {idx} out of range"))?
            }
            _ => {
                return Err(format!(
                    "JSON path \"{path}\" descends into a non-container at \"{segment}\""
                ))
            }
        };
    }
    json_as_number(cur).ok_or_else(|| format!("JSON value at \"{path}\" is not a number"))
}

/// Extracts a cell from CSV `text`. Non-empty lines are rows; when `has_header`
/// is set, line 0 is the header (data rows start at line 1) and it is consulted
/// for [`CsvColumn::Header`]. `CsvRow` indices always count from the first data
/// row, independent of the column selector. Fields are comma-separated and
/// trimmed.
fn extract_csv(
    row: &CsvRow,
    column: &CsvColumn,
    has_header: bool,
    text: &str,
) -> Result<f64, String> {
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return Err("CSV output is empty".to_string());
    }

    // Data rows and the (optional) header are split the same way regardless of
    // the column selector, so `CsvRow` indices have one consistent origin.
    let (header, data_lines): (Option<&&str>, &[&str]) = if has_header {
        (Some(&lines[0]), &lines[1..])
    } else {
        (None, &lines[..])
    };

    let col_index = match column {
        CsvColumn::Index { index } => *index,
        CsvColumn::Header { name } => {
            let header = header.ok_or_else(|| {
                format!("CSV column \"{name}\" needs has_header = true to resolve the header")
            })?;
            split_csv(header)
                .iter()
                .position(|h| h == name)
                .ok_or_else(|| format!("CSV header \"{name}\" not found"))?
        }
    };

    if data_lines.is_empty() {
        return Err("CSV has no data rows".to_string());
    }
    let line = match row {
        CsvRow::Index { index } => data_lines
            .get(*index)
            .ok_or_else(|| format!("CSV data row {index} out of range"))?,
        CsvRow::Last => data_lines
            .last()
            .expect("data_lines is non-empty (checked above)"),
    };
    let fields = split_csv(line);
    let cell = fields
        .get(col_index)
        .ok_or_else(|| format!("CSV column {col_index} out of range"))?;
    parse_number(cell.trim()).ok_or_else(|| format!("CSV cell \"{cell}\" is not a number"))
}

fn split_csv(line: &str) -> Vec<String> {
    line.split(',').map(|f| f.trim().to_string()).collect()
}

/// Parses a number, tolerating surrounding whitespace and a leading `+`.
///
/// The whole trimmed token must be a valid `f64` — a partial parse is *not*
/// attempted, because taking the leading numeric run silently misreads values
/// like `1_000` (→ 1) or `0x1F` (→ 0), corrupting the objective with no error.
/// If a tool prints a number with a trailing unit, use a regex extractor whose
/// capture group isolates the number.
fn parse_number(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

/// Interprets a JSON value as a number (numbers directly, numeric strings by
/// parsing, booleans as 0/1).
fn json_as_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => parse_number(s),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_extracts_first_group() {
        let e = Extractor::Regex {
            pattern: r"weight\s*=\s*([-0-9.eE+]+)".to_string(),
        };
        assert_eq!(
            extract_value(&e, "solve ok\nweight = 12.5 kg").unwrap(),
            12.5
        );
    }

    #[test]
    fn regex_without_group_uses_whole_match() {
        let e = Extractor::Regex {
            pattern: r"-?\d+\.\d+".to_string(),
        };
        assert_eq!(extract_value(&e, "result 3.75 done").unwrap(), 3.75);
    }

    #[test]
    fn regex_no_match_is_error() {
        let e = Extractor::Regex {
            pattern: r"x=(\d+)".to_string(),
        };
        assert!(extract_value(&e, "nothing here")
            .unwrap_err()
            .contains("did not match"));
    }

    #[test]
    fn json_path_descends_objects_and_arrays() {
        let text = r#"{"results": {"objectives": [1.0, 42.5]}}"#;
        let e = Extractor::JsonPath {
            path: "results.objectives.1".to_string(),
        };
        assert_eq!(extract_value(&e, text).unwrap(), 42.5);
    }

    #[test]
    fn json_path_missing_key_is_error() {
        let e = Extractor::JsonPath {
            path: "a.b".to_string(),
        };
        assert!(extract_value(&e, r#"{"a": {}}"#)
            .unwrap_err()
            .contains("not found"));
    }

    #[test]
    fn csv_by_header_and_last_row() {
        let text = "step,loss\n0,1.5\n1,0.8\n2,0.25\n";
        let e = Extractor::Csv {
            row: CsvRow::Last,
            column: CsvColumn::Header {
                name: "loss".to_string(),
            },
            has_header: true,
        };
        assert_eq!(extract_value(&e, text).unwrap(), 0.25);
    }

    #[test]
    fn csv_by_index_without_header() {
        let text = "1.0, 2.0, 3.0\n4.0, 5.0, 6.0\n";
        let e = Extractor::Csv {
            row: CsvRow::Index { index: 1 },
            column: CsvColumn::Index { index: 2 },
            has_header: false,
        };
        assert_eq!(extract_value(&e, text).unwrap(), 6.0);
    }

    #[test]
    fn csv_row_index_counts_from_first_data_row_with_header() {
        // has_header=true: row 0 is the first DATA row (after the header),
        // consistent whether the column is selected by index or by name.
        let text = "a,b\n10,20\n30,40\n";
        let by_index = Extractor::Csv {
            row: CsvRow::Index { index: 0 },
            column: CsvColumn::Index { index: 1 },
            has_header: true,
        };
        let by_header = Extractor::Csv {
            row: CsvRow::Index { index: 0 },
            column: CsvColumn::Header {
                name: "b".to_string(),
            },
            has_header: true,
        };
        assert_eq!(extract_value(&by_index, text).unwrap(), 20.0);
        assert_eq!(extract_value(&by_header, text).unwrap(), 20.0);
    }

    #[test]
    fn csv_header_column_requires_has_header() {
        let e = Extractor::Csv {
            row: CsvRow::Last,
            column: CsvColumn::Header {
                name: "loss".to_string(),
            },
            has_header: false,
        };
        assert!(extract_value(&e, "loss\n0.5\n")
            .unwrap_err()
            .contains("has_header"));
    }

    #[test]
    fn csv_out_of_range_is_error() {
        let text = "1.0,2.0\n";
        let e = Extractor::Csv {
            row: CsvRow::Index { index: 0 },
            column: CsvColumn::Index { index: 9 },
            has_header: false,
        };
        assert!(extract_value(&e, text)
            .unwrap_err()
            .contains("out of range"));
    }

    #[test]
    fn ambiguous_or_nonfinite_values_are_errors() {
        // A value with a trailing separator/unit is not silently truncated.
        let thousands = Extractor::Regex {
            pattern: r"cost=(\S+)".to_string(),
        };
        assert!(extract_value(&thousands, "cost=1_000").is_err());
        // inf / nan outputs are extraction errors (failed evaluation).
        let val = Extractor::Regex {
            pattern: r"f=(\S+)".to_string(),
        };
        assert!(extract_value(&val, "f=inf").unwrap_err().contains("finite"));
        assert!(extract_value(&val, "f=nan").unwrap_err().contains("finite"));
    }
}
