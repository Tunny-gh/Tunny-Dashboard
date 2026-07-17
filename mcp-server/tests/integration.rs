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
