#![cfg(feature = "lsp")]

use anyhow::{Context, Result};
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, Position, Range, ServerCapabilities, ServerInfo,
    TextDocumentSyncKind,
};
use tower_lsp::{jsonrpc, Client, LanguageServer, LspService, Server};

use crate::types::Span;
use crate::vm::{Compiler, Gc};

#[derive(Debug)]
struct Backend {
    client: Client,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn get_diagnostics(&self, source: &str) -> Vec<Diagnostic> {
        let mut gc = Gc::default();
        Compiler::compile(source, 0, &mut gc)
            .err()
            .unwrap_or_default()
            .iter()
            .map(|(err, span)| Diagnostic {
                range: get_range(source, span),
                severity: Some(DiagnosticSeverity::ERROR),
                message: err.to_string(),
                ..Default::default()
            })
            .collect()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncKind::FULL.into()),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let source = &params.text_document.text;
        let uri = params.text_document.uri;
        let version = Some(params.text_document.version);
        let diagnostics = self.get_diagnostics(source);
        self.client.publish_diagnostics(uri, diagnostics, version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let source = &params.content_changes.first().unwrap().text;
        let uri = params.text_document.uri;
        let version = Some(params.text_document.version);
        let diagnostics = self.get_diagnostics(source);
        self.client.publish_diagnostics(uri, diagnostics, version).await;
    }
}

fn get_range(source: &str, span: &Span) -> Range {
    Range { start: get_position(source, span.start), end: get_position(source, span.end) }
}

fn get_position(source: &str, idx: usize) -> Position {
    let before = &source[..idx];
    let line = before.lines().count() - 1;
    let character = before.lines().last().unwrap().len();
    Position { line: line as _, character: character as _ }
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

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
