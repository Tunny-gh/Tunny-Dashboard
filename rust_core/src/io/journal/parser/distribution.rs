use serde_json::Value;

/// Documentation.
#[derive(Debug)]
pub(super) enum Distribution {
    Float { log: bool },
    Int { low: i64, step: i64, log: bool },
    Categorical { choices: Vec<Value> },
    Uniform,
}

impl Distribution {
    /// Documentation.
    pub(super) fn from_json(json: &Value) -> Self {
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
                log: attrs
                    .get("log")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
            },
            "IntDistribution" => Distribution::Int {
                low: attrs
                    .get("low")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
                step: attrs
                    .get("step")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(1)
                    .max(1),
                log: attrs
                    .get("log")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
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

    /// Documentation.
    pub(super) fn to_display_f64(&self, internal: f64) -> f64 {
        match self {
            Distribution::Float { log } => {
                if *log {
                    internal.exp()
                } else {
                    internal
                }
            }
            Distribution::Int { low, step, log } => {
                let rounded = if *log {
                    internal.exp().round() as i64
                } else {
                    internal.round() as i64
                };
                (*low + rounded * *step) as f64
            }
            Distribution::Categorical { .. } => internal.round(),
            Distribution::Uniform => internal,
        }
    }

    /// Documentation.
    pub(super) fn categorical_label(&self, internal: f64) -> Option<String> {
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
