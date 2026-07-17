use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Sequence number to avoid collisions in `write_atomic`'s temp file names.
static TMP_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Atomically overwrites a file (writes to a temp file, then `rename`s it).
///
/// `std::fs::write` performs a non-atomic "truncate -> write" operation, so a disk-full
/// condition or crash midway through an overwrite can corrupt or lose the existing
/// file. This function writes the entire content to a temp file in the same directory
/// first, then replaces it via `rename`, so the existing file is left untouched if it
/// fails (`rename` is only atomic within the same file system, so the temp file must
/// always be created in the same directory as the target — on a different file system,
/// `rename` would fail).
///
/// If either the write or the `rename` fails, this attempts to clean up the temp file
/// before returning the error.
pub fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    // Place the temp file in the same directory as the target (avoids a cross-FS rename).
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("tunny");
    let seq = TMP_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_name = format!(".{base}.tmp-{}-{seq}", std::process::id());
    let tmp_path = dir.join(tmp_name);

    // Write the temp file fully, then rename. Clean up the temp file if it fails partway.
    if let Err(e) = write_all_to_new_file(&tmp_path, contents) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }
    Ok(())
}

/// Creates a new file, writes the content in full, and flushes it (the temp-file-write
/// part of `write_atomic`).
fn write_all_to_new_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(contents)?;
    f.flush()?;
    Ok(())
}

/// Selects an optimization result file via the native file dialog.
/// The filter names make it clear that `.log` is Optuna's Journal storage, `.csv` is
/// the format for DesignExplorer, `.db`/`.sqlite`/`.sqlite3` is Optuna's RDB (SQLite)
/// storage, and `.ghx` is a Grasshopper definition (selecting it makes
/// `TunnyApp::open_path` open the optimization settings modal).
pub fn open_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(
            "Optimization Result (Optuna .log / DesignExplorer .csv / Optuna SQLite .db/.sqlite/.sqlite3 / Grasshopper .ghx)",
            &["log", "csv", "db", "sqlite", "sqlite3", "ghx"],
        )
        .add_filter("Optuna Result (*.log)", &["log"])
        .add_filter("DesignExplorer (*.csv)", &["csv"])
        .add_filter(
            "Optuna SQLite (.db/.sqlite/.sqlite3)",
            &["db", "sqlite", "sqlite3"],
        )
        .add_filter("Grasshopper (*.ghx)", &["ghx"])
        .pick_file()
}

/// Determines whether the path's extension is `.ghx` (case-insensitive).
/// Used by both drag-and-drop and `TunnyApp::open_path` to route to the .ghx path
/// (optimization settings modal) (same shape as `io::flat_csv::is_csv_path` /
/// `io::sqlite::is_sqlite_path`).
pub fn is_ghx_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("ghx"))
}

/// Reads a file as a byte buffer.
pub fn read_journal_file(path: &PathBuf) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_existing_file_succeeds() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "test content").unwrap();
        let path = tmp.path().to_path_buf();
        let result = read_journal_file(&path);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn read_nonexistent_file_returns_err() {
        let path = PathBuf::from("/nonexistent/path/file.log");
        let result = read_journal_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn write_atomic_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_atomic(&path, b"hello").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn write_atomic_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        std::fs::write(&path, b"old contents").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn write_atomic_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_atomic(&path, b"data").unwrap();
        // Only the target file remains in the directory (the temp file is consumed by rename).
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("out.txt")]);
    }

    #[test]
    fn write_atomic_preserves_original_on_bad_directory() {
        // Returns Err when the parent directory doesn't exist (existing file is untouched).
        let path = PathBuf::from("/nonexistent_dir_write_atomic/out.txt");
        assert!(write_atomic(&path, b"x").is_err());
    }

    #[test]
    fn is_ghx_path_matches_case_insensitive() {
        assert!(is_ghx_path(&PathBuf::from("model.ghx")));
        assert!(is_ghx_path(&PathBuf::from("Model.GHX")));
        assert!(is_ghx_path(&PathBuf::from("/a/b/model.Ghx")));
        assert!(!is_ghx_path(&PathBuf::from("model.gh")));
        assert!(!is_ghx_path(&PathBuf::from("model.log")));
        assert!(!is_ghx_path(&PathBuf::from("model")));
    }
}
