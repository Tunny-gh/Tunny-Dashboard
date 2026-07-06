//! tunny-mcp: Tunny Dashboard の Optuna 分析機能を公開する MCP サーバー。
//!
//! MCP (Model Context Protocol) の stdio トランスポートを実装する。
//! stdin から改行区切りの JSON-RPC 2.0 メッセージを読み、stdout へ
//! 応答を 1 行 1 メッセージで書く。ログ・診断は stderr のみに出す
//! （stdout はプロトコル専用）。
//!
//! ```text
//! tunny-mcp            # MCP クライアント（Claude Code 等）から起動される
//! ```
//!
//! クライアント登録例（Claude Code）:
//!
//! ```text
//! claude mcp add tunny -- /path/to/tunny-mcp
//! ```

mod rpc;
mod storage;
mod tools;

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut server = rpc::Server::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("tunny-mcp: stdin read error: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle_message(&line) {
            // 1 メッセージ 1 行（改行区切り JSON-RPC）。
            if writeln!(out, "{response}")
                .and_then(|()| out.flush())
                .is_err()
            {
                // stdout が閉じられた = クライアント終了。
                break;
            }
        }
    }
}
