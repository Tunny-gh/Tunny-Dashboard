//! Parameter substitution: turning a trial's `(name, value)` pairs into the
//! concrete inputs a command receives (a rendered template string, CLI args,
//! environment variables, or a JSON stdin payload).

use std::collections::HashMap;

/// Renders `{name}` placeholders in `template` using `values` (name → value).
///
/// Integral values render without a trailing `.0` (`3`, not `3.0`) so solvers
/// that parse integers see the expected token; other values use the shortest
/// round-trippable form. `{{` / `}}` are literal braces. An unknown
/// placeholder is an error (fail loud rather than silently emit an empty
/// value).
pub fn render_template(template: &str, values: &HashMap<&str, f64>) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        match c {
            '{' => {
                if matches!(chars.peek(), Some((_, '{'))) {
                    chars.next();
                    out.push('{');
                    continue;
                }
                let mut name = String::new();
                let mut closed = false;
                for (_, nc) in chars.by_ref() {
                    if nc == '}' {
                        closed = true;
                        break;
                    }
                    name.push(nc);
                }
                if !closed {
                    return Err(format!("unclosed placeholder in template: \"{template}\""));
                }
                let value = values
                    .get(name.as_str())
                    .ok_or_else(|| format!("unknown placeholder \"{{{name}}}\" in template"))?;
                out.push_str(&format_value(*value));
            }
            '}' => {
                if matches!(chars.peek(), Some((_, '}'))) {
                    chars.next();
                    out.push('}');
                } else {
                    return Err(format!("unmatched '}}' in template: \"{template}\""));
                }
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

/// Formats a parameter value: integral values as integers, others with the
/// shortest round-trippable representation.
pub fn format_value(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 && value.abs() < 9.007_199_254_740_992e15 {
        format!("{}", value as i64)
    } else {
        // Rust's default float formatting is already shortest-round-trippable.
        format!("{value}")
    }
}

/// Builds CLI arguments by expanding `arg_template` (with `{name}` and
/// `{value}`) once per parameter, in `param_names` order. Each expansion is
/// split on ASCII whitespace into separate argv entries, so a template like
/// `"--{name} {value}"` yields two args per parameter.
pub fn build_args(
    arg_template: &str,
    param_names: &[String],
    values: &[f64],
) -> Result<Vec<String>, String> {
    if param_names.len() != values.len() {
        return Err(format!(
            "parameter/value count mismatch ({} names, {} values)",
            param_names.len(),
            values.len()
        ));
    }
    let mut args = Vec::new();
    for (name, value) in param_names.iter().zip(values) {
        let mut map: HashMap<&str, &str> = HashMap::new();
        let value_str = format_value(*value);
        map.insert("name", name.as_str());
        map.insert("value", value_str.as_str());
        let expanded = expand_named(arg_template, &map)?;
        args.extend(expanded.split_whitespace().map(str::to_string));
    }
    Ok(args)
}

/// Builds `(name, value_string)` environment variable pairs.
pub fn build_env(param_names: &[String], values: &[f64]) -> Result<Vec<(String, String)>, String> {
    if param_names.len() != values.len() {
        return Err(format!(
            "parameter/value count mismatch ({} names, {} values)",
            param_names.len(),
            values.len()
        ));
    }
    Ok(param_names
        .iter()
        .zip(values)
        .map(|(name, value)| (name.clone(), format_value(*value)))
        .collect())
}

/// Builds a JSON object `{name: value}` (numbers, integral values as integers).
pub fn build_json_stdin(param_names: &[String], values: &[f64]) -> Result<String, String> {
    if param_names.len() != values.len() {
        return Err(format!(
            "parameter/value count mismatch ({} names, {} values)",
            param_names.len(),
            values.len()
        ));
    }
    let mut map = serde_json::Map::new();
    for (name, value) in param_names.iter().zip(values) {
        let json = if value.fract() == 0.0 && value.abs() < 9.007_199_254_740_992e15 {
            serde_json::Value::from(*value as i64)
        } else {
            serde_json::Value::from(*value)
        };
        map.insert(name.clone(), json);
    }
    serde_json::to_string(&serde_json::Value::Object(map))
        .map_err(|e| format!("failed to build JSON stdin: {e}"))
}

/// Expands `{key}` placeholders in `template` from a string map (used for arg
/// templates, where `{name}` and `{value}` are the keys). `{{`/`}}` escape.
fn expand_named(template: &str, values: &HashMap<&str, &str>) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        match c {
            '{' => {
                if matches!(chars.peek(), Some((_, '{'))) {
                    chars.next();
                    out.push('{');
                    continue;
                }
                let mut key = String::new();
                let mut closed = false;
                for (_, nc) in chars.by_ref() {
                    if nc == '}' {
                        closed = true;
                        break;
                    }
                    key.push(nc);
                }
                if !closed {
                    return Err(format!("unclosed placeholder in \"{template}\""));
                }
                let value = values
                    .get(key.as_str())
                    .ok_or_else(|| format!("unknown placeholder \"{{{key}}}\""))?;
                out.push_str(value);
            }
            '}' => {
                if matches!(chars.peek(), Some((_, '}'))) {
                    chars.next();
                    out.push('}');
                } else {
                    return Err(format!("unmatched '}}' in \"{template}\""));
                }
            }
            _ => out.push(c),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&'static str, f64)]) -> HashMap<&'static str, f64> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn render_template_substitutes_and_formats() {
        let values = map(&[("span", 5.5), ("count", 3.0)]);
        assert_eq!(
            render_template("span={span}\ncount={count}", &values).unwrap(),
            "span=5.5\ncount=3"
        );
    }

    #[test]
    fn render_template_handles_escaped_braces() {
        let values = map(&[("x", 1.0)]);
        assert_eq!(
            render_template("{{literal}} {x}", &values).unwrap(),
            "{literal} 1"
        );
    }

    #[test]
    fn render_template_rejects_unknown_and_unclosed() {
        let values = map(&[("x", 1.0)]);
        assert!(render_template("{y}", &values)
            .unwrap_err()
            .contains("unknown"));
        assert!(render_template("{x", &values)
            .unwrap_err()
            .contains("unclosed"));
    }

    #[test]
    fn build_args_expands_per_parameter() {
        let names = vec!["a".to_string(), "b".to_string()];
        let args = build_args("--{name}={value}", &names, &[1.0, 2.5]).unwrap();
        assert_eq!(args, vec!["--a=1", "--b=2.5"]);

        // Whitespace in the template splits into separate argv entries.
        let args = build_args("--{name} {value}", &names, &[1.0, 2.0]).unwrap();
        assert_eq!(args, vec!["--a", "1", "--b", "2"]);
    }

    #[test]
    fn build_env_pairs_names_and_values() {
        let names = vec!["A".to_string(), "B".to_string()];
        assert_eq!(
            build_env(&names, &[1.0, 2.5]).unwrap(),
            vec![
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "2.5".to_string())
            ]
        );
    }

    #[test]
    fn build_json_stdin_uses_integers_for_integral_values() {
        let names = vec!["x".to_string(), "y".to_string()];
        let json = build_json_stdin(&names, &[3.0, 1.5]).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["x"], serde_json::json!(3));
        assert_eq!(parsed["y"], serde_json::json!(1.5));
    }

    #[test]
    fn count_mismatch_is_an_error() {
        let names = vec!["a".to_string()];
        assert!(build_args("--{name}", &names, &[1.0, 2.0]).is_err());
        assert!(build_env(&names, &[]).is_err());
        assert!(build_json_stdin(&names, &[1.0, 2.0]).is_err());
    }
}
