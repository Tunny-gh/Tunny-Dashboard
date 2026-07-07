use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// `write_atomic` の一時ファイル名を衝突させないための連番。
static TMP_WRITE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// ファイルをアトミックに上書きする（一時ファイルへ書いてから `rename`）。
///
/// `std::fs::write` は「truncate → 書込み」の非アトミック操作のため、上書き途中の
/// ディスク満杯・クラッシュで既存ファイルを破損・消失させうる。本関数は同一ディレクトリ内の
/// 一時ファイルへ全内容を書き切ってから `rename` で置き換えるため、失敗しても既存ファイルは
/// 元のまま残る（`rename` は同一ファイルシステム内でのみアトミックなので、一時ファイルは
/// 必ず対象と同じディレクトリに作る — 別ファイルシステムだと `rename` が失敗する）。
///
/// 書込み・`rename` のいずれかが失敗した場合は一時ファイルの後始末を試みてからエラーを返す。
pub fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    // 対象と同一ディレクトリに一時ファイルを置く（別 FS への rename を避ける）。
    let dir = match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let base = path.file_name().and_then(|n| n.to_str()).unwrap_or("tunny");
    let seq = TMP_WRITE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp_name = format!(".{base}.tmp-{}-{seq}", std::process::id());
    let tmp_path = dir.join(tmp_name);

    // 一時ファイルへ書き切ってから rename する。途中で失敗したら一時ファイルを掃除する。
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

/// 新規ファイルを作成し内容を書き切ってフラッシュする（`write_atomic` の一時ファイル書込み部）。
fn write_all_to_new_file(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(contents)?;
    f.flush()?;
    Ok(())
}

/// ネイティブファイルダイアログで最適化結果ファイルを選択する。
/// `.log` は Optuna の Journal ストレージ、`.csv` は DesignExplorer 向け形式、
/// `.db`/`.sqlite`/`.sqlite3` は Optuna の RDB（SQLite）ストレージであることが
/// フィルタ名から分かるようにする。
pub fn open_file_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter(
            "Optimization Result (Optuna .log / DesignExplorer .csv / Optuna SQLite .db/.sqlite/.sqlite3)",
            &["log", "csv", "db", "sqlite", "sqlite3"],
        )
        .add_filter("Optuna Result (*.log)", &["log"])
        .add_filter("DesignExplorer (*.csv)", &["csv"])
        .add_filter(
            "Optuna SQLite (.db/.sqlite/.sqlite3)",
            &["db", "sqlite", "sqlite3"],
        )
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
        // ディレクトリには対象ファイルのみが残る（一時ファイルは rename で消費される）。
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("out.txt")]);
    }

    #[test]
    fn write_atomic_preserves_original_on_bad_directory() {
        // 親ディレクトリが存在しない場合は Err を返す（既存ファイルは触らない）。
        let path = PathBuf::from("/nonexistent_dir_write_atomic/out.txt");
        assert!(write_atomic(&path, b"x").is_err());
    }
}
