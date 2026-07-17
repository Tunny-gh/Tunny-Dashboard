//! Import of the flat CSV (1 row = 1 trial) format (egui-app side bridge).
//!
//! Calls rust_core's [`tunny_core::flat_csv::parse_flat_csv`] to register a single
//! Study into the shared store, and resolves artifacts in the same directory as the
//! CSV from the `img` column.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use crate::io::artifacts::ArtifactEntry;
use crate::state::app_state::StudyMeta;

/// Return value on successful CSV import: `(StudyMeta, artifacts_dir, trial_id -> artifacts)`.
pub type CsvLoadResult = (StudyMeta, PathBuf, HashMap<u32, Vec<ArtifactEntry>>);

/// Reads a CSV and registers a single Study into the shared store.
///
/// On success, returns `(StudyMeta, artifacts_dir, trial_id -> artifacts)`.
/// `artifacts_dir` is the CSV's parent directory (the base path for the `img` column).
pub fn load_csv(path: &Path) -> Result<CsvLoadResult, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let study_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("csv")
        .to_string();

    let result = tunny_core::flat_csv::parse_flat_csv(&data, &study_name)?;

    // Register into the shared store as a single Study (study_id = 0).
    tunny_core::dataframe::store_dataframes(vec![result.dataframe]);

    let base_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let artifact_map = build_artifact_map(&base_dir, &result.images);
    let meta = crate::io::journal::convert_study_meta(result.meta);

    Ok((meta, base_dir, artifact_map))
}

/// Resolves `img` column file names against actual files in the same directory as the
/// CSV, grouped by trial_id. Files that don't exist are excluded.
fn build_artifact_map(
    base_dir: &Path,
    images: &[(u32, String)],
) -> HashMap<u32, Vec<ArtifactEntry>> {
    let mut map: HashMap<u32, Vec<ArtifactEntry>> = HashMap::new();
    for (trial_id, filename) in images {
        // Prevent references outside the CSV directory. File names containing an
        // absolute path (RootDir / Windows Prefix) or a parent directory reference
        // (`..`) are excluded without resolving.
        if !is_safe_relative_filename(filename) {
            continue;
        }
        let path = base_dir.join(filename);
        if !path.is_file() {
            continue;
        }
        // Leave mimetype as an empty string and let the type be determined from the
        // extension (ArtifactEntry::file_type).
        map.entry(*trial_id).or_default().push(ArtifactEntry {
            path,
            filename: filename.clone(),
            mimetype: String::new(),
        });
    }
    map
}

/// Determines whether the `img` column's file name is a safe relative path that stays
/// within the CSV directory. Returns `false` if it contains an absolute path
/// (`RootDir` / Windows `Prefix`) or a parent directory reference (`ParentDir` = `..`),
/// preventing file references outside the CSV directory (directory traversal).
fn is_safe_relative_filename(filename: &str) -> bool {
    Path::new(filename).components().all(|c| {
        !matches!(
            c,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

/// Determines whether the path is the flat CSV format (extension `.csv`).
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
    fn is_safe_relative_filename_accepts_plain_names() {
        assert!(is_safe_relative_filename("a.png"));
        assert!(is_safe_relative_filename("sub/dir/a.png"));
        assert!(is_safe_relative_filename("./a.png"));
    }

    #[test]
    fn is_safe_relative_filename_rejects_traversal_and_absolute() {
        assert!(!is_safe_relative_filename("../secret.png"));
        assert!(!is_safe_relative_filename("sub/../../secret.png"));
        assert!(!is_safe_relative_filename("/etc/passwd"));
        // Windows-style absolute/prefix paths (only become Prefix/RootDir under cfg(windows)).
        #[cfg(windows)]
        {
            assert!(!is_safe_relative_filename(r"C:\Windows\system32"));
            assert!(!is_safe_relative_filename(r"\\server\share\a.png"));
        }
    }

    #[test]
    fn build_artifact_map_rejects_parent_dir_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        // Even if a real file is placed outside the base directory, a name with `..` is not resolved.
        std::fs::write(base.join("secret.png"), b"x").unwrap();
        let images = vec![(0u32, "../secret.png".to_string())];
        let map = build_artifact_map(base, &images);
        assert!(map.is_empty());
    }

    #[test]
    fn build_artifact_map_rejects_absolute_path() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let outside = base.join("abs.png");
        std::fs::write(&outside, b"x").unwrap();
        // Even when a real, existing absolute path is given, absolute paths are rejected.
        let images = vec![(0u32, outside.to_string_lossy().into_owned())];
        let map = build_artifact_map(base, &images);
        assert!(map.is_empty());
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
        // img0.png exists, missing.png is excluded.
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts.contains_key(&0));
    }
}
