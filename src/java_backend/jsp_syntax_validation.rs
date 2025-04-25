use std::ops::Deref;

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

pub struct JspSyntaxValidation<'a> {
    uri: &'a Url,
    text: &'a str,
    diagnostics: Vec<Diagnostic>, 
}

impl <'a>JspSyntaxValidation<'a> {
    pub fn run(uri: &'a Url, text: &'a str) -> Vec<Diagnostic> {
        let mut validator = JspSyntaxValidation {
            uri :uri,
            text : text,
            diagnostics : Vec::new(),
        };

        validator.validate_jsp_tags();
        
        validator.diagnostics
    }

    /// Validate a JSP file for unclosed `<%` tags and return a list of diagnostics.
    pub fn validate_jsp_tags(&mut self) {
        let mut stack = Vec::new();

        for (line_idx, line) in self.text.lines().enumerate() {
            let mut col = 0;

            while let Some(start) = line[col..].find("<%") {
                let absolute_start = col + start;
                stack.push((line_idx, absolute_start));
                col = absolute_start + 2;
            }

            col = 0;
            while let Some(end) = line[col..].find("%>") {
                let absoulte_start = col + end;
                if stack.pop().is_none() {
                    self.diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position { line: line_idx as u32, character: absoulte_start as u32 },
                            end: Position { line: line_idx as u32, character: (absoulte_start +2 ) as u32 },
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        message: "Unopened %> tag".to_string(),
                        ..Default::default()
                    });
                }
                col += end + 2;
            }
        }

        for (line, col) in stack {
            self.diagnostics.push(Diagnostic {
                range: Range {
                    start: Position::new(line as u32, col as u32),
                    end: Position::new(line as u32, (col + 2) as u32),
                },
                severity: Some(DiagnosticSeverity::WARNING),
                message: "Unclosed <% tag".to_string(),
                ..Default::default()
            });
        }

    }
}



