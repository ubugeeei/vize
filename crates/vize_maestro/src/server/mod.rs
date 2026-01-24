//! LSP server implementation.
//!
//! This module contains the core LSP server using tower-lsp.

mod capabilities;
mod state;

pub use capabilities::*;
pub use state::*;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::document::DocumentStore;
use crate::ide::{
    CodeActionService, CodeLensService, CompletionService, DefinitionService, DiagnosticService,
    DocumentLinkService, HoverService, IdeContext, InlayHintService, ReferencesService,
    RenameService, SemanticTokensService, WorkspaceSymbolsService,
};

/// The Maestro LSP server.
pub struct MaestroServer {
    /// LSP client for sending notifications
    client: Client,
    /// Server state
    state: ServerState,
}

impl MaestroServer {
    /// Create a new Maestro server instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: ServerState::new(),
        }
    }

    /// Get the document store.
    pub fn documents(&self) -> &DocumentStore {
        &self.state.documents
    }

    /// Publish diagnostics for a document.
    async fn publish_diagnostics(&self, uri: &Url) {
        // Use async version when native feature is enabled (includes tsgo diagnostics)
        #[cfg(feature = "native")]
        let diagnostics = DiagnosticService::collect_async(&self.state, uri).await;

        #[cfg(not(feature = "native"))]
        let diagnostics = DiagnosticService::collect(&self.state, uri);

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    /// Get block snippet completions (when outside all blocks)
    fn get_block_snippets(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "template".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Add template block".to_string()),
                insert_text: Some("<template>\n\t$1\n</template>".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "script setup".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Add script setup block".to_string()),
                insert_text: Some("<script setup lang=\"ts\">\n$1\n</script>".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "script".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Add script block".to_string()),
                insert_text: Some(
                    "<script lang=\"ts\">\nexport default {\n\t$1\n}\n</script>".to_string(),
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "style scoped".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Add scoped style block".to_string()),
                insert_text: Some("<style scoped>\n$1\n</style>".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "style".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Add style block".to_string()),
                insert_text: Some("<style>\n$1\n</style>".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }

    /// Get lint rule documentation for diagnostics at the given position.
    fn get_lint_hover_at_position(
        &self,
        uri: &Url,
        _content: &str,
        position: Position,
    ) -> Option<String> {
        // Get diagnostics for this document
        let diagnostics = DiagnosticService::collect(&self.state, uri);

        // Find lint diagnostics at this position
        let lint_diags: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                // Check if position is within diagnostic range
                let in_range = position.line >= d.range.start.line
                    && position.line <= d.range.end.line
                    && (position.line != d.range.start.line
                        || position.character >= d.range.start.character)
                    && (position.line != d.range.end.line
                        || position.character <= d.range.end.character);

                // Only include lint diagnostics (from patina)
                let is_lint = d
                    .source
                    .as_ref()
                    .is_some_and(|s| s == "vize/lint" || s == "vize/musea");

                in_range && is_lint
            })
            .collect();

        if lint_diags.is_empty() {
            return None;
        }

        // Format lint info as markdown
        let mut markdown = String::new();

        for diag in lint_diags {
            let severity_icon = match diag.severity {
                Some(DiagnosticSeverity::ERROR) => "🔴",
                Some(DiagnosticSeverity::WARNING) => "🟡",
                Some(DiagnosticSeverity::INFORMATION) => "🔵",
                Some(DiagnosticSeverity::HINT) => "💡",
                _ => "⚪",
            };

            // Rule name as header
            if let Some(NumberOrString::String(ref rule)) = diag.code {
                markdown.push_str(&format!("### {} {}\n\n", severity_icon, rule));
            }

            // Message (split into message and help if present)
            let parts: Vec<&str> = diag.message.split("\n\nHelp: ").collect();
            markdown.push_str(parts[0]);
            markdown.push_str("\n\n");

            // Help text (if present)
            if parts.len() > 1 {
                markdown.push_str(&format!("**Help:** {}\n\n", parts[1]));
            }

            // Link to documentation
            if let Some(ref code_desc) = diag.code_description {
                markdown.push_str(&format!(
                    "[📖 View rule documentation]({})\n\n",
                    code_desc.href
                ));
            }

            markdown.push_str("---\n\n");
        }

        // Remove trailing separator
        if markdown.ends_with("---\n\n") {
            markdown.truncate(markdown.len() - 5);
        }

        Some(markdown)
    }

    /// Merge hover content with lint information.
    fn merge_hover_with_lint(hover: Option<Hover>, lint_info: String) -> Hover {
        match hover {
            Some(mut h) => {
                // Append lint info to existing hover content
                if let HoverContents::Markup(ref mut markup) = h.contents {
                    markup.value.push_str("\n\n---\n\n");
                    markup.value.push_str(&lint_info);
                }
                h
            }
            None => {
                // Create new hover with just lint info
                Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: lint_info,
                    }),
                    range: None,
                }
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for MaestroServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Set workspace root from root_uri or workspace_folders
        #[cfg(feature = "native")]
        {
            if let Some(root_uri) = params.root_uri.as_ref() {
                if let Ok(path) = root_uri.to_file_path() {
                    tracing::info!("Setting workspace root: {:?}", path);
                    self.state.set_workspace_root(path);
                }
            } else if let Some(folders) = params.workspace_folders.as_ref() {
                if let Some(folder) = folders.first() {
                    if let Ok(path) = folder.uri.to_file_path() {
                        tracing::info!("Setting workspace root from folder: {:?}", path);
                        self.state.set_workspace_root(path);
                    }
                }
            }
        }

        Ok(InitializeResult {
            capabilities: server_capabilities(),
            server_info: Some(ServerInfo {
                name: "vize-maestro".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "vize_maestro LSP server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;
        let language_id = params.text_document.language_id;

        self.state
            .documents
            .open(uri.clone(), content.clone(), version, language_id);

        // Generate virtual documents for the SFC
        self.state.update_virtual_docs(&uri, &content);

        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        self.state
            .documents
            .apply_changes(&uri, params.content_changes, version);

        // Regenerate virtual documents with updated content
        if let Some(doc) = self.state.documents.get(&uri) {
            let content = doc.text();
            self.state.update_virtual_docs(&uri, &content);
        }

        self.publish_diagnostics(&uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.publish_diagnostics(&uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.state.documents.close(&uri);

        // Clean up virtual documents cache
        self.state.remove_virtual_docs(&uri);

        // Clear diagnostics
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();

        // Convert LSP position to byte offset
        let offset =
            crate::utils::position_to_offset_str(&content, position.line, position.character);

        // Use IdeContext and HoverService for context-aware hover
        let mut hover_result: Option<Hover> = None;

        if let Some(ctx) = IdeContext::new(&self.state, uri, offset) {
            // Try tsgo-based hover first (when native feature is enabled)
            #[cfg(feature = "native")]
            {
                let tsgo_bridge = self.state.get_tsgo_bridge().await;
                hover_result = HoverService::hover_with_tsgo(&ctx, tsgo_bridge).await;
            }

            // Fallback to sync hover
            #[cfg(not(feature = "native"))]
            {
                hover_result = HoverService::hover(&ctx);
            }
        }

        // Check for lint diagnostics at this position and add info to hover
        let lint_hover = self.get_lint_hover_at_position(uri, &content, position);
        if let Some(lint_info) = lint_hover {
            hover_result = Some(Self::merge_hover_with_lint(hover_result, lint_info));
        }

        Ok(hover_result)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let offset =
            crate::utils::position_to_offset_str(&content, position.line, position.character);

        // Use IdeContext and CompletionService for context-aware completions
        if let Some(ctx) = IdeContext::new(&self.state, uri, offset) {
            if let Some(response) = CompletionService::complete(&ctx) {
                return Ok(Some(response));
            }
        }

        // Fallback: offer block snippets if we can't determine context
        let items = self.get_block_snippets();
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn completion_resolve(&self, item: CompletionItem) -> Result<CompletionItem> {
        // Return the item as-is for now
        // TODO: Add documentation, additional text edits, etc.
        Ok(item)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let offset =
            crate::utils::position_to_offset_str(&content, position.line, position.character);

        // Use IdeContext and DefinitionService for go-to-definition
        if let Some(ctx) = IdeContext::new(&self.state, uri, offset) {
            // Try tsgo-based definition first (when native feature is enabled)
            #[cfg(feature = "native")]
            {
                let tsgo_bridge = self.state.get_tsgo_bridge().await;
                if let Some(response) =
                    DefinitionService::definition_with_tsgo(&ctx, tsgo_bridge).await
                {
                    return Ok(Some(response));
                }
            }

            // Fallback to sync definition
            #[cfg(not(feature = "native"))]
            if let Some(response) = DefinitionService::definition(&ctx) {
                return Ok(Some(response));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let offset =
            crate::utils::position_to_offset_str(&content, position.line, position.character);

        // Use IdeContext and ReferencesService for find-all-references
        if let Some(ctx) = IdeContext::new(&self.state, uri, offset) {
            if let Some(locations) = ReferencesService::references(&ctx, include_declaration) {
                return Ok(Some(locations));
            }
        }

        Ok(None)
    }

    #[allow(deprecated)] // DocumentSymbol.deprecated is deprecated in favor of tags
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&content, options) else {
            return Ok(None);
        };

        let mut symbols = Vec::new();

        // Add template symbol
        if let Some(ref template) = descriptor.template {
            symbols.push(DocumentSymbol {
                name: "template".to_string(),
                kind: SymbolKind::MODULE,
                tags: None,
                deprecated: None,
                range: Range {
                    start: Position {
                        line: template.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: template.loc.end_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: template.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: template.loc.start_line.saturating_sub(1) as u32,
                        character: 10,
                    },
                },
                detail: template.lang.as_ref().map(|l| l.to_string()),
                children: None,
            });
        }

        // Add script symbol
        if let Some(ref script) = descriptor.script {
            symbols.push(DocumentSymbol {
                name: "script".to_string(),
                kind: SymbolKind::MODULE,
                tags: None,
                deprecated: None,
                range: Range {
                    start: Position {
                        line: script.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: script.loc.end_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: script.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: script.loc.start_line.saturating_sub(1) as u32,
                        character: 8,
                    },
                },
                detail: script.lang.as_ref().map(|l| l.to_string()),
                children: None,
            });
        }

        // Add script setup symbol
        if let Some(ref script_setup) = descriptor.script_setup {
            symbols.push(DocumentSymbol {
                name: "script setup".to_string(),
                kind: SymbolKind::MODULE,
                tags: None,
                deprecated: None,
                range: Range {
                    start: Position {
                        line: script_setup.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: script_setup.loc.end_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: script_setup.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: script_setup.loc.start_line.saturating_sub(1) as u32,
                        character: 14,
                    },
                },
                detail: script_setup.lang.as_ref().map(|l| l.to_string()),
                children: None,
            });
        }

        // Add style symbols
        for (i, style) in descriptor.styles.iter().enumerate() {
            let name = if let Some(ref module) = style.module {
                format!("style module={}", module)
            } else if style.scoped {
                "style scoped".to_string()
            } else {
                format!("style[{}]", i)
            };

            symbols.push(DocumentSymbol {
                name,
                kind: SymbolKind::MODULE,
                tags: None,
                deprecated: None,
                range: Range {
                    start: Position {
                        line: style.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: style.loc.end_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: style.loc.start_line.saturating_sub(1) as u32,
                        character: 0,
                    },
                    end: Position {
                        line: style.loc.start_line.saturating_sub(1) as u32,
                        character: 7,
                    },
                },
                detail: style.lang.as_ref().map(|l| l.to_string()),
                children: None,
            });
        }

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let range = params.range;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();

        // Create IDE context at start of range
        let offset =
            crate::utils::position_to_offset_str(&content, range.start.line, range.start.character);

        if let Some(ctx) = IdeContext::new(&self.state, uri, offset) {
            let actions = CodeActionService::code_actions(&ctx, range);
            if !actions.is_empty() {
                return Ok(Some(actions));
            }
        }

        Ok(None)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let position = params.position;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let offset =
            crate::utils::position_to_offset_str(&content, position.line, position.character);

        if let Some(ctx) = IdeContext::new(&self.state, uri, offset) {
            return Ok(RenameService::prepare_rename(&ctx));
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = &params.new_name;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let offset =
            crate::utils::position_to_offset_str(&content, position.line, position.character);

        if let Some(ctx) = IdeContext::new(&self.state, uri, offset) {
            return Ok(RenameService::rename(&ctx, new_name));
        }

        Ok(None)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        Ok(SemanticTokensService::get_tokens(&content, uri))
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = &params.text_document.uri;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let lenses = CodeLensService::get_lenses(&content, uri);

        if lenses.is_empty() {
            Ok(None)
        } else {
            Ok(Some(lenses))
        }
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = &params.query;
        let symbols = WorkspaceSymbolsService::search(&self.state, query);

        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(symbols))
        }
    }

    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        let uri = &params.text_document.uri;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let links = DocumentLinkService::get_links(&content, uri);

        if links.is_empty() {
            Ok(None)
        } else {
            Ok(Some(links))
        }
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let range = params.range;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let hints = InlayHintService::get_hints(&content, uri, range);

        if hints.is_empty() {
            Ok(None)
        } else {
            Ok(Some(hints))
        }
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = &params.text_document.uri;

        let Some(doc) = self.state.documents.get(uri) else {
            return Ok(None);
        };

        let content = doc.text();
        let mut ranges = Vec::new();

        // Parse SFC to get block ranges
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        if let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&content, options) {
            // Template block
            if let Some(ref template) = descriptor.template {
                if template.loc.start_line < template.loc.end_line {
                    ranges.push(FoldingRange {
                        start_line: template.loc.start_line.saturating_sub(1) as u32,
                        start_character: None,
                        end_line: template.loc.end_line.saturating_sub(1) as u32,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("template".to_string()),
                    });
                }
            }

            // Script setup block
            if let Some(ref script) = descriptor.script_setup {
                if script.loc.start_line < script.loc.end_line {
                    ranges.push(FoldingRange {
                        start_line: script.loc.start_line.saturating_sub(1) as u32,
                        start_character: None,
                        end_line: script.loc.end_line.saturating_sub(1) as u32,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("script setup".to_string()),
                    });
                }
            }

            // Script block
            if let Some(ref script) = descriptor.script {
                if script.loc.start_line < script.loc.end_line {
                    ranges.push(FoldingRange {
                        start_line: script.loc.start_line.saturating_sub(1) as u32,
                        start_character: None,
                        end_line: script.loc.end_line.saturating_sub(1) as u32,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("script".to_string()),
                    });
                }
            }

            // Style blocks
            for style in &descriptor.styles {
                if style.loc.start_line < style.loc.end_line {
                    ranges.push(FoldingRange {
                        start_line: style.loc.start_line.saturating_sub(1) as u32,
                        start_character: None,
                        end_line: style.loc.end_line.saturating_sub(1) as u32,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("style".to_string()),
                    });
                }
            }
        }

        if ranges.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ranges))
        }
    }
}
