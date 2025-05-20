mod java_backend;

use java_backend::java_lsp_connections::{JavaLspConnection, JavaLspMethod};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug, Clone)]
pub struct Backend {
    client: Client,
    java_lsp: Arc<Mutex<Option<JavaLspConnection>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, init_params: InitializeParams) -> Result<InitializeResult> {
        let workspace_path = {
            let root_path = {
                if let Some(res) = init_params.root_uri {
                    res.to_file_path().unwrap()
                } else {
                    std::env::temp_dir().join("jsp-lsp-fallback-workspace")
                }
            };

            // Universal cache dir
            let cache_dir = dirs::cache_dir().unwrap();
            let base_dir = cache_dir.join("jsp-lsp/jdtls/workspaces");

            let escaped = root_path
                .to_string_lossy()
                .replace("/", "_")
                .replace("\\", "_");

            let ws_path = base_dir.join(escaped);

            std::fs::create_dir_all(&ws_path).ok().unwrap();

            match ws_path.to_str() {
                Some(k) => k.to_owned(),
                None => {
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            "Unable to conver the workspace path to string.",
                        )
                        .await;
                    panic!("Unable to conver the workspace path to string.")
                }
            }
        };

        let lsp = JavaLspConnection::new(&self.client, workspace_path.as_str()).await;
        match lsp {
            Ok(res) => {
                self.java_lsp.lock().await.replace(res);
                self.client
                    .log_message(MessageType::INFO, "Java LSP succesfully initiated.")
                    .await;
            }
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, e.to_string())
                    .await
            }
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
            JavaLspMethod::DidOpen,
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
            self.validate_text(
                params.text_document.uri,
                change.text.clone(),
                JavaLspMethod::DidChange,
            )
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

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        java_lsp: Arc::new(Mutex::new(None)),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
