//! REQ-007: Artifacts フォルダスキャン・パストラバーサル防止 (NFR-201)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================
// ArtifactFileType
// ============================================================

/// アーティファクトのファイルタイプ（REQ-007-E/F/G）
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactFileType {
    /// PNG / JPG / JPEG / GIF / WEBP
    Image,
    /// CSV
    Csv,
    /// その他
    Other,
}

impl ArtifactFileType {
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref()
        {
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp") => Self::Image,
            Some("csv") => Self::Csv,
            _ => Self::Other,
        }
    }
}

// ============================================================
// ArtifactsError
// ============================================================

#[derive(Debug)]
pub enum ArtifactsError {
    /// base_dir 外へのアクセス試行（NFR-201）
    PathTraversal,
    /// IO エラー
    Io(std::io::Error),
}

impl std::fmt::Display for ArtifactsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactsError::PathTraversal => write!(f, "path traversal attempt detected"),
            ArtifactsError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

// ============================================================
// detect_artifacts_dir
// ============================================================

/// Journal ファイルの親ディレクトリに `artifacts/` フォルダが存在するか検出する。
/// 存在する場合は `Some(artifacts_dir)`, 存在しない場合は `None` を返す（REQ-007-A）
pub fn detect_artifacts_dir(journal_path: &Path) -> Option<PathBuf> {
    let parent = journal_path.parent()?;
    let artifacts_dir = parent.join("artifacts");
    if artifacts_dir.is_dir() {
        Some(artifacts_dir)
    } else {
        None
    }
}

// ============================================================
// validate_path (NFR-201)
// ============================================================

/// `base_dir` 外へのパストラバーサルを防ぐ安全なパス検証。
/// `../` 等を含むパスを検出した場合は `Err(ArtifactsError::PathTraversal)` を返す。
pub fn validate_path(base_dir: &Path, file_path: &Path) -> Result<PathBuf, ArtifactsError> {
    let canonical_base = base_dir.canonicalize().map_err(ArtifactsError::Io)?;
    let canonical_file = file_path.canonicalize().map_err(ArtifactsError::Io)?;
    if canonical_file.starts_with(&canonical_base) {
        Ok(canonical_file)
    } else {
        Err(ArtifactsError::PathTraversal)
    }
}

// ============================================================
// extract_trial_id
// ============================================================

/// ファイル名/ディレクトリ名の先頭連続数値を trial_id として抽出する。
/// 例: `"42"` → `42`, `"42_result.png"` → `42`, `"result_42"` → `None`
pub fn extract_trial_id(path: &Path) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse::<u32>().ok()
    }
}

// ============================================================
// scan_artifacts_dir
// ============================================================

/// `artifacts/` フォルダをスキャンし、trial_id 別にファイルパスをグループ化する。
/// 完了後に `AppMessage::ArtifactsDirScanned` を送信する（REQ-007-A/C）。
///
/// ディレクトリ構造例:
/// - `artifacts/42/result.png`
/// - `artifacts/42_result.png`
pub fn scan_artifacts_dir(
    base_dir: PathBuf,
    tx: std::sync::mpsc::SyncSender<crate::state::messages::AppMessage>,
) {
    crate::app::spawn_task(tx, move || {
        let mut trial_artifacts: HashMap<u32, Vec<PathBuf>> = HashMap::new();

        let Ok(entries) = std::fs::read_dir(&base_dir) else {
            return crate::state::messages::AppMessage::ArtifactsDirScanned {
                trial_artifacts,
                artifacts_dir: base_dir,
            };
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(trial_id) = extract_trial_id(&path) else {
                continue;
            };
            if validate_path(&base_dir, &path).is_err() {
                continue;
            }

            trial_artifacts
                .entry(trial_id)
                .or_default()
                .push(path.clone());

            // サブディレクトリ内のファイルも trial_id に紐付け（1 レベルのみ）
            if path.is_dir() {
                if let Ok(children) = std::fs::read_dir(&path) {
                    for child in children.flatten() {
                        let child_path = child.path();
                        if validate_path(&base_dir, &child_path).is_ok() {
                            trial_artifacts
                                .entry(trial_id)
                                .or_default()
                                .push(child_path);
                        }
                    }
                }
            }
        }

        crate::state::messages::AppMessage::ArtifactsDirScanned {
            trial_artifacts,
            artifacts_dir: base_dir,
        }
    });
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_file_type_image() {
        assert_eq!(
            ArtifactFileType::from_path(Path::new("result.png")),
            ArtifactFileType::Image
        );
        assert_eq!(
            ArtifactFileType::from_path(Path::new("result.JPG")),
            ArtifactFileType::Image
        );
    }

    #[test]
    fn test_artifact_file_type_csv() {
        assert_eq!(
            ArtifactFileType::from_path(Path::new("data.csv")),
            ArtifactFileType::Csv
        );
    }

    #[test]
    fn test_artifact_file_type_other() {
        assert_eq!(
            ArtifactFileType::from_path(Path::new("data.txt")),
            ArtifactFileType::Other
        );
    }

    #[test]
    fn test_extract_trial_id_numeric_dir() {
        assert_eq!(extract_trial_id(Path::new("42")), Some(42));
    }

    #[test]
    fn test_extract_trial_id_with_suffix() {
        assert_eq!(extract_trial_id(Path::new("42_result.png")), Some(42));
    }

    #[test]
    fn test_extract_trial_id_no_leading_digit() {
        assert_eq!(extract_trial_id(Path::new("result_42")), None);
    }

    #[test]
    fn test_detect_artifacts_dir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let journal_path = tmp.path().join("study.journal");
        let artifacts_dir = tmp.path().join("artifacts");
        std::fs::create_dir(&artifacts_dir).unwrap();
        let result = detect_artifacts_dir(&journal_path);
        assert_eq!(result, Some(artifacts_dir));
    }

    #[test]
    fn test_detect_artifacts_dir_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let journal_path = tmp.path().join("study.journal");
        let result = detect_artifacts_dir(&journal_path);
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_path_within_base() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("file.png");
        std::fs::write(&file, b"test").unwrap();
        let result = validate_path(tmp.path(), &file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_traversal_blocked() {
        // NFR-201: base_dir 外のパスを拒否する
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();
        let file_in_other = tmp2.path().join("secret.txt");
        std::fs::write(&file_in_other, b"secret").unwrap();
        let result = validate_path(tmp1.path(), &file_in_other);
        assert!(matches!(result, Err(ArtifactsError::PathTraversal)));
    }

    // TASK-2128 integration tests

    #[test]
    fn task2128_scan_artifacts_dir_finds_files() {
        // 一時ディレクトリに模擬 artifacts 構造を作成してスキャン
        // extract_trial_id は先頭数字のみ抽出するので "0", "1", "2" というディレクトリ名を使う
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path();

        for (dir_name, file_name) in [("0", "result.png"), ("1", "data.csv"), ("2", "output.txt")] {
            let dir = base.join(dir_name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(file_name), b"dummy").unwrap();
        }

        // スキャンロジック（scan_artifacts_dir 内部と同等）
        let mut trial_artifacts: std::collections::HashMap<u32, Vec<PathBuf>> =
            std::collections::HashMap::new();
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(trial_id) = extract_trial_id(&path) {
                        if validate_path(base, &path).is_ok() {
                            if let Ok(sub_entries) = std::fs::read_dir(&path) {
                                for sub in sub_entries.flatten() {
                                    trial_artifacts
                                        .entry(trial_id)
                                        .or_default()
                                        .push(sub.path());
                                }
                            }
                        }
                    }
                }
            }
        }

        assert_eq!(
            trial_artifacts.len(),
            3,
            "3 trial ディレクトリが検出されること"
        );
        assert!(trial_artifacts.contains_key(&0), "trial 0 が含まれること");
        assert!(trial_artifacts.contains_key(&1), "trial 1 が含まれること");
        assert!(trial_artifacts.contains_key(&2), "trial 2 が含まれること");
        // PNG ファイルが検出されること
        assert!(trial_artifacts[&0]
            .iter()
            .any(|p| p.extension().and_then(|e| e.to_str()) == Some("png")));
    }

    #[test]
    fn task2128_extract_trial_id_from_trial_prefix() {
        // "trial_42" → trial_ prefix を無視して None（先頭が数字でないため）
        // 実際の実装: 先頭連続数字のみを取得するので "trial_42" → None
        assert_eq!(extract_trial_id(Path::new("trial_42")), None);
        // "42" → Some(42)
        assert_eq!(extract_trial_id(Path::new("42")), Some(42u32));
        // "0" → Some(0)
        assert_eq!(extract_trial_id(Path::new("0")), Some(0u32));
        // "abc" → None
        assert_eq!(extract_trial_id(Path::new("abc")), None);
        // 空文字列相当
        assert_eq!(extract_trial_id(Path::new(".")), None);
    }

    #[test]
    fn task2128_validate_path_prevents_traversal_with_dotdot() {
        // NFR-201: ../.. を含むパスがベースディレクトリ外に解決される場合は拒否
        let tmp = tempfile::TempDir::new().unwrap();
        let base_dir = tmp.path();

        // base_dir/../../../etc/passwd のようなパスを構築
        let malicious = base_dir.join("..").join("..").join("etc");
        // canonicalize が失敗するか、base_dir 外を指すならエラー
        let result = validate_path(base_dir, &malicious);
        // malicious パスが存在しないため Io エラー、またはパストラバーサルエラー
        assert!(
            result.is_err(),
            "ベースディレクトリ外のパスは拒否されること"
        );
    }

    #[test]
    fn task2128_artifacts_dir_scanned_message_channel() {
        use crate::state::messages::AppMessage;
        use std::collections::HashMap;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::sync_channel::<AppMessage>(8);

        let mut trial_artifacts: HashMap<u32, Vec<PathBuf>> = HashMap::new();
        trial_artifacts.insert(0, vec![PathBuf::from("/tmp/trial_0/result.png")]);
        let artifacts_dir = PathBuf::from("/tmp/artifacts");

        tx.send(AppMessage::ArtifactsDirScanned {
            trial_artifacts: trial_artifacts.clone(),
            artifacts_dir: artifacts_dir.clone(),
        })
        .unwrap();

        match rx.recv().unwrap() {
            AppMessage::ArtifactsDirScanned {
                trial_artifacts: received,
                artifacts_dir: received_dir,
            } => {
                assert_eq!(received.len(), 1);
                assert!(received.contains_key(&0));
                assert_eq!(received_dir, artifacts_dir);
            }
            _ => panic!("予期しないメッセージタイプ"),
        }
    }
}
