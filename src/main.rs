use std::char;
use std::collections::HashSet;

use java_backend::jsp_syntax_validation::validate_jsp_tags;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod java_backend;

#[derive(Debug)]
pub struct Backend {
    client: Client,
}

impl Backend {
    async fn validate_text(&self, uri: Url, text: String) {
        if text.contains("<%") {
            let diagnostics = validate_jsp_tags(&uri, &text);
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate_text(
            params.text_document.uri.clone(),
            params.text_document.text.clone(),
        )
        .await;
        self.client
            .log_message(MessageType::INFO, params.text_document.text.clone())
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.client
                .log_message(MessageType::INFO, change.text.clone())
                .await;
            self.validate_text(params.text_document.uri, change.text.clone())
                .await;
        }
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string()),
        ])))
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("You're hovering!".to_string())),
            range: None,
        }))
    }
}

const HELP: &str = r#"
Jsp-lsp ritten in Rust
Usage: jsp-lsp --stdio -p <PathToJdtLangServer> -c <PathToConfigDirectory> -w <PathToWorkspaceDirectory>

Arguments:
    -p  Path to the jdt language server jar file
    -c  Path to the JDT LS config directory (e.g. config_linux)
    -w  Path to the Java workspace directory
"#;

pub enum ArgErrorType {
    DuplicateFlag,
    NoPathProvided,
    UnknownArgument,
    Help,
}

pub fn argumentError(error_type: ArgErrorType) {
    match error_type {
        ArgErrorType::DuplicateFlag => {
            println!("Duplicate flag passed");
        }
        ArgErrorType::NoPathProvided => {
            println!("No path provided");
        }
        ArgErrorType::UnknownArgument => {
            println!("Unknown Argument");
        }
        _ => {}
    }
    println!("{}", HELP);
}

#[tokio::main]
async fn main() {
    let mut seen: HashSet<char> = HashSet::new();
    let mut args: Vec<String> = std::env::args().collect();
    let (mut path, mut config_path, mut workspace_path) = ("", "", "");
    args.remove(0);
    if args.is_empty() {
        argumentError(ArgErrorType::Help);
        return;
    }
    let mut is_read = false;
    for i in 1..args.len() {
        if is_read {
            is_read = false;
            continue;
        }
        let arg: &str = args[i].as_ref();
        if arg == "--stdio" {
            continue;
        }

        if arg.starts_with("-") {
            let flag = arg.to_string().remove(1);
            match flag {
                'p' => {
                    if seen.contains(&flag) {
                        argumentError(ArgErrorType::DuplicateFlag);
                    } else {
                        seen.insert(flag);
                    }
                    if let Some(value) = args.get(i + 1) {
                        is_read = true;
                        path = value;
                    } else {
                        argumentError(ArgErrorType::NoPathProvided);
                        return;
                    }
                }
                'c' => {
                    if seen.contains(&flag) {
                        argumentError(ArgErrorType::DuplicateFlag);
                    } else {
                        seen.insert(flag);
                    }
                    if let Some(value) = args.get(i + 1) {
                        is_read = true;
                        config_path = value;
                    } else {
                        argumentError(ArgErrorType::NoPathProvided);
                        return;
                    }
                }
                'w' => {
                    if seen.contains(&flag) {
                        argumentError(ArgErrorType::DuplicateFlag);
                    } else {
                        seen.insert(flag);
                    }
                    if let Some(value) = args.get(i + 1) {
                        is_read = true;
                        workspace_path = value;
                    } else {
                        argumentError(ArgErrorType::NoPathProvided);
                        return;
                    }
                }
                'h' => {
                    argumentError(ArgErrorType::Help);
                    return;
                }
                _ => {
                    argumentError(ArgErrorType::UnknownArgument);
                    return;
                }
            };
        } else {
            argumentError(ArgErrorType::UnknownArgument);
            return;
        }
    }

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
