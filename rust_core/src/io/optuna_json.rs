//! Python's JSON encoder allows bare non-finite literals, unlike strict JSON.

use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

/// Accept Python's three non-finite literals as null (serde_json's own
/// non-finite representation). Constraint extraction maps these positions to
/// NaN. Quoted strings, including escaped quotes and nested JSON text, are
/// consumed intact by the regex and are never rewritten. All other syntax
/// remains subject to serde_json's strict validation.
pub(crate) fn parse(text: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(text).or_else(|_| {
        static TOKENS: OnceLock<Regex> = OnceLock::new();
        let tokens =
            TOKENS.get_or_init(|| Regex::new(r#""(?:\\.|[^"\\])*"|NaN|-?Infinity"#).unwrap());
        let normalized = tokens.replace_all(text, |captures: &regex::Captures<'_>| {
            let token = &captures[0];
            if token.starts_with('"') {
                token.to_string()
            } else {
                "null".to_string()
            }
        });
        serde_json::from_str(&normalized)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonfinite_literals_keep_positions_without_rewriting_strings_or_finite_data() {
        let parsed = parse(r#"{"constraints":[NaN,Infinity,-Infinity,0.5],"text":"NaN Infinity -Infinity \"Infinity\"","nested":"{\"x\":NaN}","other":[1,true,null]}"#).unwrap();
        assert_eq!(
            parsed["constraints"],
            serde_json::json!([null, null, null, 0.5])
        );
        assert_eq!(parsed["text"], "NaN Infinity -Infinity \"Infinity\"");
        assert_eq!(parsed["nested"], "{\"x\":NaN}");
        assert_eq!(parsed["other"], serde_json::json!([1, true, null]));
        for invalid in [
            "[InfinitySuffix]",
            "[notNaN]",
            "[+Infinity]",
            "{'x':NaN}",
            "[NaN,]",
        ] {
            assert!(parse(invalid).is_err(), "must reject {invalid}");
        }
    }
}
