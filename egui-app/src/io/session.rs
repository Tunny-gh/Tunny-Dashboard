use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// 補助型
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutConfig {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<GridCellSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCellSnapshot {
    pub row: usize,
    pub col: usize,
    pub content: Option<String>,
    pub col_span: u8,
    pub row_span: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// "objective" | "parameter"
    pub space: String,
    pub k: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSnapshot {
    pub filter_ranges: HashMap<String, (f64, f64)>,
    pub selected_indices: Vec<u32>,
    pub color_mode: String,
}

fn default_version() -> String {
    "1.0".to_string()
}

// ============================================================
// SessionSnapshot
// ============================================================

/// セッションスナップショット（保存・復元用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    // --- 既存フィールド（変更なし）---
    pub study_name: String,
    pub filter_ranges: HashMap<String, (f64, f64)>,
    pub selected_indices: Vec<u32>,
    pub saved_at: String,

    // --- REQ-002-B: フィルタ拡張 ---
    #[serde(default)]
    pub color_mode: String,

    // --- REQ-004-B: セッション識別 ---
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub journal_filename: String,
    #[serde(default)]
    pub selected_study_id: Option<u32>,

    // --- REQ-001 + REQ-004: Trade-off Navigator ---
    #[serde(default)]
    pub tradeoff_weights: Vec<f64>,

    // --- REQ-004: クラスタリング設定 ---
    #[serde(default)]
    pub cluster_config: Option<ClusterConfig>,

    // --- REQ-003 + REQ-004: レイアウト設定 ---
    #[serde(default)]
    pub layout_mode: String,
    #[serde(default)]
    pub layout_config: Option<LayoutConfig>,

    // --- REQ-004: ピン留め試行 ---
    #[serde(default)]
    pub pinned_trials: Vec<u32>,

    // --- REQ-009 + REQ-003: PCP 軸状態 ---
    #[serde(default)]
    pub pcp_axis_order: Vec<String>,
    #[serde(default)]
    pub pcp_axis_visibility: HashMap<String, bool>,
}

impl SessionSnapshot {
    pub fn new(
        study_name: String,
        filter_ranges: HashMap<String, (f64, f64)>,
        selected_indices: Vec<u32>,
    ) -> Self {
        Self {
            study_name,
            filter_ranges,
            selected_indices,
            saved_at: "".to_string(),
            color_mode: String::new(),
            version: default_version(),
            journal_filename: String::new(),
            selected_study_id: None,
            tradeoff_weights: Vec::new(),
            cluster_config: None,
            layout_mode: String::new(),
            layout_config: None,
            pinned_trials: Vec::new(),
            pcp_axis_order: Vec::new(),
            pcp_axis_visibility: HashMap::new(),
        }
    }
}

// ============================================================
// シリアライズ / デシリアライズ
// ============================================================

/// SessionSnapshot を JSON 文字列にシリアライズする
pub fn serialize_session(snapshot: &SessionSnapshot) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(snapshot)
}

/// JSON 文字列から SessionSnapshot をデシリアライズする
pub fn deserialize_session(json: &str) -> Result<SessionSnapshot, serde_json::Error> {
    serde_json::from_str(json)
}

// ============================================================
// フィルタ JSON 保存/読み込み（REQ-002）
// ============================================================

/// フィルタ条件を FilterSnapshot として JSON に書き出す
/// 戻り値: 書き込んだバイト列（テスト可能な純粋関数）
pub fn encode_filter_json(
    filter_ranges: &HashMap<String, (f64, f64)>,
    selected_indices: &[u32],
    color_mode: &str,
) -> Result<String, serde_json::Error> {
    let snap = FilterSnapshot {
        filter_ranges: filter_ranges.clone(),
        selected_indices: selected_indices.to_vec(),
        color_mode: color_mode.to_string(),
    };
    serde_json::to_string_pretty(&snap)
}

/// FilterSnapshot を JSON 文字列から復元する
pub fn decode_filter_json(json: &str) -> Result<FilterSnapshot, serde_json::Error> {
    serde_json::from_str(json)
}

/// フィルタ JSON をファイルダイアログで保存する（UI 呼び出し用）
pub fn save_filter_json(
    filter_ranges: &HashMap<String, (f64, f64)>,
    selected_indices: &[u32],
    color_mode: &str,
) {
    if let Ok(json) = encode_filter_json(filter_ranges, selected_indices, color_mode) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("filter.json")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            let _ = std::fs::write(path, json);
        }
    }
}

/// フィルタ JSON をファイルダイアログで読み込む（UI 呼び出し用）
pub fn load_filter_json() -> Option<FilterSnapshot> {
    let path = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()?;
    let json = std::fs::read_to_string(path).ok()?;
    decode_filter_json(&json).ok()
}

// ============================================================
// レイアウト JSON 保存/読み込み（REQ-003）
// ============================================================

/// LayoutConfig を JSON 文字列にエンコードする
pub fn encode_layout_json(config: &LayoutConfig) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(config)
}

/// JSON 文字列から LayoutConfig を復元する
pub fn decode_layout_json(json: &str) -> Result<LayoutConfig, serde_json::Error> {
    serde_json::from_str(json)
}

/// レイアウト JSON をファイルダイアログで保存する（UI 呼び出し用）
pub fn save_layout_json(config: &LayoutConfig) {
    if let Ok(json) = encode_layout_json(config) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("layout.json")
            .add_filter("JSON", &["json"])
            .save_file()
        {
            let _ = std::fs::write(path, json);
        }
    }
}

/// レイアウト JSON をファイルダイアログで読み込む（UI 呼び出し用）
pub fn load_layout_json() -> Option<LayoutConfig> {
    let path = rfd::FileDialog::new()
        .add_filter("JSON", &["json"])
        .pick_file()?;
    let json = std::fs::read_to_string(path).ok()?;
    decode_layout_json(&json).ok()
}

// ============================================================
// .tdash 形式 セッション保存/読み込み（REQ-004）
// ============================================================

/// SessionSnapshot をファイルダイアログで .tdash として保存する
pub fn save_session(snapshot: &SessionSnapshot) {
    if let Ok(json) = serialize_session(snapshot) {
        let filename = format!("{}.tdash", snapshot.study_name);
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&filename)
            .add_filter("Tunny Dashboard Session", &["tdash"])
            .save_file()
        {
            let _ = std::fs::write(path, json);
        }
    }
}

/// .tdash ファイルをファイルダイアログで読み込む
pub fn load_session() -> Option<SessionSnapshot> {
    let path = rfd::FileDialog::new()
        .add_filter("Tunny Dashboard Session", &["tdash"])
        .pick_file()?;
    let json = std::fs::read_to_string(path).ok()?;
    deserialize_session(&json).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot() -> SessionSnapshot {
        let mut filter_ranges = HashMap::new();
        filter_ranges.insert("x".to_string(), (0.1, 0.9));
        SessionSnapshot::new("test_study".to_string(), filter_ranges, vec![0, 1, 2])
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let snapshot = make_snapshot();
        let json = serialize_session(&snapshot).expect("serialize failed");
        let restored = deserialize_session(&json).expect("deserialize failed");
        assert_eq!(restored.study_name, "test_study");
        assert_eq!(restored.selected_indices, vec![0, 1, 2]);
        assert_eq!(restored.filter_ranges.get("x"), Some(&(0.1, 0.9)));
    }

    #[test]
    fn deserialize_invalid_json_returns_error() {
        let result = deserialize_session("{invalid json}");
        assert!(result.is_err());
    }

    #[test]
    fn filter_ranges_preserved_after_roundtrip() {
        let snapshot = make_snapshot();
        let json = serialize_session(&snapshot).unwrap();
        let restored = deserialize_session(&json).unwrap();
        assert_eq!(
            snapshot.filter_ranges.get("x"),
            restored.filter_ranges.get("x")
        );
    }

    // --- TASK-2115 テスト ---

    #[test]
    fn task2115_backward_compatible_old_json() {
        // 旧形式（新フィールドなし）のJSONをデシリアライズしてもパニックしない
        let old_json = r#"{
            "study_name": "old_study",
            "filter_ranges": {},
            "selected_indices": [1, 2],
            "saved_at": "2024-01-01"
        }"#;
        let result = deserialize_session(old_json);
        assert!(result.is_ok());
        let snap = result.unwrap();
        assert_eq!(snap.study_name, "old_study");
        assert_eq!(snap.version, "1.0"); // default_version()
        assert!(snap.tradeoff_weights.is_empty());
        assert!(snap.pcp_axis_visibility.is_empty());
    }

    #[test]
    fn task2115_new_fields_roundtrip() {
        let mut snap = make_snapshot();
        snap.tradeoff_weights = vec![0.3, 0.7];
        snap.pcp_axis_order = vec!["x".to_string(), "y".to_string()];
        snap.pcp_axis_visibility.insert("x".to_string(), true);
        snap.pinned_trials = vec![5, 10];
        snap.layout_mode = "FreeLayout".to_string();

        let json = serialize_session(&snap).unwrap();
        let restored = deserialize_session(&json).unwrap();

        assert_eq!(restored.tradeoff_weights, vec![0.3, 0.7]);
        assert_eq!(restored.pcp_axis_order, vec!["x", "y"]);
        assert_eq!(restored.pcp_axis_visibility.get("x"), Some(&true));
        assert_eq!(restored.pinned_trials, vec![5, 10]);
        assert_eq!(restored.layout_mode, "FreeLayout");
    }

    #[test]
    fn task2115_encode_decode_filter_json() {
        let mut ranges = HashMap::new();
        ranges.insert("x".to_string(), (0.0, 1.0));
        let json = encode_filter_json(&ranges, &[1, 2], "ParetoRank").unwrap();
        let snap = decode_filter_json(&json).unwrap();
        assert_eq!(snap.filter_ranges.get("x"), Some(&(0.0, 1.0)));
        assert_eq!(snap.selected_indices, vec![1, 2]);
        assert_eq!(snap.color_mode, "ParetoRank");
    }

    #[test]
    fn task2115_encode_decode_layout_json() {
        let config = LayoutConfig {
            rows: 2,
            cols: 3,
            cells: vec![GridCellSnapshot {
                row: 0,
                col: 0,
                content: Some("ParetoScatter2D".to_string()),
                col_span: 1,
                row_span: 1,
            }],
        };
        let json = encode_layout_json(&config).unwrap();
        let restored = decode_layout_json(&json).unwrap();
        assert_eq!(restored.rows, 2);
        assert_eq!(restored.cols, 3);
        assert_eq!(restored.cells.len(), 1);
        assert_eq!(
            restored.cells[0].content.as_deref(),
            Some("ParetoScatter2D")
        );
    }

    // --- TASK-2127 integration tests ---

    #[test]
    fn task2127_filter_json_file_roundtrip() {
        let mut ranges = HashMap::new();
        ranges.insert("p1".to_string(), (0.1, 0.9));
        ranges.insert("p2".to_string(), (-1.0, 1.0));
        let json = encode_filter_json(&ranges, &[0, 2, 4], "Default").unwrap();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &json).unwrap();
        let loaded_json = std::fs::read_to_string(tmp.path()).unwrap();
        let loaded = decode_filter_json(&loaded_json).unwrap();

        assert_eq!(loaded.filter_ranges.get("p1"), Some(&(0.1, 0.9)));
        assert_eq!(loaded.filter_ranges.get("p2"), Some(&(-1.0, 1.0)));
        assert_eq!(loaded.selected_indices, vec![0, 2, 4]);
        assert_eq!(loaded.color_mode, "Default");
    }

    #[test]
    fn task2127_layout_json_file_roundtrip() {
        let config = LayoutConfig {
            rows: 3,
            cols: 2,
            cells: vec![
                GridCellSnapshot {
                    row: 0,
                    col: 0,
                    content: Some("scatter".to_string()),
                    col_span: 1,
                    row_span: 1,
                },
                GridCellSnapshot {
                    row: 1,
                    col: 1,
                    content: None,
                    col_span: 2,
                    row_span: 1,
                },
            ],
        };
        let json = encode_layout_json(&config).unwrap();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &json).unwrap();
        let loaded_json = std::fs::read_to_string(tmp.path()).unwrap();
        let loaded = decode_layout_json(&loaded_json).unwrap();

        assert_eq!(loaded.rows, 3);
        assert_eq!(loaded.cols, 2);
        assert_eq!(loaded.cells.len(), 2);
        assert_eq!(loaded.cells[1].col_span, 2);
    }

    #[test]
    fn task2127_tdash_session_file_roundtrip() {
        let mut snap = make_snapshot();
        snap.tradeoff_weights = vec![0.4, 0.6];
        snap.pcp_axis_order = vec!["x1".to_string(), "x2".to_string()];
        snap.pcp_axis_visibility.insert("x1".to_string(), true);
        snap.pcp_axis_visibility.insert("x2".to_string(), false);
        snap.pinned_trials = vec![1, 3, 5];

        let json = serialize_session(&snap).unwrap();
        let tmp = tempfile::NamedTempFile::with_suffix(".tdash").unwrap();
        std::fs::write(tmp.path(), &json).unwrap();

        let loaded_json = std::fs::read_to_string(tmp.path()).unwrap();
        let loaded = deserialize_session(&loaded_json).unwrap();

        assert_eq!(loaded.tradeoff_weights, vec![0.4, 0.6]);
        assert_eq!(loaded.pcp_axis_order, vec!["x1", "x2"]);
        assert_eq!(loaded.pcp_axis_visibility.get("x2"), Some(&false));
        assert_eq!(loaded.pinned_trials, vec![1, 3, 5]);
    }

    #[test]
    fn task2127_missing_fields_use_defaults() {
        let old_json = r#"{"study_name": "legacy", "filter_ranges": {}, "selected_indices": [], "saved_at": ""}"#;
        let loaded = deserialize_session(old_json).unwrap();
        assert!(loaded.tradeoff_weights.is_empty());
        assert!(loaded.pcp_axis_visibility.is_empty());
        assert!(loaded.pcp_axis_order.is_empty());
        assert!(loaded.pinned_trials.is_empty());
    }

    // ── TASK-2231: ピン留め round-trip テスト ──────────────────

    #[test]
    fn session_round_trip_preserves_pinned_trials() {
        let mut snap = make_snapshot();
        snap.pinned_trials = vec![3, 7, 15];
        let json = serialize_session(&snap).unwrap();
        let restored = deserialize_session(&json).unwrap();
        assert_eq!(restored.pinned_trials, vec![3, 7, 15]);
    }
}
