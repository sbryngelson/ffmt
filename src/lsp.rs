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

    serde_json::from_slice(&body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn write_message(writer: &mut impl Write, msg: &Value) {
    let body = serde_json::to_string(msg).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).ok();
    writer.write_all(body.as_bytes()).ok();
    writer.flush().ok();
}

/// Compute the LSP range covering the entire document.
///
/// For a newline-terminated document, `{line: line_count, character: 0}` is
/// the canonical end-of-document position. Without a trailing newline, the
/// end must point at the end of the last line (in UTF-16 code units, per the
/// LSP spec) — one line past the end is an invalid position some clients
/// reject.
fn full_document_range(text: &str) -> Value {
    let line_count = text.lines().count();
    let end = if text.is_empty() || text.ends_with('\n') {
        json!({"line": line_count, "character": 0})
    } else {
        let last_line = text.lines().last().unwrap_or("");
        json!({
            "line": line_count.saturating_sub(1),
            "character": last_line.encode_utf16().count(),
        })
    };
    json!({
        "start": {"line": 0, "character": 0},
        "end": end,
    })
}

/// Resolve the config for a document: search for `ffmt.toml` upward from the
/// document's directory (same discovery as the CLI). Falls back to the given
/// config for non-`file://` URIs.
fn config_for_uri(uri: &str, fallback: &Config) -> Config {
    if let Some(path) = uri.strip_prefix("file://") {
        if let Some(dir) = std::path::Path::new(path).parent() {
            return Config::find_and_load(dir);
        }
    }
    fallback.clone()
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
            let uri = msg["params"]["textDocument"]["uri"].as_str().unwrap_or("");
            documents.remove(uri);
            None
        }

        "textDocument/formatting" => {
            let id = &msg["id"];
            let uri = msg["params"]["textDocument"]["uri"].as_str().unwrap_or("");

            if let Some(text) = documents.get(uri) {
                let config = config_for_uri(uri, config);
                let formatted = crate::formatter::format_with_config(text, &config, None);
                if formatted == *text {
                    return Some(vec![json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": []
                    })]);
                }

                Some(vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": [{
                        "range": full_document_range(text),
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
            let uri = msg["params"]["textDocument"]["uri"].as_str().unwrap_or("");
            let start_line = msg["params"]["range"]["start"]["line"]
                .as_u64()
                .unwrap_or(0) as usize;
            let end_line = msg["params"]["range"]["end"]["line"].as_u64().unwrap_or(0) as usize;

            if let Some(text) = documents.get(uri) {
                // LSP lines are 0-based, ffmt range is 1-based
                let config = config_for_uri(uri, config);
                let formatted = crate::formatter::format_with_config(
                    text,
                    &config,
                    Some((start_line + 1, end_line + 1)),
                );

                if formatted == *text {
                    return Some(vec![json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": []
                    })]);
                }

                Some(vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": [{
                        "range": full_document_range(text),
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
            msg.get("id").map(|id| {
                vec![json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("method not found: {}", method)
                    }
                })]
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_doc(documents: &mut HashMap<String, String>, uri: &str, text: &str) {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"textDocument": {"uri": uri, "text": text}}
        });
        handle_message(&msg, documents, &Config::default());
    }

    fn format_doc(documents: &mut HashMap<String, String>, uri: &str) -> Value {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/formatting",
            "params": {"textDocument": {"uri": uri}}
        });
        let resp = handle_message(&msg, documents, &Config::default()).unwrap();
        resp[0]["result"].clone()
    }

    #[test]
    fn full_doc_range_in_bounds_without_trailing_newline() {
        let mut docs = HashMap::new();
        // 3 lines, no trailing newline: last valid position is line 2, char 13.
        open_doc(&mut docs, "file:///t.f90", "program t\nx=1\nend program t");
        let result = format_doc(&mut docs, "file:///t.f90");
        let end = &result[0]["range"]["end"];
        assert_eq!(end["line"], 2, "end.line out of bounds: {end}");
        assert_eq!(
            end["character"],
            "end program t".encode_utf16().count(),
            "end.character must cover the last line: {end}"
        );
    }

    #[test]
    fn full_doc_range_covers_trailing_newline() {
        let mut docs = HashMap::new();
        // Newline-terminated: {line: line_count, char: 0} is the canonical doc end.
        open_doc(
            &mut docs,
            "file:///t.f90",
            "program t\nx=1\nend program t\n",
        );
        let result = format_doc(&mut docs, "file:///t.f90");
        let end = &result[0]["range"]["end"];
        assert_eq!(end["line"], 3);
        assert_eq!(end["character"], 0);
    }

    #[test]
    fn formatting_uses_config_from_document_directory() {
        let dir = std::env::temp_dir()
            .join("ffmt-lsp-tests")
            .join(format!("config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("ffmt.toml"), "indent-width = 2\n").unwrap();
        let file = dir.join("t.f90");
        std::fs::write(&file, "").unwrap();
        let uri = format!("file://{}", file.display());

        let mut docs = HashMap::new();
        open_doc(&mut docs, &uri, "program t\nx = 1\nend program t\n");
        let result = format_doc(&mut docs, &uri);
        let new_text = result[0]["newText"].as_str().unwrap();
        assert!(
            new_text.contains("\n  x = 1\n"),
            "expected 2-space indent from ffmt.toml, got:\n{new_text}"
        );
    }
}
