use crate::Backend;
use std::panic;

use super::java_lsp_connections::{JavaLspConnection, JavaLspMethod};
use serde_json::json;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, MessageType, Position, Range, Url};

struct JavaBlock {
    java_code: String,
    jsp_start_line: usize,
    jsp_start_col: usize,
    java_start_line: usize,
    java_line_count: usize,
}

impl Backend {
    /// Validate a JSP file for unclosed `<%` tags and return a list of diagnostics.
    pub async fn validate_text(&self, uri: Url, text: String, method: JavaLspMethod) {
        if text.contains("<%") {
            let mut java_blocks = Vec::new();
            let mut java_file_lines = Vec::new();
            let mut current_java_line = 0;

            for (line_idx, line) in text.lines().enumerate() {
                let mut search_start = 0;
                while let Some(start) = line[search_start..].find("<%") {
                    let abs_start = search_start + start;
                    if let Some(end) = line[abs_start..].find("%>") {
                        let abs_end = abs_start + end + 2;
                        let java_code = &line[abs_start + 2..abs_end - 2];
                        let code_lines: Vec<&str> = java_code.lines().collect();

                        java_blocks.push(JavaBlock {
                            java_code: java_code.to_string(),
                            jsp_start_line: line_idx,
                            jsp_start_col: abs_start,
                            java_start_line: current_java_line,
                            java_line_count: code_lines.len().max(1),
                        });
                        for code_line in code_lines {
                            java_file_lines.push(code_line.to_string());
                        }
                        current_java_line += code_lines.len().max(1);

                        search_start = abs_end;
                    } else {
                        // Unclosed
                        self.client
                            .publish_diagnostics(
                                uri.clone(),
                                vec![Diagnostic {
                                    range: Range {
                                        start: Position::new(line_idx as u32, abs_start as u32),
                                        end: Position::new(line_idx as u32, (abs_start + 2) as u32),
                                    },
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    message: "Unclosed <% tag".to_string(),
                                    ..Default::default()
                                }],
                                None,
                            )
                            .await;
                        break;
                    }
                }
            }
            let virtual_java_code = java_file_lines.join("\n");
            let virtual_java_uri = Url::parse("file:///__virtual_jsp_temp.java").unwrap();

            let guard = self.java_lsp.lock().await;
            let java_lsp = match guard.as_ref() {
                Some(a) => a,
                None => {
                    self.client
                        .log_message(MessageType::ERROR, "Java LSP not initiated.")
                        .await;
                    return;
                }
            };

            let did_open = json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": virtual_java_uri,
                        "languageId": "java",
                        "version": 1,
                        "text": virtual_java_code,
                    }
                }
            });
            java_lsp.send_message(&did_open.to_string()).await.unwrap();

            let rec_msg = java_lsp.read_message().await.unwrap();
            let v: serde_json::Value = serde_json::from_str(&rec_msg).unwrap();

            let diagnostic = Vec::new();


            if let Some(param) = 

            self.client.log_message(MessageType::INFO, rec_msg).await;

            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }
}
