use anyhow::{Context, Result};
use lox_common::error::ErrorS;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    MessageType, Position, Range, ServerCapabilities, ServerInfo,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities { ..Default::default() },
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                ..Default::default()
            }),
        })
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file opened").await;

        let (program, errors) = lox_syntax::parse(&params.text_document.text);
        if !errors.is_empty() {
            self.client
                .publish_diagnostics(
                    params.text_document.uri,
                    errors.iter().map(report_err).collect(),
                    Some(params.text_document.version),
                )
                .await;
        }
    }
}

fn report_err(err: &ErrorS) -> Diagnostic {
    Diagnostic {
        range: Range::new(Position { line: 0, character: 0 }, Position { line: 0, character: 0 }),
        severity: Some(DiagnosticSeverity::ERROR),
        ..Default::default()
    }
}

pub fn serve() -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to start async runtime")?
        .block_on(serve_async());
    Ok(())
}

async fn serve_async() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
