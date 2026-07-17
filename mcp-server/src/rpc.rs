//! JSON-RPC 2.0 / MCP protocol layer.
//!
//! Handles newline-delimited JSON-RPC over the stdio transport. Implements only the tools
//! feature of MCP (resources / prompts are unsupported, and aren't listed in capabilities either).
//!
//! Supported methods:
//! - `initialize` -> protocolVersion / capabilities / serverInfo
//! - `notifications/initialized` -> no response (notification)
//! - `ping` -> empty object
//! - `tools/list` -> tool definitions (with JSON Schema)
//! - `tools/call` -> executes a tool. The result is `content: [{type: "text", ...}]`
//!
//! Unknown methods get `-32601 Method not found` if they have an id, or are ignored if they're a notification.

use serde_json::{json, Value};

use crate::tools;

/// The MCP protocol version this server claims to speak. Even if a client requests a different
/// version, the server responds with this version (the client decides compatibility).
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server state. Tools are stateless, so it currently has no fields
/// (a `tools/call` before `initialize` is also accepted leniently).
pub struct Server {}

impl Server {
    pub fn new() -> Server {
        Server {}
    }

    /// Handles a single message and returns the response as a JSON string, if one is required.
    ///
    /// Returns `None` for notifications (no id). Returns `-32700 Parse error` (id = null) for
    /// unparseable lines, per the JSON-RPC spec. Batches (top-level arrays) are unsupported, so
    /// `-32600 Invalid Request` is returned explicitly to avoid leaving the client hanging with no response.
    pub fn handle_message(&mut self, line: &str) -> Option<String> {
        let msg: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                return Some(error_response(Value::Null, -32700, "Parse error").to_string());
            }
        };

        if msg.is_array() {
            return Some(
                error_response(
                    Value::Null,
                    -32600,
                    "Batch requests are not supported by this server",
                )
                .to_string(),
            );
        }

        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        // Ignore response messages (which have result/error but no method).
        if method.is_empty() {
            return None;
        }

        match (method, id) {
            // ── Notifications ─────────────────────────────────────────────
            ("notifications/initialized", None) => None,
            (_, None) => None, // Ignore unknown notifications

            // ── Requests ───────────────────────────────────────
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
            // Successful tool execution. Returned as a single text content item.
            Ok(text) => ok_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false,
                }),
            ),
            // A tool execution error is returned as a tool result with isError: true,
            // per the MCP spec (not treated as a protocol error).
            Err(tools::ToolError::Execution(msg)) => ok_response(
                id,
                json!({
                    "content": [{ "type": "text", "text": msg }],
                    "isError": true,
                }),
            ),
            // An unknown tool name is a protocol error.
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

        // The initialized notification gets no response.
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
        // Every tool must have an inputSchema.
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
    fn batch_request_gets_explicit_invalid_request() {
        // Batches (arrays) are unsupported. Ignoring them would leave the client hanging
        // waiting for a response, so -32600 is returned explicitly.
        let mut s = Server::new();
        let resp = call(
            &mut s,
            r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#,
        );
        assert_eq!(resp["error"]["code"], -32600);
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
        // A nonexistent storage -> execution error (a tool result with isError: true).
        let resp = call(
            &mut s,
            r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"list_studies","arguments":{"storage":"/nonexistent/x.db"}}}"#,
        );
        assert_eq!(resp["result"]["isError"], true);
        assert!(resp["result"]["content"][0]["text"].is_string());
    }
}
