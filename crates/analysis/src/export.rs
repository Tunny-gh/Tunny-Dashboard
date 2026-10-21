//! Derived export values. Encoding, formula guards and external writes belong to I/O.
use crate::dataframe::DataFrame;

pub struct ColumnStats {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std: f64,
    pub count: usize,
}

pub fn column_stats(df: &DataFrame) -> Vec<ColumnStats> {
    df.column_names()
        .into_iter()
        .filter_map(|name| {
            let values: Vec<_> = df
                .get_numeric_column(&name)?
                .iter()
                .copied()
                .filter(|v| v.is_finite())
                .collect();
            let count = values.len();
            if count == 0 {
                return None;
            }
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let mean = values.iter().sum::<f64>() / count as f64;
            let std = if count > 1 {
                (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64).sqrt()
            } else {
                0.0
            };
            Some(ColumnStats {
                name,
                min,
                max,
                mean,
                std,
                count,
            })
        })
        .collect()
}

pub enum ExportCell {
    Numeric(f64),
    Text(String),
    Empty,
}

pub fn select_rows(df: &DataFrame, indices: &[u32], columns: &[String]) -> Vec<Vec<ExportCell>> {
    indices
        .iter()
        .map(|&i| i as usize)
        .filter(|&i| i < df.row_count())
        .map(|row| {
            columns
                .iter()
                .map(|name| {
                    if name == "trial_id" {
                        return df
                            .get_trial_id(row)
                            .map_or(ExportCell::Empty, |id| ExportCell::Numeric(f64::from(id)));
                    }
                    if let Some(values) = df.get_numeric_column(name) {
                        return values
                            .get(row)
                            .copied()
                            .map_or(ExportCell::Empty, ExportCell::Numeric);
                    }
                    df.get_string_column(name)
                        .and_then(|values| values.get(row))
                        .cloned()
                        .map_or(ExportCell::Empty, ExportCell::Text)
                })
                .collect()
        })
        .collect()
}
