use std::collections::HashMap;

use serde_json::Value;

/// Record an Optuna trial user attribute, replacing an earlier value of any
/// JSON type under the same key. Numeric/text maps remain available to analysis.
pub(crate) fn insert_user_attr(
    key: &str,
    value: Value,
    numeric: &mut HashMap<String, f64>,
    text: &mut HashMap<String, String>,
    json: &mut HashMap<String, Value>,
) {
    numeric.remove(key);
    text.remove(key);
    if let Some(number) = value.as_f64() {
        numeric.insert(key.to_string(), number);
    } else if let Some(string) = value.as_str() {
        text.insert(key.to_string(), string.to_string());
    }
    json.insert(key.to_string(), value);
}
