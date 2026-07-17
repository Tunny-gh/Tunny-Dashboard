use serde_json::Value;

/// Optuna's distribution definition (`distribution` in the journal / `distribution_json` in the RDB).
///
/// The parameter value Optuna stores (`param_value_internal` in the journal / `param_value`
/// in the RDB) is, for Float and Int, **the external representation (the actual suggested value)
/// itself** cast to f64. `log` and `step` are part of the distribution's definition and play no
/// role in converting the stored value. The one exception is Categorical, whose stored value is
/// an index into the `choices` array.
#[derive(Debug)]
pub(crate) enum Distribution {
    Float { low: f64, high: f64 },
    Int { low: i64, high: i64 },
    Categorical { choices: Vec<Value> },
    Uniform,
}

impl Distribution {
    /// Parses the journal's distribution JSON (including the case where it's double-serialized as a string).
    pub(crate) fn from_json(json: &Value) -> Self {
        if let Some(serialized) = json.as_str() {
            if let Ok(parsed) = serde_json::from_str::<Value>(serialized) {
                return Distribution::from_json(&parsed);
            }
            return Distribution::Uniform;
        }

        let attrs = json.get("attributes").unwrap_or(json);

        match json
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("")
        {
            "FloatDistribution" => Distribution::Float {
                low: attrs
                    .get("low")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(f64::NAN),
                high: attrs
                    .get("high")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(f64::NAN),
            },
            "IntDistribution" => Distribution::Int {
                low: attrs
                    .get("low")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
                high: attrs
                    .get("high")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(i64::MIN),
            },
            "CategoricalDistribution" => Distribution::Categorical {
                choices: attrs
                    .get("choices")
                    .and_then(|value| value.as_array())
                    .cloned()
                    .unwrap_or_default(),
            },
            _ => Distribution::Uniform,
        }
    }

    /// Returns the declared range (low, high) in display units (Float / Int only).
    /// Used to make the surrogate optimization's search range match the true variable range.
    /// The low/high of a log-scale distribution are already in display (real-number) space, so they're returned as is.
    /// Returns None for Categorical / Uniform, or when the values are missing or degenerate (high <= low).
    pub(crate) fn bounds(&self) -> Option<(f64, f64)> {
        match self {
            Distribution::Float { low, high } => {
                if low.is_finite() && high.is_finite() && high > low {
                    Some((*low, *high))
                } else {
                    None
                }
            }
            Distribution::Int { low, high } => {
                if *high > *low {
                    Some((*low as f64, *high as f64))
                } else {
                    None
                }
            }
            Distribution::Categorical { .. } | Distribution::Uniform => None,
        }
    }

    /// Converts the stored value to a display value. Since Optuna stores the external
    /// representation itself as the value for Float and Int (no log transform even for log
    /// distributions, and the value is the real value even when a step is set), Float is the
    /// identity and Int only rounds to absorb floating-point error.
    /// Categorical is an index, so it's rounded and returned as-is (the label is resolved via `categorical_label`).
    pub(crate) fn to_display_f64(&self, internal: f64) -> f64 {
        match self {
            Distribution::Float { .. } | Distribution::Uniform => internal,
            Distribution::Int { .. } | Distribution::Categorical { .. } => internal.round(),
        }
    }

    /// Converts a Categorical's stored value (an index) into the corresponding choice's label string.
    pub(crate) fn categorical_label(&self, internal: f64) -> Option<String> {
        let Distribution::Categorical { choices } = self else {
            return None;
        };

        let idx = internal.round() as usize;
        choices.get(idx).map(|value| match value {
            Value::String(text) => text.clone(),
            Value::Number(number) => number.to_string(),
            Value::Bool(flag) => flag.to_string(),
            other => other.to_string(),
        })
    }
}
