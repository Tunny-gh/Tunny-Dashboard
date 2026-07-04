use serde_json::Value;

/// Optuna の distribution 定義（journal の `distribution` / RDB の `distribution_json`）。
///
/// Optuna が格納するパラメータ値（journal の `param_value_internal` / RDB の
/// `param_value`）は、Float・Int では**外部表現（実際の提案値）そのもの**を f64 に
/// したものである。log や step は分布の定義情報であり、格納値の変換には関与しない。
/// 唯一の例外が Categorical で、`choices` 配列へのインデックスが格納される。
#[derive(Debug)]
pub(crate) enum Distribution {
    Float { low: f64, high: f64 },
    Int { low: i64, high: i64 },
    Categorical { choices: Vec<Value> },
    Uniform,
}

impl Distribution {
    /// Documentation.
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

    /// Documentation.
    /// 宣言レンジ (low, high) を表示単位で返す（Float / Int のみ）。
    /// サロゲート最適化の探索範囲を真の変数範囲に一致させるために使う。
    /// log スケールの low/high も表示（実数）空間の値なのでそのまま返す。
    /// Categorical / Uniform、または値が欠落・退化（high ≤ low）の場合は None。
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

    /// 格納値を表示値へ変換する。Optuna は Float・Int の格納値として外部表現
    /// そのものを持つ（log 分布でも log 変換されず、step があっても値は実値）ため、
    /// Float は恒等、Int は浮動小数点誤差を吸収する丸めのみを行う。
    /// Categorical はインデックスなので丸めて返す（ラベルは `categorical_label` で解決）。
    pub(crate) fn to_display_f64(&self, internal: f64) -> f64 {
        match self {
            Distribution::Float { .. } | Distribution::Uniform => internal,
            Distribution::Int { .. } | Distribution::Categorical { .. } => internal.round(),
        }
    }

    /// Documentation.
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
