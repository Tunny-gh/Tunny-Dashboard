//! JSON-RPC 2.0 / MCP プロトコル層。
//!
//! stdio トランスポートの改行区切り JSON-RPC を処理する。MCP のうち
//! tools 機能のみを実装する（resources / prompts は未対応で、
//! capabilities にも載せない）。
//!
//! 対応メソッド:
//! - `initialize` → protocolVersion / capabilities / serverInfo
//! - `notifications/initialized` → 無応答（通知）
//! - `ping` → 空オブジェクト
//! - `tools/list` → ツール定義（JSON Schema 付き）
//! - `tools/call` → ツール実行。結果は `content: [{type: "text", ...}]`
//!
//! 未知のメソッドは id 付きなら `-32601 Method not found`、通知なら無視する。

use serde_json::{json, Value};

use crate::tools;

/// サーバーが名乗る MCP プロトコル版。クライアントが別の版を要求しても
/// この版で応答する（クライアント側が互換性を判断する）。
const PROTOCOL_VERSION: &str = "2024-11-05";

/// サーバー状態。現状は初期化済みフラグのみ（ツールは無状態）。
pub struct Server {
    initialized: bool,
}

impl Server {
    pub fn new() -> Server {
        Server { initialized: false }
    }

    /// 1 メッセージを処理し、返すべき応答があれば JSON 文字列で返す。
    ///
    /// 通知（id なし）には `None` を返す。パース不能な行には JSON-RPC の
    /// 規定どおり `-32700 Parse error`（id = null）を返す。
    pub fn handle_message(&mut self, line: &str) -> Option<String> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                return Some(error_response(Value::Null, -32700, "Parse error").to_string());
            }
        };

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        // 応答メッセージ（result/error を持ち method を持たない）は無視する。
        if method.is_empty() {
            return None;
        }

        match (method, id) {
            // ── 通知 ─────────────────────────────────────────────
            ("notifications/initialized", None) => {
                self.initialized = true;
                None
            }
            (_, None) => None, // 未知の通知は無視

            // ── リクエスト ───────────────────────────────────────
            ("initialize", Some(id)) => Some(ok_response(id, self.initialize()).to_string()),
            ("ping", Some(id)) => Some(ok_response(id, json!({})).to_string()),
            ("tools/list", Some(id)) => {
                Some(ok_response(id, json!({ "tools": tools::definitions() })).to_string())
            }
            ("tools/call", Some(id)) => Some(self.tools_call(id, &params).to_string()),
            (_, Some(id)) => Some(error_response(id, -32601, "Method not found").to_string()),
        }
    }

    fn initialize(&mut self) -> Value {
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "tunny-mcp",
                "version": env!("CARGO_PKG_VERSION"),
            },
        })
    }

    fn tools_call(&mut self, id: Value, params: &Value) -> Value {
        let name = params.get("name").and_then(Value::as_str).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        match tools::call(name, &arguments) {
            // ツール実行の成功。テキストコンテンツ 1 件で返す。
            Ok(text) => ok_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false,
                }),
            ),
            // ツール実行時エラーは MCP の規定どおり isError: true の
            // ツール結果として返す（プロトコルエラーにしない）。
            Err(tools::ToolError::Execution(msg)) => ok_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": msg }],
                    "isError": true,
                }),
            ),
            // 未知のツール名はプロトコルエラー。
            Err(tools::ToolError::UnknownTool) => {
                error_response(id, -32602, &format!("Unknown tool: {name}"))
            }
        }
    }
}

fn ok_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(server: &mut Server, line: &str) -> Value {
        serde_json::from_str(&server.handle_message(line).expect("response expected")).unwrap()
    }

    #[test]
    fn initialize_handshake() {
        let mut s = Server::new();
        let resp = call(
            &mut s,
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}"#,
        );
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(resp["result"]["serverInfo"]["name"], "tunny-mcp");
        assert!(resp["result"]["capabilities"]["tools"].is_object());

        // initialized 通知は無応答。
        assert!(s
            .handle_message(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none());
    }

    #[test]
    fn ping_returns_empty_object() {
        let mut s = Server::new();
        let resp = call(&mut s, r#"{"jsonrpc":"2.0","id":"p1","method":"ping"}"#);
        assert_eq!(resp["id"], "p1");
        assert_eq!(resp["result"], json!({}));
    }

    #[test]
    fn tools_list_contains_all_tools() {
        let mut s = Server::new();
        let resp = call(&mut s, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
        let tools: Vec<&str> = resp["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(tools.contains(&"list_studies"), "{tools:?}");
        assert!(tools.contains(&"study_summary"), "{tools:?}");
        assert!(tools.contains(&"study_report"), "{tools:?}");
        assert!(tools.contains(&"trials"), "{tools:?}");
        // 全ツールが inputSchema を持つこと。
        for t in resp["result"]["tools"].as_array().unwrap() {
            assert_eq!(t["inputSchema"]["type"], "object", "tool={}", t["name"]);
        }
    }

    #[test]
    fn unknown_method_is_error_and_unknown_notification_ignored() {
        let mut s = Server::new();
        let resp = call(
            &mut s,
            r#"{"jsonrpc":"2.0","id":3,"method":"resources/list"}"#,
        );
        assert_eq!(resp["error"]["code"], -32601);
        assert!(s
            .handle_message(r#"{"jsonrpc":"2.0","method":"notifications/cancelled"}"#)
            .is_none());
    }

    #[test]
    fn parse_error_returns_null_id() {
        let mut s = Server::new();
        let resp = call(&mut s, "not json");
        assert_eq!(resp["error"]["code"], -32700);
        assert!(resp["id"].is_null());
    }

    #[test]
    fn unknown_tool_is_invalid_params_error() {
        let mut s = Server::new();
        let resp = call(
            &mut s,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"nope","arguments":{}}}"#,
        );
        assert_eq!(resp["error"]["code"], -32602);
    }

    #[test]
    fn tool_execution_error_is_iserror_result() {
        let mut s = Server::new();
        // 存在しないストレージ → 実行エラー（isError: true のツール結果）。
        let resp = call(
            &mut s,
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"list_studies","arguments":{"storage":"/nonexistent/x.db"}}}"#,
        );
        assert_eq!(resp["result"]["isError"], true);
        assert!(resp["result"]["content"][0]["text"].is_string());
    }
}
