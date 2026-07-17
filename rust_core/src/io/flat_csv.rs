//! Import of optimization results in flat CSV format (1 row = 1 trial).
//!
//! Supports output from external optimization programs other than the Optuna
//! Journal. Column roles are determined from the header's label prefix:
//!   - `in:<name>`  → a parameter (variable)
//!   - `out:<name>` → an objective function evaluation value
//!   - `img`        → an artifact (filename of an image or similar file
//!     located in the same directory as the CSV)
//!
//! All other columns are ingested as user_attr (numeric if the value parses as
//! a number, otherwise string). Since the CSV carries no direction information,
//! all objectives are treated as Minimize (the same default used for an unknown
//! direction in the Journal).

use std::collections::HashMap;

use crate::dataframe::{DataFrame, TrialRow};
use crate::io::journal::parser::{OptimizationDirection, StudyMeta};

/// Result of parsing a flat CSV.
pub struct FlatCsvParseResult {
    /// Metadata for the single study (`study_id` is fixed at 0).
    pub meta: StudyMeta,
    /// The constructed DataFrame.
    pub dataframe: DataFrame,
    /// trial_id → image filename (from the `img` column). Rows with an empty cell are excluded.
    pub images: Vec<(u32, String)>,
}

/// A column's role, determined from the header's prefix.
enum ColumnRole {
    Param(String),
    Objective(String),
    /// The `img` column. Holds an artifact's filename.
    Image,
    /// Anything other than `in:`/`out:`/`img`. Ingested as a user_attr.
    UserAttr(String),
}

fn classify_header(header: &str) -> ColumnRole {
    let h = header.trim();
    if let Some(name) = h.strip_prefix("in:") {
        ColumnRole::Param(name.trim().to_string())
    } else if let Some(name) = h.strip_prefix("out:") {
        ColumnRole::Objective(name.trim().to_string())
    } else if h.eq_ignore_ascii_case("img") {
        ColumnRole::Image
    } else {
        ColumnRole::UserAttr(h.to_string())
    }
}

/// A minimal RFC 4180-compliant CSV line parser. Handles double-quote wrapping,
/// `""` escaping, and commas inside quotes. Assumes newlines never appear inside
/// quotes (1 line = 1 record).
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut field));
            }
            _ => field.push(c),
        }
    }
    fields.push(field);
    fields
}

/// Parses a flat CSV byte slice and builds a single study.
///
/// `study_name` is the study name (typically the filename). Returns an error message on failure.
pub fn parse_flat_csv(data: &[u8], study_name: &str) -> Result<FlatCsvParseResult, String> {
    let text = String::from_utf8_lossy(data);
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());

    let header_line = lines.next().ok_or("CSV is empty")?;
    let roles: Vec<ColumnRole> = parse_csv_line(header_line)
        .iter()
        .map(|h| classify_header(h))
        .collect();

    // Build a per-role name list while preserving column order.
    let mut param_names: Vec<String> = Vec::new();
    let mut objective_names: Vec<String> = Vec::new();
    for role in &roles {
        match role {
            ColumnRole::Param(n) => param_names.push(n.clone()),
            ColumnRole::Objective(n) => objective_names.push(n.clone()),
            _ => {}
        }
    }
    if objective_names.is_empty() {
        return Err(
            "CSV has no objective columns (expected at least one 'out:' header)".to_string(),
        );
    }

    // Collect all cells as strings first, to later determine whether each
    // parameter column is numeric or categorical.
    // param_raw[name] = the raw string for each row.
    let mut param_raw: HashMap<String, Vec<String>> = param_names
        .iter()
        .map(|n| (n.clone(), Vec::new()))
        .collect();
    let mut obj_raw: HashMap<String, Vec<String>> = objective_names
        .iter()
        .map(|n| (n.clone(), Vec::new()))
        .collect();
    // user_attr column names (in order of appearance).
    let mut user_attr_names: Vec<String> = Vec::new();
    let mut user_attr_raw: HashMap<String, Vec<String>> = HashMap::new();
    for role in &roles {
        if let ColumnRole::UserAttr(n) = role {
            if !user_attr_raw.contains_key(n) {
                user_attr_names.push(n.clone());
                user_attr_raw.insert(n.clone(), Vec::new());
            }
        }
    }

    let mut images: Vec<(u32, String)> = Vec::new();
    let mut row_count: u32 = 0;

    for line in lines {
        let fields = parse_csv_line(line);
        let trial_id = row_count;
        for (idx, role) in roles.iter().enumerate() {
            let cell = fields.get(idx).map(|s| s.trim()).unwrap_or("");
            match role {
                ColumnRole::Param(n) => param_raw.get_mut(n).unwrap().push(cell.to_string()),
                ColumnRole::Objective(n) => obj_raw.get_mut(n).unwrap().push(cell.to_string()),
                ColumnRole::Image => {
                    if !cell.is_empty() {
                        images.push((trial_id, cell.to_string()));
                    }
                }
                ColumnRole::UserAttr(n) => user_attr_raw.get_mut(n).unwrap().push(cell.to_string()),
            }
        }
        row_count += 1;
    }

    if row_count == 0 {
        return Err("CSV has a header but no data rows".to_string());
    }

    // Determine numeric-ness per parameter column: numeric if every row parses
    // as f64, otherwise categorical.
    // param_bounds takes the observed min/max of numeric columns (used as the
    // search box for surrogate optimization).
    let mut param_numeric: HashMap<String, Vec<f64>> = HashMap::new();
    let mut param_category: HashMap<String, Vec<String>> = HashMap::new();
    let mut param_bounds: HashMap<String, (f64, f64)> = HashMap::new();
    for name in &param_names {
        let raw = &param_raw[name];
        let parsed: Option<Vec<f64>> = raw.iter().map(|s| s.parse::<f64>().ok()).collect();
        match parsed {
            Some(vals) => {
                if let (Some(&lo), Some(&hi)) = (
                    vals.iter().min_by(|a, b| a.total_cmp(b)),
                    vals.iter().max_by(|a, b| a.total_cmp(b)),
                ) {
                    param_bounds.insert(name.clone(), (lo, hi));
                }
                param_numeric.insert(name.clone(), vals);
            }
            None => {
                param_category.insert(name.clone(), raw.clone());
            }
        }
    }

    // Determine numeric vs. string for each user_attr column.
    let mut user_attr_numeric_names: Vec<String> = Vec::new();
    let mut user_attr_string_names: Vec<String> = Vec::new();
    let mut ua_numeric: HashMap<String, Vec<f64>> = HashMap::new();
    let mut ua_string: HashMap<String, Vec<String>> = HashMap::new();
    for name in &user_attr_names {
        let raw = &user_attr_raw[name];
        let parsed: Option<Vec<f64>> = raw.iter().map(|s| s.parse::<f64>().ok()).collect();
        match parsed {
            Some(vals) => {
                user_attr_numeric_names.push(name.clone());
                ua_numeric.insert(name.clone(), vals);
            }
            None => {
                user_attr_string_names.push(name.clone());
                ua_string.insert(name.clone(), raw.clone());
            }
        }
    }

    // Parse the objective columns (non-numeric values become NaN).
    let obj_parsed: HashMap<String, Vec<f64>> = objective_names
        .iter()
        .map(|name| {
            let vals: Vec<f64> = obj_raw[name]
                .iter()
                .map(|s| s.parse::<f64>().unwrap_or(f64::NAN))
                .collect();
            (name.clone(), vals)
        })
        .collect();

    // Build row-oriented TrialRow entries.
    let mut trial_rows: Vec<TrialRow> = Vec::with_capacity(row_count as usize);
    for row in 0..row_count as usize {
        let mut param_display: HashMap<String, f64> = HashMap::new();
        let mut param_category_label: HashMap<String, String> = HashMap::new();
        for name in &param_names {
            if let Some(vals) = param_numeric.get(name) {
                param_display.insert(name.clone(), vals[row]);
            } else if let Some(vals) = param_category.get(name) {
                param_category_label.insert(name.clone(), vals[row].clone());
            }
        }
        let objective_values: Vec<f64> =
            objective_names.iter().map(|n| obj_parsed[n][row]).collect();
        let mut user_attrs_numeric: HashMap<String, f64> = HashMap::new();
        for name in &user_attr_numeric_names {
            user_attrs_numeric.insert(name.clone(), ua_numeric[name][row]);
        }
        let mut user_attrs_string: HashMap<String, String> = HashMap::new();
        for name in &user_attr_string_names {
            user_attrs_string.insert(name.clone(), ua_string[name][row].clone());
        }
        trial_rows.push(TrialRow {
            trial_id: row as u32,
            trial_number: row as u32,
            param_display,
            param_category_label,
            objective_values,
            user_attrs_numeric,
            user_attrs_string,
            constraint_values: Vec::new(),
        });
    }

    // DataFrame assumes column names are pre-sorted (same convention as finalize_state).
    let mut sorted_params = param_names.clone();
    sorted_params.sort();
    let mut sorted_uan = user_attr_numeric_names.clone();
    sorted_uan.sort();
    let mut sorted_uas = user_attr_string_names.clone();
    sorted_uas.sort();

    let dataframe = DataFrame::from_trials(
        &trial_rows,
        &sorted_params,
        &objective_names,
        &sorted_uan,
        &sorted_uas,
        0,
    );

    let mut all_user_attr_names = user_attr_names.clone();
    all_user_attr_names.sort();

    let meta = StudyMeta {
        study_id: 0,
        name: study_name.to_string(),
        directions: objective_names
            .iter()
            .map(|_| OptimizationDirection::Minimize)
            .collect(),
        completed_trials: row_count,
        total_trials: row_count,
        param_names: sorted_params,
        objective_names,
        user_attr_names: all_user_attr_names,
        has_constraints: false,
        param_bounds,
    };

    Ok(FlatCsvParseResult {
        meta,
        dataframe,
        images,
    })
}

#[cfg(test)]
mod tests;
