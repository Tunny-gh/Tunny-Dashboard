use std::path::{Path, PathBuf};
pub use tdashboard_io::file::{write_atomic, AtomicOutput};

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
        .add_filter("DesignExplorer-format CSV (*.csv)", &["csv"])
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

/// Determines whether the path's extension is `.gh` (case-insensitive) — the
/// binary Grasshopper format, which this app cannot read. Used only to give a
/// helpful "save it as .ghx instead" message on drag & drop.
pub fn is_gh_binary_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("gh"))
}

/// Determines whether the path looks like an openable result storage file
/// (Optuna journal / SQLite / DesignExplorer CSV). Used by drag-and-drop to
/// decide whether a dropped non-.ghx file should be routed to the normal
/// open flow. The extension set matches `open_file_dialog`'s filter.
pub fn is_storage_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["log", "journal", "csv", "db", "sqlite", "sqlite3"]
            .iter()
            .any(|ext| e.eq_ignore_ascii_case(ext))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_atomic_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_atomic(&path, b"hello").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        // Only the target file remains in the directory (the temp file is consumed by rename).
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("out.txt")]);
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
