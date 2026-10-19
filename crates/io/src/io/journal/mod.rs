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
