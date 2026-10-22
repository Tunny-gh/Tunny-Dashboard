//! DesignExplorer CSV path checks and session-root composition. Format decoding
//! and safe resolution live in I/O; the entry applies owned batches to managers.

use std::collections::HashMap;
use std::path::Path;

use crate::io::artifacts::ArtifactEntry;
#[cfg(test)]
use crate::state::app_state::StudyMeta;

/// Successful import: metadata and unresolved img paths associated with trial IDs.
#[cfg(test)]
pub type CsvLoadResult = (StudyMeta, Vec<(u32, String)>);

/// Legacy fixture metadata helper; it does not install a Study.
///
/// Paths are resolved after activation against the effective session root.
#[cfg(test)]
pub fn load_csv(path: &Path) -> Result<CsvLoadResult, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let study_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("csv")
        .to_string();

    let result = tdashboard_io::flat_csv::parse_flat_csv(&data, &study_name)?;

    let meta = crate::io::journal::convert_study_meta(result.meta);

    Ok((meta, result.images))
}

/// Resolves DesignExplorer `img` paths against the effective root, grouped by
/// trial_id. Files that don't exist are excluded; relative path rules are unchanged.
pub fn build_artifact_map(
    base_dir: &Path,
    images: &[(u32, String)],
) -> HashMap<u32, Vec<ArtifactEntry>> {
    tdashboard_io::artifacts::resolve_csv_images(base_dir, images)
}

/// Determines whether the `img` column's file name is a safe relative path that stays
/// within the artifact root. Returns `false` if it contains an absolute path
/// (`RootDir` / Windows `Prefix`) or a parent directory reference (`ParentDir` = `..`),
/// preventing parent-directory references (directory traversal).
#[cfg(test)]
use tdashboard_io::artifacts::is_safe_relative_filename;

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
    use std::path::PathBuf;

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

        let (meta, images) = load_csv(&csv_path).unwrap();
        let artifacts = build_artifact_map(base, &images);
        assert_eq!(meta.name, "data");
        assert_eq!(meta.param_names, vec!["x".to_string(), "y".to_string()]);
        assert_eq!(meta.objective_names, vec!["f".to_string()]);
        assert_eq!(meta.completed_trials, 2);
        // img0.png exists, missing.png is excluded.
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts.contains_key(&0));
    }
}
