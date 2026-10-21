use super::formatting::format_f64;

pub fn compute_report_stats(df: &crate::dataframe::DataFrame) -> String {
    compute_report_stats_from_df(df)
}

pub(crate) fn compute_report_stats_from_df(df: &crate::dataframe::DataFrame) -> String {
    let entries: Vec<_> = tdashboard_analysis::export::column_stats(df)
        .into_iter()
        .map(|s| {
            format!(
                r#""{}":{{"min":{},"max":{},"mean":{},"std":{},"count":{}}}"#,
                s.name.replace('"', "\\\""),
                format_f64(s.min),
                format_f64(s.max),
                format_f64(s.mean),
                format_f64(s.std),
                s.count
            )
        })
        .collect();
    format!("{{{}}}", entries.join(","))
}
