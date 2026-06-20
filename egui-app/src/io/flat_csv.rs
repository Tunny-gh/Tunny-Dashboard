//! フラット CSV（1 行 = 1 トライアル）形式のインポート（egui-app 側ブリッジ）。
//!
//! rust_core の [`tunny_core::flat_csv::parse_flat_csv`] を呼び出して単一 Study を
//! 共有ストアへ登録し、`img` 列から CSV と同じディレクトリのアーティファクトを解決する。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::StudyMeta;

/// CSV インポート成功時の戻り値: `(StudyMeta, artifacts_dir, trial_id→アーティファクト)`。
pub type CsvLoadResult = (StudyMeta, PathBuf, HashMap<u32, Vec<ArtifactEntry>>);

/// CSV を読み込み単一 Study を共有ストアへ登録する。
///
/// 成功時は `(StudyMeta, artifacts_dir, trial_id→アーティファクト)` を返す。
/// `artifacts_dir` は CSV の親ディレクトリ（`img` 列の基準パス）。
pub fn load_csv(path: &Path) -> Result<CsvLoadResult, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let study_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("csv")
        .to_string();

    let result = tunny_core::flat_csv::parse_flat_csv(&data, &study_name)?;

    // 単一 Study として共有ストアへ登録する（study_id = 0）。
    tunny_core::dataframe::store_dataframes(vec![result.dataframe]);

    let base_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let artifact_map = build_artifact_map(&base_dir, &result.images);
    let meta = crate::io::journal::convert_study_meta(result.meta);

    Ok((meta, base_dir, artifact_map))
}

/// `img` 列のファイル名を CSV と同じディレクトリの実体に解決し、trial_id 別にまとめる。
/// 実体が存在しないファイルは除外する。
fn build_artifact_map(
    base_dir: &Path,
    images: &[(u32, String)],
) -> HashMap<u32, Vec<ArtifactEntry>> {
    let mut map: HashMap<u32, Vec<ArtifactEntry>> = HashMap::new();
    for (trial_id, filename) in images {
        let path = base_dir.join(filename);
        if !path.is_file() {
            continue;
        }
        // mimetype は空文字にしておき、拡張子から種別判定させる（ArtifactEntry::file_type）。
        map.entry(*trial_id).or_default().push(ArtifactEntry {
            path,
            filename: filename.clone(),
            mimetype: String::new(),
        });
    }
    map
}

/// パスがフラット CSV 形式（拡張子 `.csv`）かを判定する。
pub fn is_csv_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn is_csv_path_detects_extension() {
        assert!(is_csv_path(Path::new("data.csv")));
        assert!(is_csv_path(Path::new("DATA.CSV")));
        assert!(!is_csv_path(Path::new("study.log")));
        assert!(!is_csv_path(Path::new("noext")));
    }

    #[test]
    fn build_artifact_map_only_includes_existing_files() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        std::fs::write(base.join("a.png"), b"img").unwrap();
        let images = vec![
            (0u32, "a.png".to_string()),
            (1u32, "missing.png".to_string()),
        ];
        let map = build_artifact_map(base, &images);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&0).unwrap()[0].filename, "a.png");
        assert!(!map.contains_key(&1));
    }

    #[test]
    fn load_csv_builds_study_and_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        std::fs::write(base.join("img0.png"), b"i0").unwrap();
        let csv = "in:x,in:y,out:f,img\n\
                   1.0,2,10.5,img0.png\n\
                   2.0,4,20.5,missing.png\n";
        let csv_path = write_temp(base, "data.csv", csv);

        let (meta, _dir, artifacts) = load_csv(&csv_path).unwrap();
        assert_eq!(meta.name, "data");
        assert_eq!(meta.param_names, vec!["x".to_string(), "y".to_string()]);
        assert_eq!(meta.objective_names, vec!["f".to_string()]);
        assert_eq!(meta.completed_trials, 2);
        // img0.png は存在、missing.png は除外。
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts.contains_key(&0));
    }
}
