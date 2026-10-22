pub mod parser;
pub mod writer;

pub fn count_created_trials(data: &[u8]) -> u32 {
    String::from_utf8_lossy(data)
        .lines()
        .filter(|line| line_u32_field(line.trim_start(), "op_code") == Some(4))
        .count() as u32
}
pub fn line_u32_field(line: &str, key: &str) -> Option<u32> {
    let bytes = line.as_bytes();
    for (start, _) in line.match_indices(key) {
        if start == 0 || bytes[start - 1] != b'"' {
            continue;
        }
        let end = start + key.len();
        if bytes.get(end) != Some(&b'"') {
            continue;
        }
        let Some(rest) = line[end + 1..].trim_start().strip_prefix(':') else {
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
pub(crate) fn constraints_from_system_attr(json: &serde_json::Value) -> Option<Vec<f64>> {
    let values = json.get("system_attr")?.get("constraints")?;
    Some(
        values
            .as_array()
            .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect())
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::{count_created_trials, line_u32_field};

    #[test]
    fn counts_all_op4_across_studies_and_states() {
        let data = [
            r#"{"op_code":0,"study_name":"a","directions":[1]}"#,
            r#"{"op_code":0,"study_name":"b","directions":[1]}"#,
            r#"{"op_code":4,"study_id":0}"#,
            r#"{"op_code":6,"trial_id":0,"state":1,"values":[1.0]}"#,
            r#"{"op_code":4,"study_id":0}"#,
            r#"{"op_code":6,"trial_id":1,"state":2}"#,
            r#"{"op_code":4,"study_id":1}"#,
            r#"{"op_code":4,"study_id":1}"#,
            r#"{"op_code":6,"trial_id":3,"state":3}"#,
        ]
        .join("\n");
        assert_eq!(count_created_trials(data.as_bytes()), 4);
    }

    #[test]
    fn counts_zero_for_empty_or_trialless_journal() {
        assert_eq!(count_created_trials(b""), 0);
        assert_eq!(
            count_created_trials(br#"{"op_code":0,"study_name":"a","directions":[1]}"#),
            0
        );
    }

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
        assert_eq!(line_u32_field(r#"{"study_idx":9}"#, "study_id"), None);
        assert_eq!(line_u32_field(r#"{study_id:9}"#, "study_id"), None);
    }

    #[test]
    fn skips_lookalike_and_finds_real_key() {
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
        assert_eq!(
            line_u32_field(r#"{"trial_id":4294967296}"#, "trial_id"),
            None
        );
    }
}
