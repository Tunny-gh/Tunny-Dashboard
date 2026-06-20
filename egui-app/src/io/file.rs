use std::path::PathBuf;

/// ネイティブファイルダイアログで最適化結果ファイルを選択する。
/// `.log` は Optuna の結果ファイル、`.csv` は DesignExplorer 向け形式であることが
/// フィルタ名から分かるようにする。
pub fn open_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(
            "Optimization Result (Optuna .log / DesignExplorer .csv)",
            &["log", "csv"],
        )
        .add_filter("Optuna Result (*.log)", &["log"])
        .add_filter("DesignExplorer (*.csv)", &["csv"])
        .pick_file()
}

/// ファイルをバイト列として読み込む
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
}
