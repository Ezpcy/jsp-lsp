use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use std::sync::{Arc, Mutex};

use super::java_lsp_connections::JavaLspConnection;

/// Validate a JSP file for unclosed `<%` tags and return a list of diagnostics.
pub fn validate_jsp_tags(uri: &Url, text: &str, java_lsp: Arc<Mutex<JavaLspConnection>>) -> Vec<Diagnostic> {
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
            let absoulte_start = col + end;
            match stack.pop() {
            Some(s) => java_syntax.push(line[(s.1)..absoulte_start].to_string()),
            None => {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: line_idx as u32, character: absoulte_start as u32 },
                        end: Position { line: line_idx as u32, character: (absoulte_start +2) as u32 },
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

diagnostics

}




