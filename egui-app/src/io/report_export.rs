//! Background generation processing for the "Report..." dialog (Phase R4).
//!
//! The UI passes one acquired Study input and its consistently prepared table,
//! plus a masked storage display name. Pure analysis and external file writes
//! are composed on a worker thread launched by
//! `crate::app::spawn_task` (so the UI is never blocked). Sends
//! `AppMessage::ReportExportDone` on success, or the existing `AppMessage::Error` on
//! failure.

use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;

#[cfg(test)]
use tdashboard_analysis::dataframe::DataFrame;
#[cfg(test)]
use tdashboard_analysis::extras::StudyExtras;
use tdashboard_analysis::report::ReportLang;

#[cfg(test)]
use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;
use crate::ui::widgets::report_modal::{export_paths, ReportFormat};

/// Builds a display-ready storage name from journal_path. RDB URLs are always turned
/// into a masked string (never leave a raw password in the report).
pub fn storage_display(journal_path: Option<&Path>) -> String {
    match journal_path {
        Some(path) => match crate::io::rdb::path_as_rdb_url(path) {
            Some(url) => url.masked(),
            None => path.display().to_string(),
        },
        None => "(no storage)".to_string(),
    }
}

/// Delegates report generation to a background thread, from a snapshot collected on
/// the UI thread.
#[allow(clippy::too_many_arguments)]
pub fn spawn_report_export(
    input: tdashboard_study::StudyInput,
    table: std::sync::Arc<tdashboard_analysis::projection::Projection>,
    storage_display: String,
    lang: ReportLang,
    top_n: usize,
    formats: Vec<ReportFormat>,
    base_path: PathBuf,
    tx: SyncSender<AppMessage>,
) {
    let generated_at_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64);

    crate::app::spawn_task(tx, move || {
        let source = tdashboard_analysis::report::ReportSource {
            storage_display,
            generated_at_unix,
        };
        let opts = tdashboard_analysis::report::ReportOptions {
            lang,
            top_n,
            ..Default::default()
        };
        let result =
            tdashboard_analysis::report::generate_prepared_report(input, table, &source, &opts)
                .and_then(|computed| write_report(&computed.report, lang, &formats, &base_path));
        match result {
            Ok((paths, overwrote)) => AppMessage::ReportExportDone { paths, overwrote },
            Err(e) => AppMessage::Error(e),
        }
    });
}

/// Builds a `StudyReport` and writes it out to a file for each selected format.
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
fn build_and_write_report(
    meta: &StudyMeta,
    df: &DataFrame,
    extras: Option<&StudyExtras>,
    storage_display: &str,
    generated_at_unix: Option<i64>,
    lang: ReportLang,
    top_n: usize,
    formats: &[ReportFormat],
    base_path: &Path,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let core_meta = to_core_meta(meta, df, extras);
    let source = tdashboard_analysis::report::ReportSource {
        storage_display: storage_display.to_string(),
        generated_at_unix,
    };
    let opts = tdashboard_analysis::report::ReportOptions {
        lang,
        top_n,
        ..Default::default()
    };
    let report =
        tdashboard_analysis::report::build_study_report(&core_meta, df, extras, &source, &opts);

    write_report(&report, lang, formats, base_path)
}

fn write_report(
    report: &tdashboard_analysis::report::StudyReport,
    lang: ReportLang,
    formats: &[ReportFormat],
    base_path: &Path,
) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let mut written = Vec::with_capacity(formats.len());
    // The base_path itself, chosen by the user in the save dialog, goes through the
    // OS's overwrite confirmation, but non-primary sibling files derived via
    // `with_extension` (e.g. base.md/base.json) don't go through the dialog, so we
    // check existence before writing to track silent overwrites.
    let mut overwritten: Vec<PathBuf> = Vec::new();
    for (format, path) in export_paths(base_path, formats) {
        let is_primary = path == base_path;
        if !is_primary && path.exists() {
            overwritten.push(path.clone());
        }
        let content = match format {
            ReportFormat::Html => tdashboard_analysis::report::render_html(report, lang),
            ReportFormat::Markdown => tdashboard_analysis::report::render_markdown(report, lang),
            ReportFormat::Json => serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to serialize report as JSON: {e}"))?,
        };
        crate::io::file::write_atomic(&path, content.as_bytes())
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        written.push(path);
    }

    Ok((written, overwritten))
}

/// Converts egui-app's `StudyMeta` (plus auxiliary info derived from DataFrame /
/// StudyExtras) into the core-side `StudyMeta` required by
/// `tdashboard_analysis::report::build_study_report`.
///
/// - `user_attr_names`: egui-app's `StudyMeta` doesn't have this, so it's filled in by
///   merging the DataFrame's numeric/string user_attr column names.
/// - `has_constraints`: true if the DataFrame has even one constraint column.
/// - `total_trials`: uses the count from `StudyExtras` (extra info for all states) if
///   available, otherwise falls back to the DataFrame's row count (= number of
///   COMPLETE trials).
#[cfg(test)]
fn to_core_meta(
    meta: &StudyMeta,
    df: &DataFrame,
    extras: Option<&StudyExtras>,
) -> tdashboard_analysis::projection::StudyMeta {
    use tdashboard_analysis::projection::{OptimizationDirection, StudyMeta as CoreStudyMeta};

    let directions = meta
        .directions
        .iter()
        .map(|d| match d {
            Direction::Minimize => OptimizationDirection::Minimize,
            Direction::Maximize => OptimizationDirection::Maximize,
        })
        .collect();

    let mut user_attr_names: Vec<String> = df
        .user_attr_numeric_col_names()
        .iter()
        .chain(df.user_attr_string_col_names())
        .cloned()
        .collect();
    user_attr_names.sort();
    user_attr_names.dedup();

    let total_trials = extras
        .map(|e| e.trials.len() as u32)
        .unwrap_or(df.row_count() as u32);

    CoreStudyMeta {
        study_id: meta.study_id,
        name: meta.name.clone(),
        directions,
        completed_trials: meta.completed_trials as u32,
        total_trials,
        param_names: meta.param_names.clone(),
        objective_names: meta.objective_names.clone(),
        user_attr_names,
        has_constraints: !df.constraint_col_names().is_empty(),
        param_bounds: meta.param_bounds.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn d4_report_worker_uses_one_acquired_input_and_the_finite_front_policy() {
        use tdashboard_study::*;
        let key = StudyKey {
            source: IdentityAllocator::new().unwrap().source().unwrap(),
            stored_id: 0,
        };
        let bytes = b"{\"op_code\":0,\"study_name\":\"report\",\"directions\":[1,1]}\n{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[1,2]}\n{\"op_code\":4,\"study_id\":0,\"distributions\":{},\"state\":1,\"values\":[-Infinity,0]}\n{\"op_code\":4,\"study_id\":0}\n";
        let batch = tdashboard_io::journal::parser::study::read_full(bytes, key).unwrap();
        let mut manager = StudyManager::new(key);
        let ticket = manager.begin(LoadKind::Initial).unwrap();
        let receipt = manager
            .apply(manager.header(ticket).unwrap(), batch)
            .unwrap();
        manager.finish(ticket, receipt.next_sequence).unwrap();
        let input = manager.acquire().unwrap();
        let mut session = tdashboard_analysis::session::AnalysisSession::default();
        let table = session.project(input.clone());
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("report.json");
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        spawn_report_export(
            input.clone(),
            table,
            "test journal".into(),
            ReportLang::En,
            10,
            vec![
                ReportFormat::Html,
                ReportFormat::Markdown,
                ReportFormat::Json,
            ],
            path.clone(),
            tx,
        );
        let ticket = manager.begin(LoadKind::Refresh).unwrap();
        manager
            .apply(
                manager.header(ticket).unwrap(),
                StudyBatch::Replace {
                    trials: vec![],
                    metadata: (**input.data().metadata()).clone(),
                },
            )
            .unwrap();
        assert!(matches!(
            rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap(),
            AppMessage::ReportExportDone { .. }
        ));
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(report["overview"]["complete_trials"], 2);
        assert_eq!(report["overview"]["total_trials"], 3);
        assert_eq!(report["outcome"]["MultiObj"]["pareto_size"], 1);
        assert!(std::fs::read_to_string(path.with_extension("html"))
            .unwrap()
            .contains("<!DOCTYPE html>"));
        assert!(std::fs::read_to_string(path.with_extension("md"))
            .unwrap()
            .contains("## "));
    }

    #[test]
    fn storage_display_masks_rdb_password() {
        let path = PathBuf::from("postgresql://user:secret@localhost:5432/optuna");
        let display = storage_display(Some(&path));
        assert!(
            !display.contains("secret"),
            "password must be masked: {display}"
        );
        assert!(display.contains("user"));
    }

    #[test]
    fn storage_display_uses_plain_path_for_local_files() {
        let path = PathBuf::from("/data/study.log");
        assert_eq!(storage_display(Some(&path)), "/data/study.log");
    }

    #[test]
    fn storage_display_none_when_no_storage() {
        assert_eq!(storage_display(None), "(no storage)");
    }

    fn make_meta() -> StudyMeta {
        StudyMeta {
            study_id: 1,
            name: "study-a".to_string(),
            directions: vec![Direction::Minimize],
            completed_trials: 3,
            param_names: vec!["x".to_string()],
            objective_names: vec!["y".to_string()],
            param_bounds: Default::default(),
        }
    }

    #[test]
    fn to_core_meta_falls_back_to_row_count_without_extras() {
        use std::collections::HashMap;
        use tdashboard_analysis::dataframe::TrialRow as CoreRow;

        let rows = vec![CoreRow {
            trial_id: 0,
            trial_number: 0,
            param_display: HashMap::from([("x".to_string(), 1.0)]),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![1.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        }];
        let df = DataFrame::from_trials(&rows, &["x".to_string()], &["y".to_string()], &[], &[], 0);
        let meta = make_meta();
        let core_meta = to_core_meta(&meta, &df, None);
        assert_eq!(core_meta.total_trials, 1);
        assert!(!core_meta.has_constraints);
        assert!(core_meta.user_attr_names.is_empty());
    }

    // ── R4-fix: surfacing silent overwrites ──────────────────────

    fn make_df() -> DataFrame {
        use std::collections::HashMap;
        use tdashboard_analysis::dataframe::TrialRow as CoreRow;

        let rows = vec![CoreRow {
            trial_id: 0,
            trial_number: 0,
            param_display: HashMap::from([("x".to_string(), 1.0)]),
            distribution_metadata: Default::default(),
            param_category_label: HashMap::new(),
            objective_values: vec![1.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            user_attrs_json: HashMap::new(),
            constraint_values: vec![],
        }];
        DataFrame::from_trials(&rows, &["x".to_string()], &["y".to_string()], &[], &[], 0)
    }

    #[test]
    fn build_and_write_report_flags_silently_overwritten_siblings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base_path = dir.path().join("report_x.html");
        // Pre-seed a sibling file (derived from base_path but a non-primary one with a
        // different selected format) to test silent-overwrite detection.
        std::fs::write(dir.path().join("report_x.json"), "stale").expect("seed sibling");

        let meta = make_meta();
        let df = make_df();
        let formats = [ReportFormat::Html, ReportFormat::Json];

        let (written, overwrote) = build_and_write_report(
            &meta,
            &df,
            None,
            "(no storage)",
            None,
            ReportLang::En,
            10,
            &formats,
            &base_path,
        )
        .expect("report export succeeds");

        // There are 2 actual files (html, json). Only json is detected as overwritten.
        assert_eq!(written.len(), 2);
        assert!(written.contains(&base_path));
        assert!(written.contains(&dir.path().join("report_x.json")));
        assert_eq!(overwrote, vec![dir.path().join("report_x.json")]);
    }

    #[test]
    fn build_and_write_report_no_note_when_nothing_overwritten() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base_path = dir.path().join("report_y.html");

        let meta = make_meta();
        let df = make_df();
        let formats = [ReportFormat::Html, ReportFormat::Json];

        let (written, overwrote) = build_and_write_report(
            &meta,
            &df,
            None,
            "(no storage)",
            None,
            ReportLang::En,
            10,
            &formats,
            &base_path,
        )
        .expect("report export succeeds");

        assert_eq!(written.len(), 2);
        assert!(written.iter().all(|p| p.exists()));
        assert!(overwrote.is_empty());
    }

    #[test]
    fn build_and_write_report_ignores_preexisting_primary_path() {
        // base_path (the primary) is assumed to already have gone through overwrite
        // confirmation on the save-dialog side, so even if it pre-exists it isn't
        // included in the overwrite note.
        let dir = tempfile::tempdir().expect("tempdir");
        let base_path = dir.path().join("report_z.html");
        std::fs::write(&base_path, "stale primary").expect("seed primary");

        let meta = make_meta();
        let df = make_df();
        let formats = [ReportFormat::Html];

        let (written, overwrote) = build_and_write_report(
            &meta,
            &df,
            None,
            "(no storage)",
            None,
            ReportLang::En,
            10,
            &formats,
            &base_path,
        )
        .expect("report export succeeds");

        assert_eq!(written, vec![base_path]);
        assert!(overwrote.is_empty());
    }
}
