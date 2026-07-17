//! tunny-mcp: an MCP server exposing Tunny Dashboard's Optuna analysis features.
//!
//! Implements the MCP (Model Context Protocol) stdio transport.
//! Reads newline-delimited JSON-RPC 2.0 messages from stdin and writes
//! responses to stdout, one message per line. Logs/diagnostics go only to
//! stderr (stdout is reserved for the protocol).
//!
//! ```text
//! tunny-mcp            # launched by an MCP client (e.g. Claude Code)
//! ```
//!
//! Example client registration (Claude Code):
//!
//! ```text
//! claude mcp add tunny -- /path/to/tunny-mcp
//! ```

mod rpc;
mod storage;
mod tools;

use std::io::{BufRead, Read, Write};

/// The maximum bytes for one line (= one JSON-RPC message).
/// This upper bound prevents memory exhaustion (DoS) from a huge input with no
/// newline, while leaving more than enough headroom for a legitimate tool
/// call (storage path + arguments).
const MAX_LINE_BYTES: u64 = 8 * 1024 * 1024;

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut reader = stdin.lock();

    let mut server = rpc::Server::new();

    loop {
        // Reads with a line-length cap. A line exceeding the cap can't have
        // its framing resynchronized, so an error response is returned and
        // the loop terminates.
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
            // One message per line (newline-delimited JSON-RPC).
            if writeln!(out, "{response}")
                .and_then(|()| out.flush())
                .is_err()
            {
                // stdout was closed = the client exited.
                break;
            }
        }
    }
}
