use std::cell::RefCell;
use std::io::Write;

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
    async fn validate_text(&self, uri: Url, text:String) {
        if text.contains("<%") {
            let diagnostics = validate_jsp_tags(&uri, &text);
            self.client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),  
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
        self.validate_text(params.text_document.uri.clone(), params.text_document.text.clone()).await;
        self.client.log_message(MessageType::INFO, params.text_document.text.clone()).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.client.log_message(MessageType::INFO, change.text.clone()).await;
            self.validate_text(params.text_document.uri, change.text.clone()).await;
        }
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
            CompletionItem::new_simple("Bye".to_string(), "More detail".to_string())
        ])))
    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(
                MarkedString::String("You're hovering!".to_string())
            ),
            range: None
        }))
    }
}

#[tokio::main]
async fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let mut file = match std::fs::File::create("../log.log") {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut flag_switch = false;

    for (i, mut arg) in args.iter_mut().enumerate() {
        if arg == "--stdio" || flag_switch {
            if flag_switch { flag_switch = false };
            continue;
        }

        if arg.starts_with("-") {
            flag_switch = true;
            let flag =arg.clone().remove(1).to_ascii_uppercase();
            if let Some(value) = args[i + 1] {
            match flag {
                'p' => file.write(format!("{}{}", value, "\n").as_bytes()),
               _ => continue, 
            };
            }
        
        }
    }


    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
