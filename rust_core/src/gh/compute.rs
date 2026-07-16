//! rhino.compute の Grasshopper エンドポイントで 1 試行を評価するクライアント。
//!
//! ローカルの rhino.compute（既定 `http://localhost:6500`）へ
//! `POST /grasshopper` を送り、`RH_IN:*` に変数値を割り当てて solve し、
//! `RH_OUT:*` の値を目的値として取り出す。リクエストはステートレスなので
//! 並列 worker = 並行リクエストで並列評価できる（同時実行数は
//! `ComputeConfig.max_parallel` のセマフォで制限）。
//!
//! HTTP は平文（http://）のみ対応。rhino.compute はローカル起動が前提のため
//! TLS は不要（RDB の TLS 対応と同様、必要になれば拡張する）。

use std::sync::{Condvar, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};

use super::compute_def::ComputeDefinition;

/// rhino.compute への接続設定。
#[derive(Debug, Clone)]
pub struct ComputeConfig {
    /// 例: `http://localhost:6500`
    pub server_url: String,
    /// rhino.compute の `RhinoComputeKey`（未設定なら送らない）
    pub api_key: Option<String>,
    /// 1 リクエスト（= 1 solve）のタイムアウト秒
    pub timeout_secs: u64,
    /// 同時リクエスト数の上限
    pub max_parallel: usize,
}

impl Default for ComputeConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:6500".to_string(),
            api_key: None,
            timeout_secs: 300,
            max_parallel: 4,
        }
    }
}

/// 実目的関数の評価器。実装は Rhino.Compute（本番）とモック（テスト・
/// ランナーの単体検証）を差し替えられる。
pub trait GhEvaluator: Send + Sync {
    /// 変数の実値（`GhProblem.variables` と同順）を受けて目的の実値
    /// （`GhProblem.objectives` と同順)を返す。
    fn evaluate(&self, values: &[f64]) -> Result<Vec<f64>, String>;
}

/// Rhino.Compute で評価する `GhEvaluator` 実装。
pub struct ComputeEvaluator {
    endpoint: String,
    api_key: Option<String>,
    /// base64 済み Compute 用定義（リクエストごとに同一）
    algo: String,
    input_params: Vec<String>,
    output_params: Vec<String>,
    agent: ureq::Agent,
    semaphore: Semaphore,
}

impl ComputeEvaluator {
    pub fn new(cfg: &ComputeConfig, def: &ComputeDefinition) -> Self {
        let endpoint = format!("{}/grasshopper", cfg.server_url.trim_end_matches('/'));
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(cfg.timeout_secs.max(1)))
            .build();
        Self {
            endpoint,
            api_key: cfg.api_key.clone(),
            algo: base64::engine::general_purpose::STANDARD.encode(def.ghx.as_bytes()),
            input_params: def.input_params.clone(),
            output_params: def.output_params.clone(),
            agent,
            semaphore: Semaphore::new(cfg.max_parallel.max(1)),
        }
    }

    fn request_body(&self, values: &[f64]) -> Value {
        let inputs: Vec<Value> = self
            .input_params
            .iter()
            .zip(values)
            .map(|(name, v)| {
                json!({
                    "ParamName": name,
                    "InnerTree": {
                        "0": [ { "type": "System.Double", "data": v.to_string() } ]
                    }
                })
            })
            .collect();
        json!({
            "algo": self.algo,
            "pointer": null,
            "cachesolve": false,
            "values": inputs,
        })
    }
}

impl GhEvaluator for ComputeEvaluator {
    fn evaluate(&self, values: &[f64]) -> Result<Vec<f64>, String> {
        if values.len() != self.input_params.len() {
            return Err(format!(
                "変数の数が一致しません（期待 {}、実際 {}）",
                self.input_params.len(),
                values.len()
            ));
        }
        let body = self.request_body(values).to_string();

        let _permit = self.semaphore.acquire();
        let mut req = self
            .agent
            .post(&self.endpoint)
            .set("Content-Type", "application/json");
        if let Some(key) = &self.api_key {
            req = req.set("RhinoComputeKey", key);
        }
        let response = match req.send_string(&body) {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                let detail = resp.into_string().unwrap_or_default();
                let detail = detail.chars().take(500).collect::<String>();
                return Err(format!(
                    "Rhino.Compute がエラーを返しました (HTTP {code}): {detail}"
                ));
            }
            Err(e) => {
                return Err(format!(
                    "Rhino.Compute に接続できません（{}）: {e}",
                    self.endpoint
                ))
            }
        };
        let text = response
            .into_string()
            .map_err(|e| format!("Rhino.Compute 応答の読み取りに失敗: {e}"))?;
        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| format!("Rhino.Compute 応答が JSON ではありません: {e}"))?;
        extract_outputs(&parsed, &self.output_params)
    }
}

/// Compute 応答（GrasshopperEndpoint の schema）から RH_OUT の値を取り出す。
///
/// 応答形式:
/// `{"values": [{"ParamName": "RH_OUT:weight", "InnerTree": {"{0}": [{"type": "...", "data": <数値または JSON 文字列>}]}}]}`
fn extract_outputs(response: &Value, output_params: &[String]) -> Result<Vec<f64>, String> {
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(|| "Rhino.Compute 応答に values がありません".to_string())?;

    let mut result = Vec::with_capacity(output_params.len());
    for name in output_params {
        let entry = values
            .iter()
            .find(|e| e.get("ParamName").and_then(Value::as_str) == Some(name.as_str()))
            .ok_or_else(|| {
                format!("応答に {name} が含まれていません。定義の出力接続を確認してください")
            })?;
        let first_item = entry
            .get("InnerTree")
            .and_then(Value::as_object)
            .and_then(|tree| tree.values().next())
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .ok_or_else(|| format!("{name} の値が空です（solve が失敗している可能性）"))?;
        let data = first_item
            .get("data")
            .ok_or_else(|| format!("{name} の応答に data がありません"))?;
        let value = parse_data_number(data)
            .ok_or_else(|| format!("{name} の値を数値として解釈できません: {data}"))?;
        if !value.is_finite() {
            return Err(format!("{name} の値が有限ではありません: {value}"));
        }
        result.push(value);
    }
    Ok(result)
}

/// Resthopper の data フィールドを数値として解釈する。
/// 数値そのもの、数値の文字列（"12.3"）、JSON 文字列（"\"12.3\""）を受ける。
fn parse_data_number(data: &Value) -> Option<f64> {
    match data {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let trimmed = s.trim();
            if let Ok(v) = trimmed.parse::<f64>() {
                return Some(v);
            }
            // "\"12.3\"" のように JSON として二重に包まれている場合
            match serde_json::from_str::<Value>(trimmed) {
                Ok(Value::Number(n)) => n.as_f64(),
                Ok(Value::String(inner)) => inner.trim().parse().ok(),
                Ok(Value::Bool(b)) => Some(if b { 1.0 } else { 0.0 }),
                _ => None,
            }
        }
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// 同時リクエスト数を制限する最小のセマフォ（RAII ガード方式）。
struct Semaphore {
    permits: Mutex<usize>,
    cv: Condvar,
}

struct SemaphoreGuard<'a>(&'a Semaphore);

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) -> SemaphoreGuard<'_> {
        let mut permits = self.permits.lock().unwrap_or_else(|e| e.into_inner());
        while *permits == 0 {
            permits = self.cv.wait(permits).unwrap_or_else(|e| e.into_inner());
        }
        *permits -= 1;
        SemaphoreGuard(self)
    }
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        let mut permits = self.0.permits.lock().unwrap_or_else(|e| e.into_inner());
        *permits += 1;
        self.0.cv.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    #[test]
    fn extracts_outputs_from_response() {
        let response: Value = serde_json::from_str(
            r#"{
              "pointer": null,
              "values": [
                {"ParamName": "RH_OUT:disp", "InnerTree": {"{0;0}": [{"type": "System.Double", "data": "4.5"}]}},
                {"ParamName": "RH_OUT:weight", "InnerTree": {"{0}": [{"type": "System.Double", "data": 12.3}]}}
              ]
            }"#,
        )
        .unwrap();
        let outputs = vec!["RH_OUT:weight".to_string(), "RH_OUT:disp".to_string()];
        assert_eq!(
            extract_outputs(&response, &outputs).unwrap(),
            vec![12.3, 4.5]
        );
    }

    #[test]
    fn missing_output_is_reported() {
        let response: Value = serde_json::from_str(r#"{"values": []}"#).unwrap();
        let err = extract_outputs(&response, &["RH_OUT:weight".to_string()]).unwrap_err();
        assert!(err.contains("RH_OUT:weight"), "unexpected: {err}");
    }

    #[test]
    fn parses_various_data_encodings() {
        assert_eq!(parse_data_number(&json!(1.5)), Some(1.5));
        assert_eq!(parse_data_number(&json!("2.5")), Some(2.5));
        assert_eq!(parse_data_number(&json!("\"3.5\"")), Some(3.5));
        assert_eq!(parse_data_number(&json!(true)), Some(1.0));
        assert_eq!(parse_data_number(&json!("abc")), None);
    }

    /// 最小の HTTP サーバーを立てて evaluate のリクエスト/レスポンス往復を検証する。
    #[test]
    fn evaluate_round_trip_against_fake_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let server = std::thread::spawn(move || -> String {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let line = line.trim();
                if line.is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = v.trim().parse().unwrap();
                }
            }
            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body).unwrap();
            let body = String::from_utf8(body).unwrap();

            let reply = r#"{"values": [{"ParamName": "RH_OUT:weight", "InnerTree": {"{0}": [{"type": "System.Double", "data": "42.5"}]}}]}"#;
            let mut stream = reader.into_inner();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                reply.len(),
                reply
            )
            .unwrap();
            body
        });

        let def = ComputeDefinition {
            ghx: "<Archive/>".to_string(),
            input_params: vec!["RH_IN:span".to_string()],
            output_params: vec!["RH_OUT:weight".to_string()],
        };
        let cfg = ComputeConfig {
            server_url: format!("http://127.0.0.1:{port}"),
            timeout_secs: 10,
            ..Default::default()
        };
        let evaluator = ComputeEvaluator::new(&cfg, &def);
        let result = evaluator.evaluate(&[5.5]).unwrap();
        assert_eq!(result, vec![42.5]);

        // サーバーが受け取ったリクエストの中身を検証
        let request_body = server.join().unwrap();
        let req: Value = serde_json::from_str(&request_body).unwrap();
        assert_eq!(req["values"][0]["ParamName"], "RH_IN:span");
        assert_eq!(req["values"][0]["InnerTree"]["0"][0]["data"], "5.5");
        assert!(req["algo"].as_str().unwrap().len() > 4);
    }

    #[test]
    fn semaphore_limits_concurrency() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        let sem = Arc::new(Semaphore::new(2));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let (sem, active, peak) = (sem.clone(), active.clone(), peak.clone());
                std::thread::spawn(move || {
                    let _permit = sem.acquire();
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    active.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert!(peak.load(Ordering::SeqCst) <= 2);
    }
}
