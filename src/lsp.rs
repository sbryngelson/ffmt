//! Minimal LSP server for ffmt.
//!
//! Supports:
//! - textDocument/formatting (full document)
//! - textDocument/rangeFormatting (line range)
//!
//! Run with: `ffmt --lsp`

use crate::config::Config;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

/// Run the LSP server on stdin/stdout.
pub fn run_lsp() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut documents: HashMap<String, String> = HashMap::new();
    let config = Config::default();

    while let Ok(msg) = read_message(&mut reader) {
        if let Some(responses) = handle_message(&msg, &mut documents, &config) {
            for response in responses {
                write_message(&mut writer, &response);
            }
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Value> {
    // Read headers
    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        let header = header.trim();
        if header.is_empty() {
            break;
        }
        if let Some(len_str) = header.strip_prefix("Content-Length: ") {
            content_length = len_str
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
    }

    if content_length == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "no content length",
        ));
    }

    // Read body
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;

    serde_json::from_slice(&body)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_message(writer: &mut impl Write, msg: &Value) {
    let body = serde_json::to_string(msg).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).ok();
    writer.write_all(body.as_bytes()).ok();
    writer.flush().ok();
}

fn handle_message(
    msg: &Value,
    documents: &mut HashMap<String, String>,
    config: &Config,
) -> Option<Vec<Value>> {
    let method = msg["method"].as_str()?;

    match method {
        "initialize" => {
            let id = &msg["id"];
            Some(vec![json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "capabilities": {
                        "textDocumentSync": 1,  // Full sync
                        "documentFormattingProvider": true,
                        "documentRangeFormattingProvider": true,
                    }
                }
            })])
        }
        "initialized" => None,
        "shutdown" => {
            let id = &msg["id"];
            Some(vec![json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": null
            })])
        }
        "exit" => std::process::exit(0),

        "textDocument/didOpen" => {
            let uri = msg["params"]["textDocument"]["uri"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let text = msg["params"]["textDocument"]["text"]
                .as_str()
                .unwrap_or("")
                .to_string();
            documents.insert(uri, text);
            None
        }
        "textDocument/didChange" => {
            let uri = msg["params"]["textDocument"]["uri"]
                .as_str()
                .unwrap_or("")
                .to_string();
            // Full sync: take the full content from the first change
            if let Some(changes) = msg["params"]["contentChanges"].as_array() {
                if let Some(change) = changes.first() {
                    if let Some(text) = change["text"].as_str() {
                        documents.insert(uri, text.to_string());
                    }
                }
            }
            None
        }
        "textDocument/didClose" => {
            let uri = msg["params"]["textDocument"]["uri"]
                .as_str()
                .unwrap_or("");
            documents.remove(uri);
            None
        }

        "textDocument/formatting" => {
            let id = &msg["id"];
            let uri = msg["params"]["textDocument"]["uri"]
                .as_str()
                .unwrap_or("");

            if let Some(text) = documents.get(uri) {
                let formatted = crate::formatter::format_with_config(text, config, None);
                if formatted == *text {
                    return Some(vec![json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": []
                    })]);
                }

                let line_count = text.lines().count();
                Some(vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": [{
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": line_count, "character": 0}
                        },
                        "newText": formatted
                    }]
                })])
            } else {
                Some(vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": []
                })])
            }
        }

        "textDocument/rangeFormatting" => {
            let id = &msg["id"];
            let uri = msg["params"]["textDocument"]["uri"]
                .as_str()
                .unwrap_or("");
            let start_line = msg["params"]["range"]["start"]["line"]
                .as_u64()
                .unwrap_or(0) as usize;
            let end_line = msg["params"]["range"]["end"]["line"]
                .as_u64()
                .unwrap_or(0) as usize;

            if let Some(text) = documents.get(uri) {
                // LSP lines are 0-based, ffmt range is 1-based
                let formatted = crate::formatter::format_with_config(
                    text,
                    config,
                    Some((start_line + 1, end_line + 1)),
                );

                if formatted == *text {
                    return Some(vec![json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": []
                    })]);
                }

                let line_count = text.lines().count();
                Some(vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": [{
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": line_count, "character": 0}
                        },
                        "newText": formatted
                    }]
                })])
            } else {
                Some(vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": []
                })])
            }
        }

        _ => {
            // Unknown method — return method not found for requests (with id)
            if let Some(id) = msg.get("id") {
                Some(vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("method not found: {}", method)
                    }
                })])
            } else {
                None // notification, ignore
            }
        }
    }
}
