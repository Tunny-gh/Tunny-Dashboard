//! DesignExplorer CSV path checks and session-root composition. Format decoding
//! and safe resolution live in I/O; the entry applies owned batches to managers.

use std::collections::HashMap;
use std::path::Path;

use crate::io::artifacts::ArtifactEntry;

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

    #[test]
    fn is_csv_path_detects_extension() {
        assert!(is_csv_path(Path::new("data.csv")));
        assert!(is_csv_path(Path::new("DATA.CSV")));
        assert!(!is_csv_path(Path::new("study.log")));
        assert!(!is_csv_path(Path::new("noext")));
    }

    #[test]
    fn is_safe_relative_filename_accepts_plain_names() {
        assert!(is_safe_relative_filename("a.png"));
        assert!(is_safe_relative_filename("sub/dir/a.png"));
        assert!(is_safe_relative_filename("./a.png"));
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
}
