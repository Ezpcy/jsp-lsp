mod java_backend;

use java_backend::java_lsp_connections::JavaLspConnection;
use std::{char, panic};
use std::collections::HashSet;
use std::sync::{Arc, LockResult, Mutex, MutexGuard};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug, Clone)]
pub struct Backend {
    path: String,
    config_path: String,
    client: Client,
    java_lsp: Arc<Mutex<Option<JavaLspConnection>>>,
}


#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, init_params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root_uri) = init_params.root_uri {
            let workspace_path = {
                let root_path = root_uri.to_file_path().unwrap();

                // Universal cache dir
                let cache_dir = dirs::cache_dir().unwrap();
                let base_dir = cache_dir.join("jsp-lsp/jdtls/workspaces");

                let escaped = root_path
                    .to_string_lossy()
                    .replace("/", "_")
                    .replace("\\", "_");
                
                let ws_path = base_dir.join(escaped);

                std::fs::create_dir_all(&ws_path).ok().unwrap();
                
                ws_path
            };
            
            let lsp =  JavaLspConnection::new(self.path.to_owned(), self.config_path.to_owned(), workspace_path.to_str().unwrap()).await;

            self.java_lsp.lock().unwrap().replace(lsp);
        }
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
Jsp-lsp written in Rust
Usage: jsp-lsp --stdio -p <PathToJdtLangServer> -c <PathToConfigDirectory>

Arguments:
    -p  Path to the jdt language server jar file
    -c  Path to the JDT LS config directory (e.g. config_linux)
"#;

pub enum ArgErrorType {
    DuplicateFlag,
    NoPathProvided,
    UnknownArgument,
    Help,
}

pub fn argument_error(error_type: ArgErrorType) {
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
    let (mut path, mut config_path) = ("", "");
    args.remove(0);
    if args.is_empty() {
        argument_error(ArgErrorType::Help);
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
                        argument_error(ArgErrorType::DuplicateFlag);
                    } else {
                        seen.insert(flag);
                    }
                    if let Some(value) = args.get(i + 1) {
                        is_read = true;
                        path = value;
                    } else {
                        argument_error(ArgErrorType::NoPathProvided);
                        return;
                    }
                }
                'c' => {
                    if seen.contains(&flag) {
                        argument_error(ArgErrorType::DuplicateFlag);
                    } else {
                        seen.insert(flag);
                    }
                    if let Some(value) = args.get(i + 1) {
                        is_read = true;
                        config_path = value;
                    } else {
                        argument_error(ArgErrorType::NoPathProvided);
                        return;
                    }
                }
                'h' => {
                    argument_error(ArgErrorType::Help);
                    return;
                }
                _ => {
                    argument_error(ArgErrorType::UnknownArgument);
                    return;
                }
            };
        } else {
            argument_error(ArgErrorType::UnknownArgument);
            return;
        }
    }

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { 
        path : path.into(),
        config_path: config_path.into(),
        client,
        java_lsp: Arc::new(Mutex::new(None)),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
