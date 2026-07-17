//! REQ-007: Artifacts folder scanning and path traversal prevention (NFR-201)
//!
//! In Optuna's artifact feature, the actual file is stored as `artifacts/<artifact_id>`
//! (no extension, filename = artifact_id), and the mapping to the trial, the original
//! filename, and the MIME type are recorded as a JSON string in the Journal's
//! `set_trial_system_attr` (key `artifacts:<artifact_id>`). Therefore **the trial_id
//! cannot be inferred from the filename**, and the Journal metadata must be consulted
//! to link a trial to its artifacts.

use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

// ============================================================
// ArtifactFileType
// ============================================================

/// Artifact file type (REQ-007-E/F/G)
#[derive(Debug, Clone, PartialEq)]
pub enum ArtifactFileType {
    /// PNG / JPG / JPEG / GIF / WEBP
    Image,
    /// CSV
    Csv,
    /// Other
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

    /// Determines the type from the MIME type (returns None if it cannot be determined).
    pub fn from_mime(mime: &str) -> Option<Self> {
        if mime.starts_with("image/") {
            Some(Self::Image)
        } else if mime == "text/csv" {
            Some(Self::Csv)
        } else {
            None
        }
    }

    /// Prefers the MIME type, falling back to the original filename's extension if absent.
    pub fn classify(filename: &str, mime: &str) -> Self {
        Self::from_mime(mime).unwrap_or_else(|| Self::from_path(Path::new(filename)))
    }
}

// ============================================================
// ArtifactEntry / ArtifactMeta
// ============================================================

/// A single artifact resolved for display.
/// `path` is the on-disk entity (`base_dir/<artifact_id>`), `filename` is the original
/// filename used for display, and `mimetype` is used for type classification (empty
/// string if unknown).
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

/// Structure decoded from the Journal's `artifacts:<id>` system attribute value (a JSON string).
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
    /// Attempted access outside base_dir (NFR-201)
    PathTraversal,
    /// IO error
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
// validate_path (NFR-201)
// ============================================================

/// Safe path validation that prevents path traversal outside `base_dir`.
/// Returns `Err(ArtifactsError::PathTraversal)` if a path containing `../` or similar is detected.
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
// extract_trial_id (fallback for the legacy layout)
// ============================================================

/// Extracts the leading run of digits in a filename/directory name as the trial_id.
/// Example: `"42"` → `42`, `"42_result.png"` → `42`, `"result_42"` → `None`
///
/// Not used by Optuna's standard artifact store, but kept as a backward-compatibility
/// fallback for custom layouts such as `artifacts/<trial_id>/file`.
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

/// Scans the Journal and builds a `trial_id → [ArtifactMeta]` mapping.
///
/// Optuna records a JSON string under the key `artifacts:<artifact_id>`, either in
/// `set_trial_system_attr` (op_code 9, key `system_attr`) or in the inline `system_attrs`
/// (op_code 4) emitted at trial creation. Since trial_id is unique across the whole Journal,
/// all entries are returned together without distinguishing by study.
pub fn parse_artifact_metadata(journal_path: &Path) -> HashMap<u32, Vec<ArtifactMeta>> {
    let mut map: HashMap<u32, Vec<ArtifactMeta>> = HashMap::new();
    let Ok(file) = std::fs::File::open(journal_path) else {
        return map;
    };
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        // Lines that fail to read (e.g. non-UTF-8) are skipped individually while
        // scanning continues (`map_while(Result::ok)` would abort the entire scan
        // at the first invalid line and silently drop the remaining metadata, so
        // it is not used here).
        let Ok(line) = line else {
            continue;
        };
        // Skip JSON parsing for lines that don't contain `artifacts:` (optimization for large Journals).
        if !line.contains("artifacts:") {
            continue;
        }
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        // Skip if trial_id doesn't fit in u32 (do not silently truncate).
        let Some(trial_id) = json
            .get("trial_id")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
        else {
            continue;
        };
        // op_code 9 uses "system_attr"; the inline form for op_code 4 uses "system_attrs".
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

/// Resolves `base_dir/<artifact_id>` based on the metadata (Optuna's standard store).
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
            // Only accept entries whose file exists and stays within base_dir (NFR-201).
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

/// Scans the legacy layout (leading number in filename/directory name = trial_id).
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

/// Builds an `ArtifactEntry` that uses the on-disk filename directly as the display name (for legacy layout).
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
// Tests
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
        // Even without an extension (artifact_id as filename), MIME allows classification.
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
    fn test_validate_path_within_base() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("file.png");
        std::fs::write(&file, b"test").unwrap();
        let result = validate_path(tmp.path(), &file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_path_traversal_blocked() {
        // NFR-201: reject paths outside base_dir
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();
        let file_in_other = tmp2.path().join("secret.txt");
        std::fs::write(&file_in_other, b"secret").unwrap();
        let result = validate_path(tmp1.path(), &file_in_other);
        assert!(matches!(result, Err(ArtifactsError::PathTraversal)));
    }

    // ── Journal metadata parsing ───────────────────────────────────

    #[test]
    fn parse_artifact_metadata_reads_system_attr_op9() {
        let tmp = tempfile::tempdir().unwrap();
        let journal = tmp.path().join("study.journal");
        // op_code 9: set_trial_system_attr. The value is a JSON string.
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
        assert_eq!(metas[0].artifact_id, "def456"); // filled in from the key suffix
        assert_eq!(metas[0].filename, "data.csv");
    }

    #[test]
    fn parse_artifact_metadata_skips_out_of_range_trial_id() {
        // When trial_id exceeds the u32 range, the whole entry is skipped instead of
        // silently truncating it via `as u32` (regression test for a truncation bug
        // found during audit).
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
        // Create the file using artifact_id as the filename (no extension).
        std::fs::write(base.join("abc123"), b"img").unwrap();
        // Also verify that a nonexistent artifact is excluded.
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
