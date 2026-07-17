//! Client that evaluates a single trial via rhino.compute's Grasshopper endpoint.
//!
//! Sends `POST /grasshopper` to a local rhino.compute instance (default
//! `http://localhost:6500`), assigns variable values to `RH_IN:*`, solves,
//! and extracts the `RH_OUT:*` values as the objective values. Requests are
//! stateless, so parallel workers map directly to concurrent requests
//! (concurrency is capped by a semaphore via `ComputeConfig.max_parallel`).
//!
//! Only plain HTTP (http://) is supported. Since rhino.compute is assumed to
//! run locally, TLS is not required (extend this if needed later, mirroring
//! how the RDB connection handles TLS).

use std::sync::{Condvar, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};

use super::compute_def::ComputeDefinition;

/// Connection settings for rhino.compute.
#[derive(Debug, Clone)]
pub struct ComputeConfig {
    /// e.g. `http://localhost:6500`
    pub server_url: String,
    /// rhino.compute's `RhinoComputeKey` (not sent if unset).
    pub api_key: Option<String>,
    /// Timeout in seconds for a single request (= one solve).
    pub timeout_secs: u64,
    /// Upper bound on the number of concurrent requests.
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

/// Evaluator for the real objective function. The implementation can be
/// swapped between Rhino.Compute (production) and a mock (for tests and for
/// verifying the runner in isolation).
pub trait GhEvaluator: Send + Sync {
    /// Takes variable values (in the same order as `GhProblem.variables`) and
    /// returns objective values (in the same order as `GhProblem.objectives`).
    fn evaluate(&self, values: &[f64]) -> Result<Vec<f64>, String>;
}

/// `GhEvaluator` implementation that evaluates via Rhino.Compute.
pub struct ComputeEvaluator {
    endpoint: String,
    api_key: Option<String>,
    /// Base64-encoded Compute definition (same for every request).
    algo: String,
    /// Local file path of the definition, sent as the request's `pointer`.
    /// See [`ComputeEvaluator::with_definition_pointer`].
    pointer: Option<String>,
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
            pointer: None,
            input_params: def.input_params.clone(),
            output_params: def.output_params.clone(),
            agent,
            semaphore: Semaphore::new(cfg.max_parallel.max(1)),
        }
    }

    /// Sends `path` as the request's `pointer` so compute loads (and caches) the
    /// definition from the local file instead of re-deserializing the base64
    /// `algo` on every request. This avoids compute's noisy binary-first
    /// deserialization attempt (which logs an exception per request for XML
    /// payloads) and skips redundant per-request parsing.
    ///
    /// Only valid when rhino.compute runs on the same machine as this process
    /// (e.g. the EXE launch mode). The base64 `algo` is still included as a
    /// fallback in case the file cannot be read.
    pub fn with_definition_pointer(mut self, path: impl Into<String>) -> Self {
        self.pointer = Some(path.into());
        self
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
            "pointer": self.pointer.as_deref(),
            "cachesolve": false,
            "values": inputs,
        })
    }
}

impl GhEvaluator for ComputeEvaluator {
    fn evaluate(&self, values: &[f64]) -> Result<Vec<f64>, String> {
        if values.len() != self.input_params.len() {
            return Err(format!(
                "Variable count mismatch (expected {}, got {})",
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
                    "Rhino.Compute returned an error (HTTP {code}): {detail}"
                ));
            }
            Err(e) => {
                return Err(format!(
                    "Cannot connect to Rhino.Compute ({}): {e}",
                    self.endpoint
                ))
            }
        };
        let text = response
            .into_string()
            .map_err(|e| format!("Failed to read the Rhino.Compute response: {e}"))?;
        let parsed: Value = serde_json::from_str(&text)
            .map_err(|e| format!("Rhino.Compute response is not JSON: {e}"))?;
        extract_outputs(&parsed, &self.output_params)
    }
}

/// Extracts RH_OUT values from a Compute response (GrasshopperEndpoint schema).
///
/// Response format:
/// `{"values": [{"ParamName": "RH_OUT:weight", "InnerTree": {"{0}": [{"type": "...", "data": <number or JSON string>}]}}]}`
fn extract_outputs(response: &Value, output_params: &[String]) -> Result<Vec<f64>, String> {
    let values = response
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(|| "Rhino.Compute response has no values field".to_string())?;

    let mut result = Vec::with_capacity(output_params.len());
    for name in output_params {
        let entry = values
            .iter()
            .find(|e| e.get("ParamName").and_then(Value::as_str) == Some(name.as_str()))
            .ok_or_else(|| {
                format!(
                    "The response does not contain {name}; check the definition's output wiring"
                )
            })?;
        let first_item = entry
            .get("InnerTree")
            .and_then(Value::as_object)
            .and_then(|tree| tree.values().next())
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .ok_or_else(|| format!("{name} is empty (the solve may have failed)"))?;
        let data = first_item
            .get("data")
            .ok_or_else(|| format!("{name} has no data field in the response"))?;
        let value = parse_data_number(data)
            .ok_or_else(|| format!("Cannot interpret the value of {name} as a number: {data}"))?;
        if !value.is_finite() {
            return Err(format!("The value of {name} is not finite: {value}"));
        }
        result.push(value);
    }
    Ok(result)
}

/// Interprets a Resthopper `data` field as a number.
/// Accepts a raw number, a numeric string (`"12.3"`), or a doubly-wrapped JSON string (`"\"12.3\""`).
fn parse_data_number(data: &Value) -> Option<f64> {
    match data {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let trimmed = s.trim();
            if let Ok(v) = trimmed.parse::<f64>() {
                return Some(v);
            }
            // Case where the value is doubly-wrapped as JSON, e.g. "\"12.3\""
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

/// Minimal semaphore that limits the number of concurrent requests (RAII guard style).
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
    fn request_body_includes_pointer_when_set() {
        let def = ComputeDefinition {
            ghx: "<Archive/>".to_string(),
            input_params: vec!["RH_IN:x".to_string()],
            output_params: vec!["RH_OUT:v".to_string()],
        };
        let cfg = ComputeConfig::default();
        let plain = ComputeEvaluator::new(&cfg, &def);
        assert_eq!(plain.request_body(&[1.0])["pointer"], Value::Null);
        let with_pointer =
            ComputeEvaluator::new(&cfg, &def).with_definition_pointer(r"C:\runs\model.compute.ghx");
        let body = with_pointer.request_body(&[1.0]);
        assert_eq!(body["pointer"], r"C:\runs\model.compute.ghx");
        // algo stays included as a fallback for when the file cannot be read
        assert!(body["algo"].as_str().unwrap().len() > 4);
    }

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

    /// Spins up a minimal HTTP server to verify the request/response round trip of `evaluate`.
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

        // Verify the contents of the request the server received
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
