//! 「Report…」ダイアログのバックグラウンド生成処理（Phase R4）。
//!
//! UI スレッドはこのモジュールへ study のスナップショット（`StudyMeta` クローン・
//! `Arc<DataFrame>`・`Arc<StudyExtras>`・マスク済みストレージ表示名・生成日時）を
//! 渡すだけで、実際の `tunny_core::report::build_study_report` 呼び出しとファイル書き込みは
//! `crate::app::spawn_task` が起動するワーカースレッドで行う（UI をブロックしない）。
//! 成功時は `AppMessage::ReportExportDone`、失敗時は既存の `AppMessage::Error` を送る。

use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;

use tunny_core::dataframe::DataFrame;
use tunny_core::extras::StudyExtras;
use tunny_core::report::ReportLang;

use crate::state::app_state::{Direction, StudyMeta};
use crate::state::messages::AppMessage;
use crate::ui::widgets::report_modal::{export_paths, ReportFormat};

/// journal_path から表示用のストレージ名を作る。RDB URL は必ずマスク済み文字列にする
/// （レポートに生パスワードを残さない）。
pub fn storage_display(journal_path: Option<&Path>) -> String {
    match journal_path {
        Some(path) => match crate::io::rdb::path_as_rdb_url(path) {
            Some(url) => url.masked(),
            None => path.display().to_string(),
        },
        None => "(no storage)".to_string(),
    }
}

/// UI スレッドで集めたスナップショットからレポート生成をバックグラウンドスレッドへ委譲する。
#[allow(clippy::too_many_arguments)]
pub fn spawn_report_export(
    meta: StudyMeta,
    df: Arc<DataFrame>,
    extras: Option<Arc<StudyExtras>>,
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
        match build_and_write_report(
            &meta,
            &df,
            extras.as_deref(),
            &storage_display,
            generated_at_unix,
            lang,
            top_n,
            &formats,
            &base_path,
        ) {
            Ok((paths, overwrote)) => AppMessage::ReportExportDone { paths, overwrote },
            Err(e) => AppMessage::Error(e),
        }
    });
}

/// `StudyReport` を構築し、選択フォーマットぶんのファイルへ書き出す。
#[allow(clippy::too_many_arguments)]
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
    let source = tunny_core::report::ReportSource {
        storage_display: storage_display.to_string(),
        generated_at_unix,
    };
    let opts = tunny_core::report::ReportOptions {
        lang,
        top_n,
        ..Default::default()
    };
    let report = tunny_core::report::build_study_report(&core_meta, df, extras, &source, &opts);

    let mut written = Vec::with_capacity(formats.len());
    // ユーザーが保存ダイアログで選んだ base_path 自体は OS 側の上書き確認を経由するが、
    // `with_extension` で導出する非プライマリの兄弟ファイル（例: base.md/base.json）は
    // ダイアログを通らないため、書き込み前に存在チェックしてサイレント上書きを追跡する。
    let mut overwritten: Vec<PathBuf> = Vec::new();
    for (format, path) in export_paths(base_path, formats) {
        let is_primary = path == base_path;
        if !is_primary && path.exists() {
            overwritten.push(path.clone());
        }
        let content = match format {
            ReportFormat::Html => tunny_core::report::render_html(&report, lang),
            ReportFormat::Markdown => tunny_core::report::render_markdown(&report, lang),
            ReportFormat::Json => serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to serialize report as JSON: {e}"))?,
        };
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
        written.push(path);
    }

    Ok((written, overwritten))
}

/// egui-app の `StudyMeta`（+ DataFrame / StudyExtras 由来の補助情報）を、
/// `tunny_core::report::build_study_report` が要求する core 側 `StudyMeta` へ変換する。
///
/// - `user_attr_names`: egui-app 側の `StudyMeta` は持たないため、DataFrame の
///   数値/文字列 user_attr 列名を統合して補う。
/// - `has_constraints`: DataFrame に制約列が 1 つでもあれば true とする。
/// - `total_trials`: `StudyExtras`（全 state の付帯情報）があればその件数、
///   無ければ DataFrame の行数（= COMPLETE trial 数）で代用する。
fn to_core_meta(
    meta: &StudyMeta,
    df: &DataFrame,
    extras: Option<&StudyExtras>,
) -> tunny_core::io::journal::parser::StudyMeta {
    use tunny_core::io::journal::parser::{OptimizationDirection, StudyMeta as CoreStudyMeta};

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
        use tunny_core::dataframe::TrialRow as CoreRow;

        let rows = vec![CoreRow {
            trial_id: 0,
            trial_number: 0,
            param_display: HashMap::from([("x".to_string(), 1.0)]),
            param_category_label: HashMap::new(),
            objective_values: vec![1.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }];
        let df = DataFrame::from_trials(&rows, &["x".to_string()], &["y".to_string()], &[], &[], 0);
        let meta = make_meta();
        let core_meta = to_core_meta(&meta, &df, None);
        assert_eq!(core_meta.total_trials, 1);
        assert!(!core_meta.has_constraints);
        assert!(core_meta.user_attr_names.is_empty());
    }

    // ── R4-fix: サイレント上書きの可視化 ──────────────────────

    fn make_df() -> DataFrame {
        use std::collections::HashMap;
        use tunny_core::dataframe::TrialRow as CoreRow;

        let rows = vec![CoreRow {
            trial_id: 0,
            trial_number: 0,
            param_display: HashMap::from([("x".to_string(), 1.0)]),
            param_category_label: HashMap::new(),
            objective_values: vec![1.0],
            user_attrs_numeric: HashMap::new(),
            user_attrs_string: HashMap::new(),
            constraint_values: vec![],
        }];
        DataFrame::from_trials(&rows, &["x".to_string()], &["y".to_string()], &[], &[], 0)
    }

    #[test]
    fn build_and_write_report_flags_silently_overwritten_siblings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base_path = dir.path().join("report_x.html");
        // 兄弟ファイル（base_path 由来だが選択フォーマットが異なる非プライマリ）を
        // あらかじめ用意しておき、サイレント上書き検出をテストする。
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

        // 実ファイルは 2 件（html, json）。上書きは json のみ検出される。
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
        // base_path（プライマリ）は保存ダイアログ側で上書き確認済みという前提のため、
        // 事前に存在していても overwrite ノートには含めない。
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
