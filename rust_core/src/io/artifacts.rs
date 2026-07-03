//! REQ-007: Artifacts フォルダスキャン・パストラバーサル防止 (NFR-201)
//!
//! Optuna のアーティファクト機能では、ファイルの実体は `artifacts/<artifact_id>`
//! （拡張子なし、ファイル名 = artifact_id）として保存され、trial との対応・元のファイル名・
//! MIME タイプは Journal の `set_trial_system_attr`（キー `artifacts:<artifact_id>`）に
//! JSON 文字列で記録される。したがって **ファイル名から trial_id を推測することはできず**、
//! Journal のメタデータを参照して trial と artifact を結び付ける必要がある。

use std::collections::HashMap;
use std::io::BufRead;
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

    /// MIME タイプから種別を判定する（判定できない場合は None）。
    pub fn from_mime(mime: &str) -> Option<Self> {
        if mime.starts_with("image/") {
            Some(Self::Image)
        } else if mime == "text/csv" {
            Some(Self::Csv)
        } else {
            None
        }
    }

    /// MIME タイプを優先し、無ければ元ファイル名の拡張子で種別を判定する。
    pub fn classify(filename: &str, mime: &str) -> Self {
        Self::from_mime(mime).unwrap_or_else(|| Self::from_path(Path::new(filename)))
    }
}

// ============================================================
// ArtifactEntry / ArtifactMeta
// ============================================================

/// 表示用に解決済みの 1 アーティファクト。
/// `path` はディスク上の実体（`base_dir/<artifact_id>`）、`filename` は表示用の元ファイル名、
/// `mimetype` は種別判定用（不明な場合は空文字）。
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactEntry {
    pub path: PathBuf,
    pub filename: String,
    pub mimetype: String,
}

impl ArtifactEntry {
    pub fn file_type(&self) -> ArtifactFileType {
        ArtifactFileType::classify(&self.filename, &self.mimetype)
    }
}

/// Journal の `artifacts:<id>` システム属性値（JSON 文字列）をデコードした構造。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ArtifactMeta {
    #[serde(default)]
    pub artifact_id: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub mimetype: String,
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
// extract_trial_id（レガシーレイアウト用フォールバック）
// ============================================================

/// ファイル名/ディレクトリ名の先頭連続数値を trial_id として抽出する。
/// 例: `"42"` → `42`, `"42_result.png"` → `42`, `"result_42"` → `None`
///
/// Optuna 標準のアーティファクトストアでは使われないが、`artifacts/<trial_id>/file` のような
/// 独自レイアウトの後方互換フォールバックとして残す。
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
// parse_artifact_metadata
// ============================================================

/// Journal を走査し、`trial_id → [ArtifactMeta]` のマッピングを構築する。
///
/// Optuna は `set_trial_system_attr`（op_code 9, キー `system_attr`）またはトライアル生成時の
/// インライン `system_attrs`（op_code 4）に、キー `artifacts:<artifact_id>` で JSON 文字列を記録する。
/// trial_id は Journal 全体で一意なので study を区別せず全件まとめて返す。
pub fn parse_artifact_metadata(journal_path: &Path) -> HashMap<u32, Vec<ArtifactMeta>> {
    let mut map: HashMap<u32, Vec<ArtifactMeta>> = HashMap::new();
    let Ok(file) = std::fs::File::open(journal_path) else {
        return map;
    };
    let reader = std::io::BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        // `artifacts:` を含まない行は JSON パースを省略する（大きな Journal 対策）。
        if !line.contains("artifacts:") {
            continue;
        }
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        // trial_id は u32 の範囲に収まらない場合スキップする（黙って切り捨てない）。
        let Some(trial_id) = json
            .get("trial_id")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
        else {
            continue;
        };
        // op_code 9 は "system_attr"、op_code 4 のインラインは "system_attrs"。
        let Some(obj) = json
            .get("system_attr")
            .or_else(|| json.get("system_attrs"))
            .and_then(|v| v.as_object())
        else {
            continue;
        };
        for (attr_key, attr_val) in obj {
            let Some(artifact_id) = attr_key.strip_prefix("artifacts:") else {
                continue;
            };
            let Some(s) = attr_val.as_str() else {
                continue;
            };
            if let Ok(mut meta) = serde_json::from_str::<ArtifactMeta>(s) {
                if meta.artifact_id.is_empty() {
                    meta.artifact_id = artifact_id.to_string();
                }
                map.entry(trial_id).or_default().push(meta);
            }
        }
    }
    map
}

// ============================================================
// resolve_from_metadata / scan_legacy_layout
// ============================================================

/// メタデータを元に `base_dir/<artifact_id>` を解決する（Optuna 標準ストア）。
pub fn resolve_from_metadata(
    base_dir: &Path,
    meta_by_trial: &HashMap<u32, Vec<ArtifactMeta>>,
) -> HashMap<u32, Vec<ArtifactEntry>> {
    let mut out: HashMap<u32, Vec<ArtifactEntry>> = HashMap::new();
    for (&trial_id, metas) in meta_by_trial {
        for meta in metas {
            if meta.artifact_id.is_empty() {
                continue;
            }
            let path = base_dir.join(&meta.artifact_id);
            // 実体が存在し base_dir 内に収まるもののみ採用する（NFR-201）。
            if validate_path(base_dir, &path).is_err() {
                continue;
            }
            let filename = if meta.filename.is_empty() {
                meta.artifact_id.clone()
            } else {
                meta.filename.clone()
            };
            out.entry(trial_id).or_default().push(ArtifactEntry {
                path,
                filename,
                mimetype: meta.mimetype.clone(),
            });
        }
    }
    out
}

/// レガシーレイアウト（ファイル名/ディレクトリ名の先頭数値 = trial_id）をスキャンする。
pub fn scan_legacy_layout(base_dir: &Path) -> HashMap<u32, Vec<ArtifactEntry>> {
    let mut out: HashMap<u32, Vec<ArtifactEntry>> = HashMap::new();
    let Ok(entries) = std::fs::read_dir(base_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(trial_id) = extract_trial_id(&path) else {
            continue;
        };
        if validate_path(base_dir, &path).is_err() {
            continue;
        }
        if path.is_dir() {
            if let Ok(children) = std::fs::read_dir(&path) {
                for child in children.flatten() {
                    let child_path = child.path();
                    if validate_path(base_dir, &child_path).is_ok() {
                        out.entry(trial_id)
                            .or_default()
                            .push(make_entry(child_path));
                    }
                }
            }
        } else {
            out.entry(trial_id).or_default().push(make_entry(path));
        }
    }
    out
}

/// ディスク上のファイル名をそのまま表示名に使う `ArtifactEntry` を作る（レガシー用）。
fn make_entry(path: PathBuf) -> ArtifactEntry {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    ArtifactEntry {
        path,
        filename,
        mimetype: String::new(),
    }
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
    fn classify_prefers_mime_then_extension() {
        // 拡張子なし（artifact_id ファイル名）でも MIME で判定できる。
        assert_eq!(
            ArtifactFileType::classify("00b40d87-uuid", "image/png"),
            ArtifactFileType::Image
        );
        assert_eq!(
            ArtifactFileType::classify("result.png", ""),
            ArtifactFileType::Image
        );
        assert_eq!(
            ArtifactFileType::classify("data.csv", ""),
            ArtifactFileType::Csv
        );
        assert_eq!(
            ArtifactFileType::classify("blob", ""),
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

    // ── Journal メタデータ解析 ───────────────────────────────────

    #[test]
    fn parse_artifact_metadata_reads_system_attr_op9() {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("study.journal");
        // op_code 9: set_trial_system_attr。値は JSON 文字列。
        let line = r#"{"op_code":9,"worker_id":"w","trial_id":42,"system_attr":{"artifacts:abc123":"{\"artifact_id\": \"abc123\", \"filename\": \"result.png\", \"mimetype\": \"image/png\"}"}}"#;
        std::fs::write(&journal, format!("{line}\n")).unwrap();

        let map = parse_artifact_metadata(&journal);
        let metas = map.get(&42).expect("trial 42 should have metadata");
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0].artifact_id, "abc123");
        assert_eq!(metas[0].filename, "result.png");
        assert_eq!(metas[0].mimetype, "image/png");
    }

    #[test]
    fn parse_artifact_metadata_uses_key_suffix_when_id_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("study.journal");
        let line = r#"{"op_code":9,"trial_id":7,"system_attr":{"artifacts:def456":"{\"filename\": \"data.csv\", \"mimetype\": \"text/csv\"}"}}"#;
        std::fs::write(&journal, format!("{line}\n")).unwrap();

        let map = parse_artifact_metadata(&journal);
        let metas = map.get(&7).unwrap();
        assert_eq!(metas[0].artifact_id, "def456"); // キー接尾辞から補完
        assert_eq!(metas[0].filename, "data.csv");
    }

    #[test]
    fn parse_artifact_metadata_skips_out_of_range_trial_id() {
        // trial_id が u32 の範囲を超える場合は `as u32` で黙って切り捨てず、
        // そのエントリごとスキップする（監査で見つかった切り捨てバグの回帰テスト）。
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("study.journal");
        let oversized_trial_id = u64::from(u32::MAX) + 1;
        let inner = serde_json::json!({
            "filename": "result.png",
            "mimetype": "image/png",
        })
        .to_string();
        let record = serde_json::json!({
            "op_code": 9,
            "trial_id": oversized_trial_id,
            "system_attr": { "artifacts:abc123": inner },
        });
        std::fs::write(&journal, format!("{record}\n")).unwrap();

        let map = parse_artifact_metadata(&journal);

        assert!(
            map.is_empty(),
            "out-of-range trial_id must be skipped, not silently truncated"
        );
    }

    #[test]
    fn resolve_from_metadata_matches_files_on_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        // artifact_id をファイル名として実体を作成（拡張子なし）。
        std::fs::write(base.join("abc123"), b"img").unwrap();
        // 存在しない artifact は除外されることも確認。
        let mut meta_by_trial: HashMap<u32, Vec<ArtifactMeta>> = HashMap::new();
        meta_by_trial.insert(
            42,
            vec![
                ArtifactMeta {
                    artifact_id: "abc123".into(),
                    filename: "result.png".into(),
                    mimetype: "image/png".into(),
                },
                ArtifactMeta {
                    artifact_id: "missing".into(),
                    filename: "gone.png".into(),
                    mimetype: "image/png".into(),
                },
            ],
        );

        let out = resolve_from_metadata(base, &meta_by_trial);
        let entries = out.get(&42).unwrap();
        assert_eq!(entries.len(), 1, "存在するファイルのみ採用");
        assert_eq!(entries[0].filename, "result.png");
        assert_eq!(entries[0].file_type(), ArtifactFileType::Image);
        assert!(entries[0].path.ends_with("abc123"));
    }

    #[test]
    fn scan_legacy_layout_groups_by_leading_digits() {
        let tmp = tempfile::TempDir::new().unwrap();
        let base = tmp.path();
        for (dir_name, file_name) in [("0", "result.png"), ("1", "data.csv")] {
            let dir = base.join(dir_name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(file_name), b"dummy").unwrap();
        }
        let out = scan_legacy_layout(base);
        assert_eq!(out.len(), 2);
        assert!(out.contains_key(&0));
        assert!(out.contains_key(&1));
    }
}
