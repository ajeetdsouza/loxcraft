use anyhow::{Context, Result};
use lox_common::error::ErrorS;
use lox_common::types::Span;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, Position, Range, ServerCapabilities, ServerInfo,
    TextDocumentSyncKind,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
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
                ..Default::default()
            }),
        })
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let (mut program, mut errors) = lox_syntax::parse(&params.text_document.text);
        errors.extend(lox_interpreter::resolve(&mut program));
        let diagnostics = report_err(&params.text_document.text, &errors);

        self.client
            .publish_diagnostics(
                params.text_document.uri,
                diagnostics,
                Some(params.text_document.version),
            )
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let source = &params.content_changes.first().unwrap().text;
        let (mut program, mut errors) = lox_syntax::parse(source);
        errors.extend(lox_interpreter::resolve(&mut program));
        let diagnostics = report_err(source, &errors);

        self.client
            .publish_diagnostics(
                params.text_document.uri,
                diagnostics,
                Some(params.text_document.version),
            )
            .await;
    }
}

fn get_range(source: &str, span: &Span) -> Range {
    Range { start: get_position(source, span.start), end: get_position(source, span.end) }
}

fn get_position(source: &str, idx: usize) -> Position {
    let before = &source[..idx];
    let line = before.lines().count() - 1;
    // TODO: verify that lines always returns at least one element
    let character = before.lines().last().unwrap().len();
    Position { line: line as _, character: character as _ }
}

fn report_err(source: &str, errors: &[ErrorS]) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|(err, span)| Diagnostic {
            range: get_range(source, span),
            severity: Some(DiagnosticSeverity::ERROR),
            message: err.to_string(),
            ..Default::default()
        })
        .collect()
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
