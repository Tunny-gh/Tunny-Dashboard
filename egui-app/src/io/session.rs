use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// セッションスナップショット（保存・復元用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub study_name: String,
    pub filter_ranges: HashMap<String, (f64, f64)>,
    pub selected_indices: Vec<u32>,
    pub saved_at: String,
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
            saved_at: "".to_string(), // In production, use a real timestamp
        }
    }
}

/// SessionSnapshot を JSON 文字列にシリアライズする
pub fn serialize_session(snapshot: &SessionSnapshot) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(snapshot)
}

/// JSON 文字列から SessionSnapshot をデシリアライズする
pub fn deserialize_session(json: &str) -> Result<SessionSnapshot, serde_json::Error> {
    serde_json::from_str(json)
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
}
