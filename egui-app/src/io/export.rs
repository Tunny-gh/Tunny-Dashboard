use crate::state::types::StudyView;
use tunny_core::export::{CsvField, CsvWriter};

/// CSVエクスポートの対象
#[derive(Debug, Clone, PartialEq)]
pub enum ExportTarget {
    AllData,
    SelectedOnly,
    ParetoOnly,
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

/// `StudyView` と行インデックスリストから CSV 文字列を生成する。
/// 列順: trial_id, trial_number, params..., objectives..., pareto_rank, cluster_id。
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

    #[test]
    fn save_helper_returns_error_on_write_failure() {
        // Write to a path that cannot be created (nonexistent parent directory)
        let bad_path = std::path::Path::new("/nonexistent_dir_xyz/export.csv");
        let result = write_csv_to_path("header\nrow", bad_path);
        assert!(result.is_err(), "write to bad path should return Err");
    }
}
