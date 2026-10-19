//! Python's JSON encoder allows bare non-finite literals, unlike strict JSON.

use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

/// Acquisition-only lexical evidence for Python's nonstandard numeric literals.
/// The marker key is absent from decoded object keys, including escaped spellings,
/// so user objects cannot collide. It never escapes into a Study record or storage.
pub(crate) struct Record {
    pub json: Value,
    extended: Option<Value>,
    marker: String,
}
impl Record {
    pub fn root(&self) -> &Value {
        self.extended.as_ref().unwrap_or(&self.json)
    }
    pub fn number(&self, value: &Value) -> Option<f64> {
        value.as_f64().or_else(|| {
            if self.marker.is_empty() {
                return None;
            }
            match value.get(&self.marker)?.as_str()? {
                "Infinity" => Some(f64::INFINITY),
                "-Infinity" => Some(f64::NEG_INFINITY),
                "NaN" => Some(f64::NAN),
                _ => None,
            }
        })
    }
    pub fn attribute(&self, value: &Value) -> tdashboard_study::AttributeValue {
        use tdashboard_study::{AttributeValue, Number};
        if !self.marker.is_empty() && value.get(&self.marker).is_some() {
            return AttributeValue::Number(Number(self.number(value).unwrap()));
        }
        match value {
            Value::Array(values) => {
                let values: Vec<_> = values.iter().map(|v| self.attribute(v)).collect();
                if values.iter().all(|v| matches!(v, AttributeValue::Json(_))) {
                    AttributeValue::Json(value.clone())
                } else {
                    AttributeValue::Array(values)
                }
            }
            Value::Object(values) => {
                let values: std::collections::BTreeMap<_, _> = values
                    .iter()
                    .map(|(k, v)| (k.clone(), self.attribute(v)))
                    .collect();
                if values
                    .values()
                    .all(|v| matches!(v, AttributeValue::Json(_)))
                {
                    AttributeValue::Json(value.clone())
                } else {
                    AttributeValue::Object(values)
                }
            }
            _ => AttributeValue::Json(value.clone()),
        }
    }
}
pub(crate) fn record(text: &str) -> Result<Record, serde_json::Error> {
    if let Ok(json) = serde_json::from_str(text) {
        return Ok(Record {
            json,
            extended: None,
            marker: String::new(),
        });
    }
    fn keys(value: &Value, names: &mut std::collections::HashSet<String>) {
        match value {
            Value::Object(values) => {
                for (key, value) in values {
                    names.insert(key.clone());
                    keys(value, names);
                }
            }
            Value::Array(values) => {
                for value in values {
                    keys(value, names);
                }
            }
            _ => (),
        }
    }
    let json = parse(text)?;
    let mut names = std::collections::HashSet::new();
    keys(&json, &mut names);
    let mut marker = "__tunny_nonfinite".to_string();
    while names.contains(&marker) {
        marker.push('_');
    }
    static TOKENS: OnceLock<Regex> = OnceLock::new();
    let tokens = TOKENS.get_or_init(|| Regex::new(r#""(?:\\.|[^"\\])*"|NaN|-?Infinity"#).unwrap());
    let rewritten = tokens.replace_all(text, |captures: &regex::Captures<'_>| {
        let token = &captures[0];
        if token.starts_with('"') {
            token.to_string()
        } else {
            format!("{{\"{marker}\":\"{token}\"}}")
        }
    });
    let extended: Value = serde_json::from_str(&rewritten)?;
    Ok(Record {
        json,
        extended: Some(extended),
        marker,
    })
}

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
