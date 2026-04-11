use super::formatting::format_f64;

/// Documentation.
///
/// Documentation.
/// Documentation.
/// Documentation.
///
/// Documentation.
pub fn compute_report_stats() -> String {
    let result = crate::dataframe::with_active_df(compute_report_stats_from_df);
    result.unwrap_or_else(|| "{}".to_string())
}

/// Documentation.
pub(crate) fn compute_report_stats_from_df(df: &crate::dataframe::DataFrame) -> String {
    if df.row_count() == 0 {
        return "{}".to_string();
    }

    let mut entries: Vec<String> = Vec::new();

    for col_name in df.column_names() {
        if let Some(vals) = df.get_numeric_column(&col_name) {
            let finite: Vec<f64> = vals.iter().copied().filter(|v| v.is_finite()).collect();
            if finite.is_empty() {
                continue;
            }

            let count = finite.len();
            let min = finite.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = finite.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let mean = finite.iter().sum::<f64>() / count as f64;

            let std = if count > 1 {
                let variance =
                    finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64;
                variance.sqrt()
            } else {
                0.0
            };

            let safe_name = col_name.replace('"', "\\\"");
            let entry = format!(
                r#""{}":{{"min":{},"max":{},"mean":{},"std":{},"count":{}}}"#,
                safe_name,
                format_f64(min),
                format_f64(max),
                format_f64(mean),
                format_f64(std),
                count
            );
            entries.push(entry);
        }
    }

    format!("{{{}}}", entries.join(","))
}
