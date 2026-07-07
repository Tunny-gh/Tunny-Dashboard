//! Optuna JournalStorage（JSON Lines）の読み取り。
//!
//! `parser` が一括/オンデマンド解析、`live_update` がポーリング差分解析を担う。

pub mod live_update;
pub mod parser;

/// JSON パースを行わずに、行から `"key": <非負整数>` の値を高速抽出する。
///
/// op_code / study_id / trial_id の string-level フィルタリング用
/// （`parser` の Phase 1/2 スキャンと `live_update` のカウンタ seed 計算で共用）。
/// キーは `"key"` の形（前後がダブルクォートの完全一致）のみ受理し、行毎の
/// `format!` 等のヒープ割り当てなしで走査する。値が u32 に収まらない・数値でない
/// 場合は `None`。
pub(crate) fn line_u32_field(line: &str, key: &str) -> Option<u32> {
    let bytes = line.as_bytes();
    for (key_start, _) in line.match_indices(key) {
        // 前後をダブルクォートで挟まれた完全一致キーのみ受理する
        // （"study_id" が "study_idx" 等の部分文字列に誤マッチしないように）。
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

#[cfg(test)]
mod tests {
    use super::line_u32_field;

    #[test]
    fn extracts_leading_field() {
        assert_eq!(line_u32_field(r#"{"op_code":4,"study_id":2}"#, "op_code"), Some(4));
    }

    #[test]
    fn extracts_middle_field() {
        assert_eq!(line_u32_field(r#"{"op_code":4,"study_id":2}"#, "study_id"), Some(2));
    }

    #[test]
    fn allows_whitespace_around_colon() {
        assert_eq!(line_u32_field(r#"{"op_code" : 7}"#, "op_code"), Some(7));
    }

    #[test]
    fn rejects_partial_key_match() {
        // "study_id" は "study_idx" にマッチしない（閉じクォート必須）。
        assert_eq!(line_u32_field(r#"{"study_idx":9}"#, "study_id"), None);
        // 前にクォートが無い裸のキーもマッチしない。
        assert_eq!(line_u32_field(r#"{study_id:9}"#, "study_id"), None);
    }

    #[test]
    fn skips_lookalike_and_finds_real_key() {
        // 部分一致のキーを読み飛ばし、後続の完全一致キーを拾う。
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
        // u32 を超える値は None（黙って切り捨てない）。
        assert_eq!(line_u32_field(r#"{"trial_id":4294967296}"#, "trial_id"), None);
    }
}
