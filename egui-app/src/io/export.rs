use crate::state::types::StudyView;
use tunny_core::export::{CsvField, CsvWriter};

/// CSVエクスポートの対象
#[derive(Debug, Clone, PartialEq)]
pub enum ExportTarget {
    AllData,
    SelectedOnly,
    ParetoOnly,
}

/// trial 行 CSV の末尾に付けるオプション列（ランク / クラスタ）のフラグ。
/// 呼び出し元ごとに必要な列だけを true にする（pareto エクスポートはランクのみ、
/// 全件エクスポートはランク＋クラスタ）。
#[derive(Debug, Clone, Copy, Default)]
pub struct TrialCsvColumns {
    /// `pareto_rank` 列（各行の Pareto ランク。0 = フロント）を含める。
    pub pareto_rank: bool,
    /// `cluster_id` 列（クラスタ割当。未割当は空欄）を含める。
    pub cluster_id: bool,
}

/// `StudyView` と行インデックスリストから trial 行 CSV を生成する。
/// 列順: trial_id, trial_number, params..., objectives..., [pareto_rank], [cluster_id]。
/// 末尾のランク列・クラスタ列の有無は `columns` で切り替える。
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

/// `StudyView` と行インデックスリストから CSV 文字列を生成する（ランク＋クラスタ列付き）。
/// 列順: trial_id, trial_number, params..., objectives..., pareto_rank, cluster_id。
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
/// 上書き途中のクラッシュで既存ファイルを壊さないようアトミックに書き込む。
pub fn write_csv_to_path(csv: &str, path: &std::path::Path) -> Result<(), String> {
    crate::io::file::write_atomic(path, csv.as_bytes()).map_err(|e| e.to_string())
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
