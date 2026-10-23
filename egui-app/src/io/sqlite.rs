use std::path::Path;

/// Determines whether a path is an Optuna SQLite storage (extension db/sqlite/sqlite3,
/// case-insensitive).
pub fn is_sqlite_path(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        e.eq_ignore_ascii_case("db")
            || e.eq_ignore_ascii_case("sqlite")
            || e.eq_ignore_ascii_case("sqlite3")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_sqlite_path_detects_extensions() {
        assert!(is_sqlite_path(Path::new("study.db")));
        assert!(is_sqlite_path(Path::new("STUDY.DB")));
        assert!(is_sqlite_path(Path::new("study.sqlite")));
        assert!(is_sqlite_path(Path::new("study.sqlite3")));
        assert!(!is_sqlite_path(Path::new("study.log")));
        assert!(!is_sqlite_path(Path::new("study.csv")));
        assert!(!is_sqlite_path(Path::new("noext")));
    }
}
