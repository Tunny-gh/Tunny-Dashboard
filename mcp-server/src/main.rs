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

use std::io::{BufRead, Read, Write};

/// 1 行（= 1 JSON-RPC メッセージ）の最大バイト数。
/// 改行なしの巨大入力によるメモリ枯渇（DoS）を防ぐ上限で、正当な
/// ツール呼び出し（storage パス + 引数）には十分すぎる余裕を持たせている。
const MAX_LINE_BYTES: u64 = 8 * 1024 * 1024;

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut reader = stdin.lock();

    let mut server = rpc::Server::new();

    loop {
        // 行長上限付き読み込み。上限を超えた行はフレーミングを再同期
        // できないため、エラー応答を返して終了する。
        let mut buf = String::new();
        match (&mut reader).take(MAX_LINE_BYTES).read_line(&mut buf) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("tunny-mcp: stdin read error: {e}");
                break;
            }
        }
        if buf.len() as u64 >= MAX_LINE_BYTES && !buf.ends_with('\n') {
            eprintln!("tunny-mcp: input line exceeds {MAX_LINE_BYTES} bytes; terminating");
            let _ = writeln!(
                out,
                r#"{{"jsonrpc":"2.0","id":null,"error":{{"code":-32600,"message":"Request exceeds maximum message size"}}}}"#
            );
            break;
        }
        if buf.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle_message(&buf) {
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
