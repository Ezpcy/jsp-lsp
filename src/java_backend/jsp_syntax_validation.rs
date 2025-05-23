use anyhow::Result;
use log::error;
use std::{error::Error, io::Write, sync::{Arc, Mutex}};
use crate::Backend;

use super::java_lsp_connections::JavaLspConnection;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, MessageType, Position, Range, Url};

impl Backend { 
/// Validate a JSP file for unclosed `<%` tags and return a list of diagnostics.
    pub async fn validate_text(&self, uri: Url, text: String) {
        if text.contains("<%") {
            let mut diagnostics = Vec::new();
            let mut stack = Vec::new();
            let mut java_syntax = Vec::new();

            for (line_idx, line) in text.lines().enumerate() {
                let mut col = 0;

                while let Some(start) = line[col..].find("<%") {
                    let absolute_start = col + start;
                    stack.push((line_idx, absolute_start));
                    col = absolute_start + 2;
                }

                col = 0;
                while let Some(end) = line[col..].find("%>") {
                    let absolute_start = col + end;
                    match stack.pop() {
                    Some(_) => {
                            if let (Some(start_pos), Some(end_pos)) = (text.find("<%"), text.find("%>")) {
                                java_syntax.push(text[start_pos + 2..end_pos].to_string())
                            }
                        },
                    None => {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position { line: line_idx as u32, character: absolute_start as u32 },
                                end: Position { line: line_idx as u32, character: (absolute_start +2) as u32 },
                            },
                            severity: Some(DiagnosticSeverity::WARNING),
                            message: "Unopened %> tag".to_string(),
                            ..Default::default()
                        });
                        }
                    }
                    col += end + 2;
                }

            }

            for (line, col) in stack {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position::new(line as u32, col as u32),
                        end: Position::new(line as u32, (col + 2) as u32),
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    message: "Unclosed <% tag".to_string(),
                    ..Default::default()
                });
            }
        

        self.client.log_message(MessageType::INFO, java_syntax.join(" ")).await;

        if let lsp = self.java_lsp.lock().await.is_none() {
            self.client.log_message(MessageType::ERROR, "Java LSP not initiated.").await
        }


        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
        }
    }
}
