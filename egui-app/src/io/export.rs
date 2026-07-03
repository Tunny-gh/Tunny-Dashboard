use crate::state::app_state::TrialRow;
use crate::state::types::StudyView;
use tunny_core::export::{CsvField, CsvWriter};

/// CSVエクスポートの対象
#[derive(Debug, Clone, PartialEq)]
pub enum ExportTarget {
    AllData,
    SelectedOnly,
    ParetoOnly,
}

/// エクスポート対象の TrialRow をフィルタリングして返す
pub fn select_rows_for_export<'a>(
    trial_rows: &'a [TrialRow],
    selected_indices: &[u32],
    pareto_indices: &[u32],
    target: &ExportTarget,
) -> Vec<&'a TrialRow> {
    match target {
        ExportTarget::AllData => trial_rows.iter().collect(),
        ExportTarget::SelectedOnly => trial_rows
            .iter()
            .filter(|r| selected_indices.contains(&r.trial_id))
            .collect(),
        ExportTarget::ParetoOnly => trial_rows
            .iter()
            .filter(|r| pareto_indices.contains(&r.trial_id))
            .collect(),
    }
}

/// trial エクスポート共通のヘッダ列
/// （trial_id, trial_number, params..., objectives..., pareto_rank, cluster_id）を書き込む。
fn write_trial_header(w: &mut CsvWriter, param_names: &[String], objective_names: &[String]) {
    let mut header: Vec<&str> = vec!["trial_id", "trial_number"];
    header.extend(param_names.iter().map(String::as_str));
    header.extend(objective_names.iter().map(String::as_str));
    header.push("pareto_rank");
    header.push("cluster_id");
    w.header(header);
}

/// `TrialRow` のスライスから CSV 文字列を生成する純粋関数。
/// 列順: trial_id, trial_number, <params...>, <objectives...>, pareto_rank, cluster_id
/// エスケープ・数式ガードは tunny_core の `CsvWriter` に委譲する。
pub fn build_csv_string(
    rows: &[&TrialRow],
    param_names: &[String],
    objective_names: &[String],
) -> String {
    let mut w = CsvWriter::new();
    write_trial_header(&mut w, param_names, objective_names);

    for row in rows {
        let mut fields = vec![
            CsvField::UInt(row.trial_id as u64),
            CsvField::UInt(row.trial_number as u64),
        ];
        for name in param_names {
            fields.push(
                row.params
                    .get(name)
                    .map(|v| CsvField::Num(*v))
                    .unwrap_or(CsvField::Empty),
            );
        }
        for i in 0..objective_names.len() {
            fields.push(
                row.objectives
                    .get(i)
                    .map(|v| CsvField::Num(*v))
                    .unwrap_or(CsvField::Empty),
            );
        }
        fields.push(CsvField::UInt(row.pareto_rank as u64));
        fields.push(
            row.cluster_id
                .map(|c| CsvField::Int(c as i64))
                .unwrap_or(CsvField::Empty),
        );
        w.row(fields);
    }

    w.finish()
}

/// `StudyView` と行インデックスリストから CSV 文字列を生成する。
/// `build_csv_string` と同一の列順（trial_id, trial_number, params..., objectives..., pareto_rank, cluster_id）。
pub fn build_csv_string_from_view(
    view: &StudyView,
    row_indices: &[usize],
    param_names: &[String],
    objective_names: &[String],
) -> String {
    let param_cols = view.numeric_columns(param_names);
    let obj_cols = view.numeric_columns(objective_names);

    let mut w = CsvWriter::new();
    write_trial_header(&mut w, param_names, objective_names);

    for &i in row_indices {
        let trial_id = view.trial_ids.get(i).copied().unwrap_or(i as u32);
        let trial_number = view.df.get_trial_number(i).unwrap_or(i as u32);
        let rank = view.pareto_rank.get(i).copied().unwrap_or(0);
        let cluster = view.cluster_id.get(i).copied().flatten();
        let mut fields = vec![
            CsvField::UInt(trial_id as u64),
            CsvField::UInt(trial_number as u64),
        ];
        for col in param_cols.iter().chain(&obj_cols) {
            let v = col.and_then(|c| c.get(i)).copied().unwrap_or(f64::NAN);
            fields.push(CsvField::Num(v));
        }
        fields.push(CsvField::UInt(rank as u64));
        fields.push(
            cluster
                .map(|c| CsvField::Int(c as i64))
                .unwrap_or(CsvField::Empty),
        );
        w.row(fields);
    }

    w.finish()
}

/// `StudyView` ベースのエクスポート対象行インデックスを返す。
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

/// CSV 文字列を指定パスへ書き込む。失敗時はエラー文字列を返す。
pub fn write_csv_to_path(csv: &str, path: &std::path::Path) -> Result<(), String> {
    std::fs::write(path, csv).map_err(|e| e.to_string())
}

/// ファイル保存ダイアログを開いて CSV を保存する。
/// ダイアログがキャンセルされた場合は `Ok(())` を返す。
pub fn save_csv_to_file(csv: &str) -> Result<(), String> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name("export.csv")
        .save_file()
    {
        write_csv_to_path(csv, &path)
    } else {
        Ok(())
    }
}

/// デフォルトファイル名を指定してCSVをファイルダイアログ経由で保存する。
/// ダイアログがキャンセルされた場合は `Ok(())` を返す。
pub fn save_csv_to_file_named(csv: &str, default_name: &str) -> Result<(), String> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("CSV", &["csv"])
        .set_file_name(default_name)
        .save_file()
    {
        write_csv_to_path(csv, &path)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::TrialState;
    use std::collections::HashMap;

    fn make_trial(id: u32) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            params: HashMap::new(),
            objectives: vec![],
            pareto_rank: 0,
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }
    }

    fn make_trial_with_data(
        id: u32,
        params: HashMap<String, f64>,
        objectives: Vec<f64>,
    ) -> TrialRow {
        TrialRow {
            trial_id: id,
            trial_number: id,
            params,
            objectives,
            pareto_rank: if id == 0 { 1 } else { 2 },
            cluster_id: None,
            state: TrialState::Complete,
            user_attrs: HashMap::new(),
        }
    }

    #[test]
    fn all_data_returns_all_rows() {
        let rows = vec![make_trial(0), make_trial(1), make_trial(2)];
        let result = select_rows_for_export(&rows, &[], &[], &ExportTarget::AllData);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn selected_only_filters_by_indices() {
        let rows = vec![make_trial(0), make_trial(1), make_trial(2)];
        let result = select_rows_for_export(&rows, &[0, 2], &[], &ExportTarget::SelectedOnly);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].trial_id, 0);
        assert_eq!(result[1].trial_id, 2);
    }

    #[test]
    fn pareto_only_filters_by_pareto_indices() {
        let rows = vec![make_trial(0), make_trial(1), make_trial(2)];
        let result = select_rows_for_export(&rows, &[], &[1], &ExportTarget::ParetoOnly);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].trial_id, 1);
    }

    #[test]
    fn selected_only_empty_selection_returns_none() {
        let rows = vec![make_trial(0), make_trial(1)];
        let result = select_rows_for_export(&rows, &[], &[], &ExportTarget::SelectedOnly);
        assert_eq!(result.len(), 0);
    }

    // ── TASK-2229: 新規テスト ────────────────────────────────────

    #[test]
    fn build_csv_string_includes_required_headers() {
        let rows = [make_trial(0)];
        let param_names = vec!["x".to_string(), "y".to_string()];
        let obj_names = vec!["f1".to_string()];
        let csv = build_csv_string(&rows.iter().collect::<Vec<_>>(), &param_names, &obj_names);
        let header = csv.lines().next().unwrap_or("");
        assert!(header.contains("trial_id"), "missing trial_id");
        assert!(header.contains("trial_number"), "missing trial_number");
        assert!(header.contains("x"), "missing param x");
        assert!(header.contains("y"), "missing param y");
        assert!(header.contains("f1"), "missing objective f1");
        assert!(header.contains("pareto_rank"), "missing pareto_rank");
        assert!(header.contains("cluster_id"), "missing cluster_id");
    }

    #[test]
    fn build_csv_string_respects_export_target_rows() {
        let mut p = HashMap::new();
        p.insert("x".to_string(), 1.0_f64);
        let rows = vec![
            make_trial_with_data(0, p.clone(), vec![0.5]),
            make_trial_with_data(1, p.clone(), vec![0.6]),
            make_trial_with_data(2, p.clone(), vec![0.7]),
        ];
        let param_names = vec!["x".to_string()];
        let obj_names = vec!["f1".to_string()];

        // AllData: 3 data rows + 1 header = 4 lines
        let all = select_rows_for_export(&rows, &[0, 1], &[0], &ExportTarget::AllData);
        let csv = build_csv_string(&all, &param_names, &obj_names);
        assert_eq!(csv.lines().count(), 4, "AllData should produce 3 data rows");

        // SelectedOnly: 2 rows
        let sel = select_rows_for_export(&rows, &[0, 1], &[0], &ExportTarget::SelectedOnly);
        let csv = build_csv_string(&sel, &param_names, &obj_names);
        assert_eq!(
            csv.lines().count(),
            3,
            "SelectedOnly should produce 2 data rows"
        );

        // ParetoOnly: 1 row
        let par = select_rows_for_export(&rows, &[0, 1], &[0], &ExportTarget::ParetoOnly);
        let csv = build_csv_string(&par, &param_names, &obj_names);
        assert_eq!(
            csv.lines().count(),
            2,
            "ParetoOnly should produce 1 data row"
        );
    }

    #[test]
    fn save_helper_returns_error_on_write_failure() {
        // Write to a path that cannot be created (nonexistent parent directory)
        let bad_path = std::path::Path::new("/nonexistent_dir_xyz/export.csv");
        let result = write_csv_to_path("header\nrow", bad_path);
        assert!(result.is_err(), "write to bad path should return Err");
    }

    #[test]
    fn export_pipeline_generates_same_row_count_as_selection() {
        let rows: Vec<TrialRow> = (0..5).map(make_trial).collect();
        let selected = vec![0u32, 2, 4];
        let exported = select_rows_for_export(&rows, &selected, &[], &ExportTarget::SelectedOnly);
        let csv = build_csv_string(&exported, &[], &[]);
        // 3 data rows + 1 header line
        let data_lines = csv.lines().count().saturating_sub(1);
        assert_eq!(
            data_lines,
            selected.len(),
            "data line count must match selection count"
        );
    }

    // ── TASK-2246: CSV export regression ────────────────────────

    #[test]
    fn export_and_pinning_logic_have_dedicated_regression_tests() {
        // Regression: SelectedOnly with empty selection returns 0 rows
        let rows: Vec<TrialRow> = (0..5).map(make_trial).collect();
        let empty: &[u32] = &[];
        let result = select_rows_for_export(&rows, empty, empty, &ExportTarget::SelectedOnly);
        assert_eq!(result.len(), 0, "empty selection must produce no rows");

        // Regression: AllData always returns full set regardless of selection
        let result_all = select_rows_for_export(&rows, &[0], &[], &ExportTarget::AllData);
        assert_eq!(result_all.len(), 5, "AllData ignores selected_indices");

        // Regression: CSV header is always present even for 0-row export
        let csv = build_csv_string(&result, &["p".to_string()], &["o".to_string()]);
        assert!(csv.lines().next().is_some(), "CSV must have a header line");
    }
}
