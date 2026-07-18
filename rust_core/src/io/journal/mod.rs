//! Reader for Optuna JournalStorage (JSON Lines).
//!
//! `parser` handles bulk/on-demand parsing, while `live_update` handles polling diff parsing.

pub mod live_update;
pub mod parser;
pub mod writer;

/// Quickly extract the value of `"key": <non-negative integer>` from a line without full JSON parsing.
///
/// Used for string-level filtering of op_code / study_id / trial_id
/// (shared by `parser`'s Phase 1/2 scans and `live_update`'s counter seed computation).
/// Only accepts keys in the exact form `"key"` (surrounded by matching double quotes),
/// and scans without per-line heap allocation such as `format!`. Returns `None` if the
/// value doesn't fit in a u32 or isn't numeric.
pub(crate) fn line_u32_field(line: &str, key: &str) -> Option<u32> {
    let bytes = line.as_bytes();
    for (key_start, _) in line.match_indices(key) {
        // Only accept an exact-match key surrounded by double quotes
        // (so "study_id" doesn't accidentally match a substring like "study_idx").
        if key_start == 0 || bytes[key_start - 1] != b'"' {
            continue;
        }
        let after_key = key_start + key.len();
        if bytes.get(after_key) != Some(&b'"') {
            continue;
        }
        let rest = line[after_key + 1..].trim_start();
        let Some(rest) = rest.strip_prefix(':') else {
            continue;
        };
        let digits = rest.trim_start();
        let end = digits
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(digits.len());
        if end == 0 {
            return None;
        }
        return digits[..end].parse().ok();
    }
    None
}

/// Splits op8 (`SET_TRIAL_USER_ATTR`) values into numeric / string attribute
/// maps: JSON numbers become numeric attrs, JSON strings become string attrs,
/// anything else is dropped. Shared by the full parser and the live-diff parser
/// so both classify a given journal record into the same column type.
pub(crate) fn classify_user_attrs(
    attrs: &serde_json::Map<String, serde_json::Value>,
    numeric: &mut std::collections::HashMap<String, f64>,
    string: &mut std::collections::HashMap<String, String>,
) {
    for (key, value) in attrs {
        if let Some(number) = value.as_f64() {
            numeric.insert(key.clone(), number);
        } else if let Some(text) = value.as_str() {
            string.insert(key.clone(), text.to_string());
        }
    }
}

/// Extracts the `"constraints"` array from an op9 (`SET_TRIAL_SYSTEM_ATTR`)
/// record's `system_attr` object (non-numeric entries dropped). Shared by the
/// full parser and the live-diff parser.
pub(crate) fn constraints_from_system_attr(json: &serde_json::Value) -> Option<Vec<f64>> {
    json.get("system_attr")?
        .get("constraints")?
        .as_array()
        .map(|values| values.iter().filter_map(|v| v.as_f64()).collect())
}

#[cfg(test)]
mod tests {
    use super::line_u32_field;

    #[test]
    fn extracts_leading_field() {
        assert_eq!(
            line_u32_field(r#"{"op_code":4,"study_id":2}"#, "op_code"),
            Some(4)
        );
    }

    #[test]
    fn extracts_middle_field() {
        assert_eq!(
            line_u32_field(r#"{"op_code":4,"study_id":2}"#, "study_id"),
            Some(2)
        );
    }

    #[test]
    fn allows_whitespace_around_colon() {
        assert_eq!(line_u32_field(r#"{"op_code" : 7}"#, "op_code"), Some(7));
    }

    #[test]
    fn rejects_partial_key_match() {
        // "study_id" does not match "study_idx" (closing quote required).
        assert_eq!(line_u32_field(r#"{"study_idx":9}"#, "study_id"), None);
        // A bare key without a preceding quote doesn't match either.
        assert_eq!(line_u32_field(r#"{study_id:9}"#, "study_id"), None);
    }

    #[test]
    fn skips_lookalike_and_finds_real_key() {
        // Skip a partial-match key and pick up the subsequent exact-match key.
        assert_eq!(
            line_u32_field(r#"{"study_idx":9,"study_id":3}"#, "study_id"),
            Some(3)
        );
    }

    #[test]
    fn rejects_non_numeric_and_missing() {
        assert_eq!(line_u32_field(r#"{"op_code":"x"}"#, "op_code"), None);
        assert_eq!(line_u32_field(r#"{"trial_id":1}"#, "op_code"), None);
        assert_eq!(line_u32_field("", "op_code"), None);
    }

    #[test]
    fn rejects_out_of_range_value() {
        // A value exceeding u32 range is None (never silently truncated).
        assert_eq!(
            line_u32_field(r#"{"trial_id":4294967296}"#, "trial_id"),
            None
        );
    }
}
