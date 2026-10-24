//! End-to-end integration test for tunny-mcp.
//!
//! Launches the real binary (`env!("CARGO_BIN_EXE_tunny-mcp")`) as a child process, feeds
//! newline-delimited JSON-RPC messages into stdin, and checks the stdout responses.
//! Doesn't depend on the network or external files; only reads an Optuna journal file
//! fixture placed in a temporary directory (deleted on completion).

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// A minimal Optuna journal fixture.
///
/// Creates study "demo_study" (2 objectives: minimize obj0, maximize obj1) with 3 COMPLETE
/// trials, each having param "x" (FloatDistribution [0.0, 1.0]).
///
/// op_code: 0=create_study, 4=create_trial, 5=set_trial_param,
/// 6=set_trial_state_values (state=1 is COMPLETE). trial_id is auto-numbered by the order in
/// which create_trial appears throughout the file, so the subsequent op 5/6 entries reference
/// 0, 1, 2 in that order.
fn journal_fixture() -> String {
    concat!(
        "{\"op_code\":0,\"worker_id\":\"w\",\"study_name\":\"demo_study\",\"directions\":[1,2]}\n",
        // --- trial 0 ---
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:00.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":0,\"param_name\":\"x\",\"param_value_internal\":0.1,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":0,\"state\":1,\"values\":[1.0,5.0],\"datetime_complete\":\"2024-01-01T00:00:01.000000\"}\n",
        // --- trial 1 ---
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:02.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":1,\"param_name\":\"x\",\"param_value_internal\":0.5,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":1,\"state\":1,\"values\":[0.5,8.0],\"datetime_complete\":\"2024-01-01T00:00:03.000000\"}\n",
        // --- trial 2 ---
        "{\"op_code\":4,\"worker_id\":\"w\",\"study_id\":0,\"datetime_start\":\"2024-01-01T00:00:04.000000\"}\n",
        "{\"op_code\":5,\"worker_id\":\"w\",\"trial_id\":2,\"param_name\":\"x\",\"param_value_internal\":0.9,\"distribution\":{\"name\":\"FloatDistribution\",\"low\":0.0,\"high\":1.0}}\n",
        "{\"op_code\":6,\"worker_id\":\"w\",\"trial_id\":2,\"state\":1,\"values\":[2.0,3.0],\"datetime_complete\":\"2024-01-01T00:00:05.000000\"}\n",
    )
    .to_string()
}

/// Creates a unique temporary directory for exclusive use by the test.
fn make_temp_dir() -> std::path::PathBuf {
    let unique = format!(
        "tunny-mcp-it-{}-{:?}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        std::thread::current().id(),
    );
    let dir = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn req(id: i64, method: &str, params: Value) -> String {
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    }))
    .unwrap()
}

fn notif(method: &str) -> String {
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
    }))
    .unwrap()
}

fn stdio(requests: &[String]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tunny-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for request in requests {
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout must contain only JSON-RPC"))
        .collect()
}

fn check_constrained_storage(storage: &std::path::Path) {
    use serde_json::json;
    let calls = [
        ("list_studies", json!({})),
        ("study_summary", json!({})),
        ("study_report", json!({"format":"json"})),
        ("study_report", json!({"format":"markdown","lang":"ja"})),
        ("trials", json!({"limit":2000})),
        ("trials", json!({"offset":1,"limit":2})),
        ("trials", json!({"offset":5,"limit":2})),
        ("trials", json!({"limit":0})),
        ("trials", json!({"study_id":999})),
    ];
    let requests: Vec<_> = calls
        .iter()
        .enumerate()
        .map(|(index, (name, extra))| {
            let mut args = json!({"storage":storage.to_string_lossy(),"study_id":1});
            args.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            req(
                index as i64 + 1,
                "tools/call",
                json!({"name":name,"arguments":args}),
            )
        })
        .collect();
    let responses = stdio(&requests);
    assert_eq!(responses.len(), requests.len());
    for (index, response) in responses.iter().enumerate() {
        assert_eq!(response["id"], index + 1);
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["result"]["isError"], index == 8, "{response}");
    }
    let text = |index: usize| {
        responses[index]["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
    };
    let payload = |index: usize| -> Value { serde_json::from_str(text(index)).unwrap() };
    let list = payload(0);
    let target = list["studies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["study_id"] == 1)
        .unwrap();
    assert_eq!(target["name"], "constrained");
    assert_eq!(target["directions"], json!(["minimize", "minimize"]));
    let summary = payload(1);
    assert_eq!(summary["overview"]["total_trials"], 6);
    assert_eq!(summary["overview"]["complete_trials"], 5);
    assert_eq!(summary["pareto"]["front_size"], 0);
    let report = payload(2);
    let outcome = &report["outcome"]["MultiObj"];
    assert_eq!(outcome["pareto_size"], 0);
    let candidates = outcome["pareto_table"].as_array().unwrap();
    assert_eq!(candidates.len(), 4);
    for (number, feasibility) in [
        (0, "Feasibility unverified"),
        (1, "Infeasible"),
        (2, "Feasibility unverified"),
        (4, "Feasibility unverified"),
    ] {
        let trial = candidates
            .iter()
            .find(|t| t["trial_number"] == number)
            .unwrap();
        assert_eq!(trial["feasibility"], feasibility);
    }
    // The RUNNING trial supplies width 3: trial #4's two negative values do not
    // establish feasibility. Infinite objective #3 is not a report candidate.
    assert_eq!(report["overview"]["state_counts"]["RUNNING"], 1);
    assert!(text(3).contains("constrained"));
    assert!(!text(3).contains("NaN"));
    let trials = payload(4);
    let rows = trials["trials"].as_array().unwrap();
    assert_eq!(trials["total_complete_trials"], 5);
    assert_eq!(trials["returned"], 5);
    assert_eq!(
        rows.iter()
            .map(|r| r["trial_number"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(rows[3]["objectives"]["obj0"], Value::Null);
    assert_eq!(rows[3]["params"]["x"], Value::Null);
    assert_eq!(rows[3]["constraints"].as_object().unwrap().len(), 3);
    assert!(rows[3]["constraints"]
        .as_object()
        .unwrap()
        .values()
        .any(Value::is_null));
    let page = payload(5);
    assert_eq!(page["offset"], 1);
    assert_eq!(page["returned"], 2);
    assert_eq!(page["trials"], json!([rows[1], rows[2]]));
    assert_eq!(payload(6)["returned"], 0);
    assert_eq!(payload(7)["returned"], 0);
    assert!(text(8).contains("999"));
}

#[test]
fn d7_journal_stdio_preserves_ids_all_state_constraints_and_nonfinite_json() {
    use serde_json::json;
    let directory = make_temp_dir();
    let path = directory.join("constrained.log");
    let mut lines = vec![
        json!({"op_code":0,"worker_id":"w","study_name":"other","directions":[1]}).to_string(),
        json!({"op_code":0,"worker_id":"w","study_name":"constrained","directions":[1,1]})
            .to_string(),
    ];
    for number in 0..6 {
        // Interleaving another Study makes global IDs unlike Study trial numbers.
        lines.push(json!({"op_code":4,"worker_id":"w","study_id":0}).to_string());
        lines.push(json!({"op_code":4,"worker_id":"w","study_id":1}).to_string());
        let id = number * 2 + 1;
        let constraints = match number {
            0 => Some("[-1]"),
            1 => Some("[1]"),
            2 => None,
            3 => Some("[NaN,-1]"),
            4 => Some("[-1,-1]"),
            _ => Some("[-1,-1,-1]"),
        };
        if let Some(values) = constraints {
            lines.push(format!(r#"{{"op_code":9,"worker_id":"w","trial_id":{id},"system_attr":{{"constraints":{values}}}}}"#));
        }
        if number < 5 {
            let x = if number == 3 {
                "Infinity".into()
            } else {
                number.to_string()
            };
            lines.push(format!(r#"{{"op_code":5,"worker_id":"w","trial_id":{id},"param_name":"x","param_value_internal":{x},"distribution":{{"name":"FloatDistribution","low":0,"high":10}}}}"#));
            let first = if number == 3 {
                "Infinity".into()
            } else {
                (number + 1).to_string()
            };
            let second = if number == 4 { 2 } else { 5 - number };
            lines.push(format!(r#"{{"op_code":6,"worker_id":"w","trial_id":{id},"state":1,"values":[{first},{second}]}}"#));
        }
    }
    std::fs::write(&path, lines.join("\n")).unwrap();
    check_constrained_storage(&path);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[ignore = "requires TUNNY_FIXTURE_PYTHON (Python 3 standard-library SQLite fixture writer)"]
fn d7_sqlite_stdio_preserves_ids_all_state_constraints_and_nonfinite_json() {
    let directory = make_temp_dir();
    let path = directory.join("constrained.db");
    let output =
        Command::new(std::env::var_os("TUNNY_FIXTURE_PYTHON").expect("Python 3 executable"))
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/create_sqlite_fixture.py"
            ))
            .arg(&path)
            .output()
            .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    check_constrained_storage(&path);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn d7_stdio_masks_credentials_in_rejected_storage_requests() {
    use serde_json::json;
    for scheme in ["postgresql", "mysql"] {
        let storage = format!("{scheme}://user:d7-secret@db.invalid/study?sslmode=require");
        let requests: Vec<_> = ["list_studies", "study_summary", "study_report", "trials"]
            .iter()
            .enumerate()
            .map(|(i, name)| {
                req(
                    i as i64,
                    "tools/call",
                    json!({"name":name,"arguments":{"storage":storage,"study_id":1}}),
                )
            })
            .collect();
        let responses = stdio(&requests);
        assert_eq!(responses.len(), 4);
        for response in responses {
            assert_eq!(response["result"]["isError"], true);
            assert!(!response.to_string().contains("d7-secret"));
            assert!(!response.to_string().contains(&storage));
        }
    }
}

#[test]
#[ignore = "requires the disposable genuine Optuna server fixtures"]
fn d3_live_servers_keep_stdio_ingestion_pagination_and_masked_reports() {
    use serde_json::json;
    for family in ["POSTGRES", "MYSQL"] {
        let storage = std::env::var(format!("TUNNY_SNAPSHOT_{family}_URL"))
            .expect("disposable server required");
        let study = tdashboard_io::storage::scan_studies(&storage)
            .unwrap()
            .into_iter()
            .find(|s| s.name == "tunny-server-snapshot-20")
            .unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_tunny-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        for (id, name, offset) in [
            (1, "study_summary", 0),
            (2, "study_report", 0),
            (3, "trials", 0),
            (4, "trials", 1),
        ] {
            writeln!(input,"{}",req(id,"tools/call",json!({"name":name,"arguments":{
                "storage":storage,"study_id":study.study_id,"format":"json","offset":offset,"limit":1
            }}))).unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let password = storage
            .split("://")
            .nth(1)
            .unwrap()
            .split('@')
            .next()
            .unwrap()
            .split(':')
            .nth(1)
            .unwrap();
        assert!(
            !text.contains(password),
            "credential must not appear in protocol output"
        );
        let responses: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(responses.len(), 4);
        let payload = |index: usize| -> Value {
            assert_eq!(responses[index]["result"]["isError"], false);
            serde_json::from_str(
                responses[index]["result"]["content"][0]["text"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
        };
        assert_eq!(payload(0)["overview"]["total_trials"], 2);
        assert_eq!(payload(1)["overview"]["complete_trials"], 1);
        assert!(payload(1)["source"]["storage_display"]
            .as_str()
            .unwrap()
            .contains("***"));
        assert_eq!(payload(2)["total_complete_trials"], 1);
        assert_eq!(payload(2)["returned"], 1);
        assert_eq!(payload(3)["returned"], 0);
    }
}

#[test]
fn categorical_importance_is_explicit_in_mcp_summary_and_report() {
    use serde_json::json;
    for mixed in [false, true] {
        let dir = make_temp_dir();
        let path = dir.join("nominal.log");
        let mut lines = vec![
            json!({"op_code":0,"worker_id":"w","study_name":"nominal","directions":[1]})
                .to_string(),
        ];
        for i in 0..6 {
            lines.push(json!({"op_code":4,"worker_id":"w","study_id":0}).to_string());
            lines.push(json!({"op_code":5,"worker_id":"w","trial_id":i,"param_name":"cat","param_value_internal":i % 3,"distribution":{"name":"CategoricalDistribution","choices":["steel","wood","glass"]}}).to_string());
            if mixed {
                lines.push(json!({"op_code":5,"worker_id":"w","trial_id":i,"param_name":"x","param_value_internal":i as f64,"distribution":{"name":"FloatDistribution","low":0.0,"high":10.0}}).to_string());
            }
            lines.push(
                json!({"op_code":6,"worker_id":"w","trial_id":i,"state":1,"values":[i as f64]})
                    .to_string(),
            );
        }
        std::fs::write(&path, lines.join("\n")).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_tunny-mcp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        for (id, name) in [(1, "study_summary"), (2, "study_report")] {
            writeln!(stdin, "{}", req(id, "tools/call", json!({"name":name,"arguments":{"storage":path.to_string_lossy(),"study_id":0,"format":"json"}}))).unwrap();
        }
        drop(stdin);
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let responses: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(responses.len(), 2);
        let payload = |index: usize| -> Value {
            assert_eq!(responses[index]["result"]["isError"], false);
            serde_json::from_str(
                responses[index]["result"]["content"][0]["text"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
        };
        assert_eq!(
            payload(0)["unsupported_categorical_importance"],
            json!(["cat"])
        );
        let report = payload(1);
        assert_eq!(
            report["importance"]["unsupported_categorical"],
            json!(["cat"])
        );
        let scores = report["importance"]["scores"].as_array().unwrap();
        assert_eq!(scores.len(), usize::from(mixed));
        assert!(scores.iter().all(|score| score[0] == "x"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn full_session_over_stdio() {
    let dir = make_temp_dir();
    let journal_path = dir.join("study.log");
    std::fs::write(&journal_path, journal_fixture()).expect("write journal fixture");
    let storage = journal_path.to_string_lossy().to_string();

    let mut child = Command::new(env!("CARGO_BIN_EXE_tunny-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tunny-mcp");

    let mut stdin = child.stdin.take().expect("child stdin");

    let requests = vec![
        req(
            1,
            "initialize",
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "it", "version": "0" },
            }),
        ),
        notif("notifications/initialized"),
        req(2, "tools/list", serde_json::json!({})),
        req(
            3,
            "tools/call",
            serde_json::json!({ "name": "list_studies", "arguments": { "storage": storage } }),
        ),
        req(
            4,
            "tools/call",
            serde_json::json!({
                "name": "study_summary",
                "arguments": { "storage": storage, "study_id": 0 },
            }),
        ),
        req(
            5,
            "tools/call",
            serde_json::json!({
                "name": "trials",
                "arguments": { "storage": storage, "study_id": 0, "limit": 2 },
            }),
        ),
        req(
            6,
            "tools/call",
            serde_json::json!({
                "name": "study_report",
                "arguments": { "storage": storage, "study_id": 0, "format": "markdown" },
            }),
        ),
    ];

    {
        let stdin = &mut stdin;
        for line in &requests {
            writeln!(stdin, "{line}").expect("write request line");
        }
        stdin.flush().expect("flush stdin");
    }
    // Closing stdin (EOF) terminates the server's main loop.
    drop(stdin);

    let stdout = child.stdout.take().expect("child stdout");
    let reader = BufReader::new(stdout);
    let response_lines: Vec<String> = reader
        .lines()
        .map(|l| l.expect("read stdout line"))
        .filter(|l| !l.trim().is_empty())
        .collect();

    let status = child.wait().expect("wait for child");
    assert!(status.success(), "tunny-mcp exited with {status:?}");

    // notifications/initialized gets no response, so 7 messages sent - 1 notification = 6 lines.
    assert_eq!(
        response_lines.len(),
        6,
        "unexpected response count: {response_lines:#?}"
    );

    let responses: Vec<Value> = response_lines
        .iter()
        .map(|l| {
            serde_json::from_str(l).unwrap_or_else(|e| panic!("invalid JSON-RPC line {l:?}: {e}"))
        })
        .collect();

    // Every response must be valid JSON-RPC with the corresponding id.
    let expected_ids: Vec<i64> = vec![1, 2, 3, 4, 5, 6];
    for (resp, expected_id) in responses.iter().zip(expected_ids.iter()) {
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"].as_i64(), Some(*expected_id), "resp={resp:#?}");
    }

    // ── initialize ──────────────────────────────────────────────
    let init = &responses[0];
    assert_eq!(init["result"]["serverInfo"]["name"], "tunny-mcp");

    // ── tools/list ──────────────────────────────────────────────
    let tools_list = &responses[1];
    let tools = tools_list["result"]["tools"]
        .as_array()
        .expect("tools array");
    assert_eq!(tools.len(), 4, "tools={tools:#?}");

    // ── tools/call list_studies ─────────────────────────────────
    let list_studies_resp = &responses[2];
    assert_eq!(list_studies_resp["result"]["isError"], false);
    let list_studies_text = list_studies_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("list_studies text content");
    let list_studies_json: Value =
        serde_json::from_str(list_studies_text).expect("list_studies JSON");
    let studies = list_studies_json["studies"].as_array().expect("studies");
    assert_eq!(studies.len(), 1, "studies={studies:#?}");
    assert_eq!(studies[0]["name"], "demo_study");
    assert_eq!(studies[0]["study_id"], 0);

    // ── tools/call study_summary ────────────────────────────────
    let study_summary_resp = &responses[3];
    assert_eq!(study_summary_resp["result"]["isError"], false);
    let study_summary_text = study_summary_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("study_summary text content");
    let study_summary_json: Value =
        serde_json::from_str(study_summary_text).expect("study_summary JSON");
    assert!(study_summary_json["overview"].is_object());
    assert_eq!(study_summary_json["overview"]["name"], "demo_study");
    assert!(study_summary_json["key_findings"].is_array());
    assert!(study_summary_json["convergence"].is_object());
    // Since this is a 2-objective study, the pareto section should appear (best should not).
    assert!(
        study_summary_json["pareto"].is_object(),
        "{study_summary_json:#?}"
    );
    assert!(study_summary_json.get("best").is_none());

    // ── tools/call trials (limit=2) ─────────────────────────────
    let trials_resp = &responses[4];
    assert_eq!(trials_resp["result"]["isError"], false);
    let trials_text = trials_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("trials text content");
    let trials_json: Value = serde_json::from_str(trials_text).expect("trials JSON");
    assert_eq!(trials_json["total_complete_trials"], 3);
    assert_eq!(trials_json["offset"], 0);
    assert_eq!(trials_json["returned"], 2);
    let trial_rows = trials_json["trials"].as_array().expect("trials array");
    assert_eq!(trial_rows.len(), 2);
    let first = &trial_rows[0];
    assert!(first["objectives"].get("obj0").is_some(), "{first:#?}");
    assert!(first["objectives"].get("obj1").is_some(), "{first:#?}");
    assert!(first["params"].get("x").is_some(), "{first:#?}");

    // ── tools/call study_report (markdown) ──────────────────────
    let study_report_resp = &responses[5];
    assert_eq!(study_report_resp["result"]["isError"], false);
    let study_report_text = study_report_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("study_report text content");
    // The Markdown renderer escapes `_` in user-derived strings so it isn't misinterpreted as
    // emphasis markup, so the study name becomes `demo\_study`.
    assert!(study_report_text.contains(r"demo\_study"));
    assert!(study_report_text.contains("## "));

    std::fs::remove_dir_all(&dir).ok();
}
