use std::path::PathBuf;
use std::sync::mpsc::SyncSender;

use crate::state::messages::AppMessage;
use crate::state::types::StudyView;
use tunny_core::export::{CsvField, CsvWriter};

/// Target rows for CSV export.
#[derive(Debug, Clone, PartialEq)]
pub enum ExportTarget {
    AllData,
    SelectedOnly,
    ParetoOnly,
}

/// Flags for optional columns (rank / cluster) appended to the end of a trial-row CSV.
/// Each caller sets only the columns it needs to true (a pareto export needs only the
/// rank, a full export needs rank + cluster).
#[derive(Debug, Clone, Copy, Default)]
pub struct TrialCsvColumns {
    /// Include the `pareto_rank` column (each row's Pareto rank; 0 = front).
    pub pareto_rank: bool,
    /// Include the `cluster_id` column (cluster assignment; blank if unassigned).
    pub cluster_id: bool,
}

/// Generates a trial-row CSV from a `StudyView` and a row index list.
/// Column order: trial_id, trial_number, params..., objectives..., [pareto_rank], [cluster_id].
/// Whether the trailing rank/cluster columns are present is controlled by `columns`.
pub fn build_trial_csv_from_view(
    view: &StudyView,
    row_indices: &[usize],
    param_names: &[String],
    objective_names: &[String],
    columns: TrialCsvColumns,
) -> String {
    let param_cols = view.numeric_columns(param_names);
    let obj_cols = view.numeric_columns(objective_names);

    let mut w = CsvWriter::new();
    let mut header: Vec<&str> = vec!["trial_id", "trial_number"];
    header.extend(param_names.iter().map(String::as_str));
    header.extend(objective_names.iter().map(String::as_str));
    if columns.pareto_rank {
        header.push("pareto_rank");
    }
    if columns.cluster_id {
        header.push("cluster_id");
    }
    w.header(header);

    for &i in row_indices {
        let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
        let trial_number = view.df.get_trial_number(i).unwrap_or(i as u32);
        let mut fields = vec![
            CsvField::UInt(trial_id as u64),
            CsvField::UInt(trial_number as u64),
        ];
        for col in param_cols.iter().chain(&obj_cols) {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            fields.push(CsvField::Num(v));
        }
        if columns.pareto_rank {
            let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
            fields.push(CsvField::UInt(rank as u64));
        }
        if columns.cluster_id {
            let cluster = view.cluster_id.get(i).copied().flatten();
            fields.push(
                cluster
                    .map(|c| CsvField::Int(c as i64))
                    .unwrap_or(CsvField::Empty),
            );
        }
        w.row(fields);
    }

    w.finish()
}

/// Generates a CSV string from a `StudyView` and a row index list (with rank + cluster columns).
/// Column order: trial_id, trial_number, params..., objectives..., pareto_rank, cluster_id.
pub fn build_csv_string_from_view(
    view: &StudyView,
    row_indices: &[usize],
    param_names: &[String],
    objective_names: &[String],
) -> String {
    build_trial_csv_from_view(
        view,
        row_indices,
        param_names,
        objective_names,
        TrialCsvColumns {
            pareto_rank: true,
            cluster_id: true,
        },
    )
}

/// Returns the row indices to export, based on the `StudyView`.
pub fn select_row_indices_for_export(
    view: &StudyView,
    selected_indices: &[u32],
    pareto_indices: &[u32],
    target: &ExportTarget,
) -> Vec<usize> {
    let n = view.row_count();
    match target {
        ExportTarget::AllData => (0..n).collect(),
        ExportTarget::SelectedOnly => {
            let id_set: std::collections::HashSet<u32> = selected_indices.iter().copied().collect();
            (0..n)
                .filter(|&i| view.trial_ids.get(i).is_some_and(|id| id_set.contains(id)))
                .collect()
        }
        ExportTarget::ParetoOnly => {
            let pareto_set: std::collections::HashSet<u32> =
                pareto_indices.iter().copied().collect();
            (0..n)
                .filter(|&i| {
                    view.trial_ids
                        .get(i)
                        .is_some_and(|id| pareto_set.contains(id))
                })
                .collect()
        }
    }
}

/// Writes a CSV string to the given path. Returns an error string on failure.
/// Writes atomically so a crash mid-overwrite doesn't corrupt the existing file.
pub fn write_csv_to_path(csv: &str, path: &std::path::Path) -> Result<(), String> {
    crate::io::file::write_atomic(path, csv.as_bytes()).map_err(|e| e.to_string())
}

/// Opens the CSV save dialog and only determines the save path (does not write).
/// Since the native save dialog blocks the UI thread, the caller runs this first on
/// the UI thread to obtain the path, then delegates CSV construction and writing to
/// the background (the same convention as `spawn_report_export`). Returns `None` on cancel.
pub fn pick_csv_save_path(default_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name(default_name)
        .save_file()
}

/// Writes an already-built CSV string to the confirmed path on a background thread.
/// The save dialog has already been run by the caller on the UI thread; here only the
/// file I/O is delegated to `spawn_task` (so the UI doesn't freeze even for a huge
/// Study). Sends `CsvExportDone` on success, `CsvExportFailed` on failure.
pub fn spawn_csv_write(csv: String, path: PathBuf, tx: SyncSender<AppMessage>) {
    crate::app::spawn_task(tx, move || match write_csv_to_path(&csv, &path) {
        Ok(()) => AppMessage::CsvExportDone,
        Err(e) => AppMessage::CsvExportFailed(e),
    });
}

/// Builds a trial-row CSV from a `StudyView` snapshot and writes it to the confirmed
/// path, running both steps together on a background thread. Since both building the
/// CSV string (heavy for a huge Study, as it scans all trials) and writing happen on
/// `spawn_task`, the UI thread only handles the save dialog and resolving the row
/// selection. Only owned clones are passed to the worker; no borrows are carried across.
pub fn spawn_view_csv_export(
    view: StudyView,
    row_indices: Vec<usize>,
    param_names: Vec<String>,
    objective_names: Vec<String>,
    path: PathBuf,
    tx: SyncSender<AppMessage>,
) {
    crate::app::spawn_task(tx, move || {
        let csv = build_csv_string_from_view(&view, &row_indices, &param_names, &objective_names);
        match write_csv_to_path(&csv, &path) {
            Ok(()) => AppMessage::CsvExportDone,
            Err(e) => AppMessage::CsvExportFailed(e),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_helper_returns_error_on_write_failure() {
        // Write to a path that cannot be created (nonexistent parent directory)
        let bad_path = std::path::Path::new("/nonexistent_dir_xyz/export.csv");
        let result = write_csv_to_path("header\nrow", bad_path);
        assert!(result.is_err(), "write to bad path should return Err");
    }
}
