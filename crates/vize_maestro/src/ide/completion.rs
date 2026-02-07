//! Completion provider for Vue SFC files.
//!
//! Provides context-aware completions for:
//! - Template expressions and directives
//! - Script bindings and imports
//! - CSS properties and Vue-specific selectors
//! - Real completions from tsgo (when available)
//!
//! Uses vize_croquis for accurate scope analysis and type information.

use std::sync::Arc;

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionResponse,
    Documentation, InsertTextFormat, MarkupContent, MarkupKind,
};
use vize_croquis::{Analyzer, AnalyzerOptions};
use vize_relief::BindingType;

#[cfg(feature = "native")]
use vize_canon::{LspCompletionItem, LspDocumentation, TsgoBridge};

use super::IdeContext;
use crate::virtual_code::BlockType;

/// Completion service for providing context-aware completions.
pub struct CompletionService;

impl CompletionService {
    /// Get completions for the given context.
    pub fn complete(ctx: &IdeContext) -> Option<CompletionResponse> {
        // Check if this is an Art file
        if ctx.uri.path().ends_with(".art.vue") {
            return Self::complete_art(ctx);
        }

        // Check if cursor is inside <art> block in a regular .vue file
        if matches!(ctx.block_type, Some(BlockType::Art(_))) {
            return Self::complete_inline_art(ctx);
        }

        let items = match ctx.block_type? {
            BlockType::Template => Self::complete_template(ctx),
            BlockType::Script => Self::complete_script(ctx, false),
            BlockType::ScriptSetup => Self::complete_script(ctx, true),
            BlockType::Style(index) => Self::complete_style(ctx, index),
            BlockType::Art(_) => unreachable!(), // handled above
        };

        if items.is_empty() {
            None
        } else {
            Some(CompletionResponse::Array(items))
        }
    }

    /// Get completions with tsgo support (async version).
    #[cfg(feature = "native")]
    pub async fn complete_with_tsgo(
        ctx: &IdeContext<'_>,
        tsgo_bridge: Option<Arc<TsgoBridge>>,
    ) -> Option<CompletionResponse> {
        // Check if this is an Art file
        if ctx.uri.path().ends_with(".art.vue") {
            return Self::complete_art(ctx);
        }

        // Check if cursor is inside <art> block in a regular .vue file
        if matches!(ctx.block_type, Some(BlockType::Art(_))) {
            return Self::complete_inline_art(ctx);
        }

        let block_type = ctx.block_type?;

        // Try tsgo completion first
        if let Some(bridge) = tsgo_bridge {
            let tsgo_items = match block_type {
                BlockType::Template => Self::complete_template_with_tsgo(ctx, &bridge).await,
                BlockType::Script => Self::complete_script_with_tsgo(ctx, false, &bridge).await,
                BlockType::ScriptSetup => Self::complete_script_with_tsgo(ctx, true, &bridge).await,
                BlockType::Style(_) => vec![],
                BlockType::Art(_) => unreachable!(), // handled above
            };

            if !tsgo_items.is_empty() {
                // Merge tsgo items with static completions
                let mut items = tsgo_items;
                items.extend(match block_type {
                    BlockType::Template => Self::directive_completions(),
                    BlockType::Script => Self::composition_api_completions(),
                    BlockType::ScriptSetup => {
                        let mut v = Self::composition_api_completions();
                        v.extend(Self::macro_completions());
                        v
                    }
                    BlockType::Style(_) => Self::vue_css_completions(),
                    BlockType::Art(_) => unreachable!(), // handled above
                });

                return Some(CompletionResponse::Array(items));
            }
        }

        // Fall back to synchronous completions
        Self::complete(ctx)
    }

    /// Get completions for template with tsgo.
    #[cfg(feature = "native")]
    async fn complete_template_with_tsgo(
        ctx: &IdeContext<'_>,
        bridge: &TsgoBridge,
    ) -> Vec<CompletionItem> {
        if let Some(ref virtual_docs) = ctx.virtual_docs {
            if let Some(ref template) = virtual_docs.template {
                if let Some(vts_offset) =
                    crate::ide::hover::HoverService::sfc_to_virtual_ts_offset(ctx, ctx.offset)
                {
                    let (line, character) =
                        super::offset_to_position(&template.content, vts_offset);
                    let uri = format!("vize-virtual://{}.template.ts", ctx.uri.path());

                    if bridge.is_initialized() {
                        let _ = bridge
                            .open_or_update_virtual_document(
                                &format!("{}.template.ts", ctx.uri.path()),
                                &template.content,
                            )
                            .await;

                        if let Ok(items) = bridge.completion(&uri, line, character).await {
                            return items
                                .into_iter()
                                .map(Self::convert_lsp_completion)
                                .collect();
                        }
                    }
                }
            }
        }

        vec![]
    }

    /// Get completions for script with tsgo.
    #[cfg(feature = "native")]
    async fn complete_script_with_tsgo(
        ctx: &IdeContext<'_>,
        is_setup: bool,
        bridge: &TsgoBridge,
    ) -> Vec<CompletionItem> {
        if let Some(ref virtual_docs) = ctx.virtual_docs {
            let script_doc = if is_setup {
                virtual_docs.script_setup.as_ref()
            } else {
                virtual_docs.script.as_ref()
            };

            if let Some(script) = script_doc {
                if let Some(vts_offset) =
                    crate::ide::hover::HoverService::sfc_to_virtual_ts_script_offset(
                        ctx, ctx.offset,
                    )
                {
                    let (line, character) = super::offset_to_position(&script.content, vts_offset);
                    let suffix = if is_setup { "setup.ts" } else { "script.ts" };
                    let uri = format!("vize-virtual://{}.{}", ctx.uri.path(), suffix);

                    if bridge.is_initialized() {
                        let _ = bridge
                            .open_or_update_virtual_document(
                                &format!("{}.{}", ctx.uri.path(), suffix),
                                &script.content,
                            )
                            .await;

                        if let Ok(items) = bridge.completion(&uri, line, character).await {
                            return items
                                .into_iter()
                                .map(Self::convert_lsp_completion)
                                .collect();
                        }
                    }
                }
            }
        }

        vec![]
    }

    /// Convert tsgo LspCompletionItem to tower-lsp CompletionItem.
    #[cfg(feature = "native")]
    fn convert_lsp_completion(item: LspCompletionItem) -> CompletionItem {
        CompletionItem {
            label: item.label,
            kind: item.kind.map(Self::convert_completion_kind),
            detail: item.detail,
            documentation: item.documentation.map(|doc| match doc {
                LspDocumentation::String(s) => Documentation::String(s),
                LspDocumentation::Markup(m) => Documentation::MarkupContent(MarkupContent {
                    kind: if m.kind == "markdown" {
                        MarkupKind::Markdown
                    } else {
                        MarkupKind::PlainText
                    },
                    value: m.value,
                }),
            }),
            insert_text: item.insert_text,
            insert_text_format: item.insert_text_format.map(|f| {
                if f == 2 {
                    InsertTextFormat::SNIPPET
                } else {
                    InsertTextFormat::PLAIN_TEXT
                }
            }),
            filter_text: item.filter_text,
            sort_text: item.sort_text,
            ..Default::default()
        }
    }

    /// Convert LSP completion item kind number to CompletionItemKind.
    #[cfg(feature = "native")]
    fn convert_completion_kind(kind: u32) -> CompletionItemKind {
        match kind {
            1 => CompletionItemKind::TEXT,
            2 => CompletionItemKind::METHOD,
            3 => CompletionItemKind::FUNCTION,
            4 => CompletionItemKind::CONSTRUCTOR,
            5 => CompletionItemKind::FIELD,
            6 => CompletionItemKind::VARIABLE,
            7 => CompletionItemKind::CLASS,
            8 => CompletionItemKind::INTERFACE,
            9 => CompletionItemKind::MODULE,
            10 => CompletionItemKind::PROPERTY,
            11 => CompletionItemKind::UNIT,
            12 => CompletionItemKind::VALUE,
            13 => CompletionItemKind::ENUM,
            14 => CompletionItemKind::KEYWORD,
            15 => CompletionItemKind::SNIPPET,
            16 => CompletionItemKind::COLOR,
            17 => CompletionItemKind::FILE,
            18 => CompletionItemKind::REFERENCE,
            19 => CompletionItemKind::FOLDER,
            20 => CompletionItemKind::ENUM_MEMBER,
            21 => CompletionItemKind::CONSTANT,
            22 => CompletionItemKind::STRUCT,
            23 => CompletionItemKind::EVENT,
            24 => CompletionItemKind::OPERATOR,
            25 => CompletionItemKind::TYPE_PARAMETER,
            _ => CompletionItemKind::TEXT,
        }
    }

    /// Get completions for Art files (*.art.vue).
    fn complete_art(ctx: &IdeContext) -> Option<CompletionResponse> {
        let mut items = Vec::new();

        // Get the content and determine context
        let content = &ctx.content;
        let offset = ctx.offset;

        // Determine if we're inside <art>, <variant>, or at root level
        let before_cursor = &content[..offset.min(content.len())];

        if is_inside_art_tag(before_cursor) {
            // Inside <art> opening tag - suggest attributes
            items.extend(Self::art_attribute_completions());
        } else if is_inside_variant_tag(before_cursor) {
            // Inside <variant> opening tag - suggest attributes
            items.extend(Self::variant_attribute_completions());
        } else if should_suggest_art_block(before_cursor) {
            // At root level - suggest <art> block
            items.extend(Self::art_block_completions());
        } else if should_suggest_variant_block(before_cursor) {
            // Inside <art> content - suggest <variant> block
            items.extend(Self::variant_block_completions());
        }

        // Also add script and style block completions
        items.extend(Self::art_script_completions());

        if items.is_empty() {
            None
        } else {
            Some(CompletionResponse::Array(items))
        }
    }

    /// Get completions for inline <art> blocks in regular .vue files.
    fn complete_inline_art(ctx: &IdeContext) -> Option<CompletionResponse> {
        let mut items = Vec::new();

        let content = &ctx.content;
        let offset = ctx.offset;
        let before_cursor = &content[..offset.min(content.len())];

        if is_inside_art_tag(before_cursor) {
            items.extend(Self::art_attribute_completions());
        } else if is_inside_variant_tag(before_cursor) {
            items.extend(Self::variant_attribute_completions());
        } else if should_suggest_variant_block(before_cursor) {
            items.extend(Self::variant_block_completions());
            // Inside variant content, suggest <Self> for referencing the host component
            items.push(Self::self_component_completion());
        }

        if items.is_empty() {
            None
        } else {
            Some(CompletionResponse::Array(items))
        }
    }

    /// Completion item for <Self> component reference in inline art blocks.
    fn self_component_completion() -> CompletionItem {
        CompletionItem {
            label: "Self".to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some("Reference to the host component".to_string()),
            insert_text: Some("<Self $1>$0</Self>".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**`<Self>`**\n\nReferences the host component in inline art blocks.\nReplaced with the component name at build time.".to_string(),
            })),
            ..Default::default()
        }
    }

    /// Art block completions at root level.
    fn art_block_completions() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "art".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Create Art block".to_string()),
                insert_text: Some(
                    "<art title=\"$1\" component=\"$2\">\n\t<variant name=\"$3\" default>\n\t\t$0\n\t</variant>\n</art>".to_string()
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**Art Block**\n\nDefines a component gallery entry with metadata and variants.\n\n```vue\n<art title=\"Button\" component=\"./Button.vue\">\n  <variant name=\"Primary\" default>\n    <Button>Click</Button>\n  </variant>\n</art>\n```".to_string(),
                })),
                ..Default::default()
            },
        ]
    }

    /// Art attribute completions inside <art> tag.
    fn art_attribute_completions() -> Vec<CompletionItem> {
        vec![
            Self::attr_item("title", "Component title (required)", "title=\"$1\""),
            Self::attr_item("component", "Path to component file", "component=\"$1\""),
            Self::attr_item("description", "Component description", "description=\"$1\""),
            Self::attr_item(
                "category",
                "Component category (e.g., atoms, molecules)",
                "category=\"$1\"",
            ),
            Self::attr_item("tags", "Comma-separated tags", "tags=\"$1\""),
            Self::attr_item(
                "status",
                "Component status (ready, draft, deprecated)",
                "status=\"$1\"",
            ),
            Self::attr_item("order", "Display order in gallery", "order=\"$1\""),
        ]
    }

    /// Variant block completions inside <art>.
    fn variant_block_completions() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "variant".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Create variant block".to_string()),
                insert_text: Some(
                    "<variant name=\"$1\">\n\t$0\n</variant>".to_string()
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "**Variant Block**\n\nDefines a component variation with specific props.\n\n```vue\n<variant name=\"Primary\" default>\n  <Button variant=\"primary\">Click</Button>\n</variant>\n```".to_string(),
                })),
                ..Default::default()
            },
            CompletionItem {
                label: "variant with args".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Create variant with args".to_string()),
                insert_text: Some(
                    "<variant name=\"$1\" args='{\"$2\": $3}'>\n\t$0\n</variant>".to_string()
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }

    /// Variant attribute completions inside <variant> tag.
    fn variant_attribute_completions() -> Vec<CompletionItem> {
        vec![
            Self::attr_item("name", "Variant name (required)", "name=\"$1\""),
            Self::attr_item("default", "Mark as default variant", "default"),
            Self::attr_item("args", "Props as JSON", "args='{\"$1\": $2}'"),
            Self::attr_item(
                "viewport",
                "Viewport dimensions (WxH or WxH@scale)",
                "viewport=\"$1\"",
            ),
            Self::attr_item("skip-vrt", "Skip visual regression test", "skip-vrt"),
        ]
    }

    /// Create an attribute completion item.
    fn attr_item(label: &str, description: &str, snippet: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(description.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }
    }

    /// Script block completions for Art files.
    fn art_script_completions() -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "script setup".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Add script setup block".to_string()),
                insert_text: Some(
                    "<script setup lang=\"ts\">\nimport $1 from '$2'\n</script>".to_string(),
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
            CompletionItem {
                label: "style".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Add style block".to_string()),
                insert_text: Some("<style scoped>\n$0\n</style>".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ]
    }

    /// Get completions for template context.
    fn complete_template(ctx: &IdeContext) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Add Vue directives
        items.extend(Self::directive_completions());

        // Add built-in components
        items.extend(Self::builtin_component_completions());

        // Use vize_croquis for accurate scope analysis and type information
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        if let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&ctx.content, options) {
            // Analyze script setup with croquis
            if let Some(ref script_setup) = descriptor.script_setup {
                let mut analyzer = Analyzer::with_options(AnalyzerOptions {
                    analyze_script: true,
                    ..Default::default()
                });
                analyzer.analyze_script_setup(&script_setup.content);
                let croquis = analyzer.finish();

                // Add bindings with accurate type information
                for (name, binding_type) in croquis.bindings.iter() {
                    let (kind, type_detail, doc) =
                        Self::binding_type_to_completion_info(binding_type);
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(kind),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: Some(type_detail.clone()),
                            description: None,
                        }),
                        detail: Some(type_detail),
                        documentation: Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: doc,
                        })),
                        sort_text: Some(format!("0{}", name)), // Prioritize user bindings
                        ..Default::default()
                    });
                }

                // Add props with type information
                for prop in croquis.macros.props() {
                    let prop_type = prop
                        .prop_type
                        .as_ref()
                        .map(|t| t.as_str())
                        .unwrap_or("unknown");
                    let required = if prop.required { "" } else { "?" };

                    items.push(CompletionItem {
                        label: prop.name.to_string(),
                        kind: Some(CompletionItemKind::PROPERTY),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: Some(format!(": {}{}", prop_type, required)),
                            description: None,
                        }),
                        detail: Some(format!("prop: {}", prop_type)),
                        documentation: Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!(
                                "**Prop** `{}`\n\n```typescript\n{}: {}{}\n```\n\n{}",
                                prop.name,
                                prop.name,
                                prop_type,
                                if prop.required { "" } else { " // optional" },
                                if prop.default_value.is_some() {
                                    "Has default value"
                                } else {
                                    ""
                                }
                            ),
                        })),
                        sort_text: Some(format!("0{}", prop.name)), // Prioritize props
                        ..Default::default()
                    });
                }

                // Add reactive sources with special handling
                for source in croquis.reactivity.sources() {
                    let kind_str = source.kind.to_display();
                    items.push(CompletionItem {
                        label: source.name.to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: Some(format!(" ({})", kind_str)),
                            description: None,
                        }),
                        detail: Some(format!("Reactive: {}", kind_str)),
                        documentation: Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!(
                                "**{}** `{}`\n\n{}\n\nAuto-unwrapped in template.",
                                kind_str,
                                source.name,
                                if source.kind.needs_value_access() {
                                    "Needs `.value` in script"
                                } else {
                                    "Direct access (no `.value` needed)"
                                }
                            ),
                        })),
                        sort_text: Some(format!("0{}", source.name)),
                        ..Default::default()
                    });
                }
            }
        }

        // Add common template snippets
        items.extend(Self::template_snippets());

        items
    }

    /// Convert BindingType to completion item information.
    fn binding_type_to_completion_info(
        binding_type: BindingType,
    ) -> (CompletionItemKind, String, String) {
        match binding_type {
            BindingType::SetupRef => (
                CompletionItemKind::VARIABLE,
                " (ref)".to_string(),
                "**Ref**\n\nReactive reference. Auto-unwrapped in template, needs `.value` in script.".to_string(),
            ),
            BindingType::SetupMaybeRef => (
                CompletionItemKind::VARIABLE,
                " (maybeRef)".to_string(),
                "**MaybeRef**\n\nPossibly a ref (from toRef/toRefs). Auto-unwrapped in template.".to_string(),
            ),
            BindingType::SetupReactiveConst => (
                CompletionItemKind::VARIABLE,
                " (reactive)".to_string(),
                "**Reactive**\n\nReactive object. Direct access without `.value`.".to_string(),
            ),
            BindingType::SetupConst => (
                CompletionItemKind::CONSTANT,
                " (const)".to_string(),
                "**Const**\n\nConstant binding (function, class, or literal).".to_string(),
            ),
            BindingType::SetupLet => (
                CompletionItemKind::VARIABLE,
                " (let)".to_string(),
                "**Let**\n\nMutable variable.".to_string(),
            ),
            BindingType::Props => (
                CompletionItemKind::PROPERTY,
                " (prop)".to_string(),
                "**Prop**\n\nComponent property from defineProps.".to_string(),
            ),
            BindingType::PropsAliased => (
                CompletionItemKind::PROPERTY,
                " (prop alias)".to_string(),
                "**Aliased Prop**\n\nDestructured prop with alias.".to_string(),
            ),
            BindingType::Data => (
                CompletionItemKind::VARIABLE,
                " (data)".to_string(),
                "**Data**\n\nReactive data property (Options API).".to_string(),
            ),
            BindingType::Options => (
                CompletionItemKind::METHOD,
                " (options)".to_string(),
                "**Options**\n\nComputed or method (Options API).".to_string(),
            ),
            BindingType::LiteralConst => (
                CompletionItemKind::CONSTANT,
                " (literal)".to_string(),
                "**Literal**\n\nLiteral constant value.".to_string(),
            ),
            BindingType::ExternalModule => (
                CompletionItemKind::MODULE,
                " (import)".to_string(),
                "**Import**\n\nImported from external module.".to_string(),
            ),
            BindingType::VueGlobal => (
                CompletionItemKind::VARIABLE,
                " (vue)".to_string(),
                "**Vue Global**\n\nVue global ($refs, $emit, etc.).".to_string(),
            ),
            _ => (
                CompletionItemKind::VARIABLE,
                "".to_string(),
                "Binding from script.".to_string(),
            ),
        }
    }

    /// Get completions for script context.
    fn complete_script(ctx: &IdeContext, is_setup: bool) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Add Vue Composition API
        items.extend(Self::composition_api_completions());

        // Add Vue macros (script setup only)
        if is_setup {
            items.extend(Self::macro_completions());
        }

        // Add common imports
        items.extend(Self::import_completions());

        // Use vize_croquis for accurate bindings in script
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        if let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&ctx.content, options) {
            let script = if is_setup {
                descriptor.script_setup.as_ref()
            } else {
                descriptor.script.as_ref()
            };

            if let Some(script) = script {
                let mut analyzer = Analyzer::with_options(AnalyzerOptions {
                    analyze_script: true,
                    ..Default::default()
                });

                if is_setup {
                    analyzer.analyze_script_setup(&script.content);
                } else {
                    analyzer.analyze_script_plain(&script.content);
                }

                let croquis = analyzer.finish();

                // Add bindings with type information
                for (name, binding_type) in croquis.bindings.iter() {
                    let (kind, type_detail, doc) =
                        Self::binding_type_to_completion_info(binding_type);

                    // For refs in script, add .value hint
                    let needs_value = matches!(
                        binding_type,
                        BindingType::SetupRef | BindingType::SetupMaybeRef
                    );

                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(kind),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: Some(type_detail.clone()),
                            description: if needs_value {
                                Some(".value".to_string())
                            } else {
                                None
                            },
                        }),
                        detail: Some(type_detail),
                        documentation: Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: doc,
                        })),
                        sort_text: Some(format!("0{}", name)),
                        ..Default::default()
                    });
                }

                // Add reactive sources
                for source in croquis.reactivity.sources() {
                    let needs_value = source.kind.needs_value_access();
                    let kind_str = source.kind.to_display();

                    items.push(CompletionItem {
                        label: source.name.to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: Some(kind_str.to_string()),
                            description: if needs_value {
                                Some(".value".to_string())
                            } else {
                                None
                            },
                        }),
                        detail: Some(kind_str.to_string()),
                        documentation: Some(Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: if needs_value {
                                "Needs `.value` access in script.".to_string()
                            } else {
                                "Direct access (no `.value` needed).".to_string()
                            },
                        })),
                        sort_text: Some(format!("0{}", source.name)),
                        ..Default::default()
                    });
                }
            }
        }

        items
    }

    /// Get completions for style context.
    fn complete_style(_ctx: &IdeContext, _index: usize) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Add Vue CSS features
        items.extend(Self::vue_css_completions());

        items
    }

    /// Vue directive completions.
    fn directive_completions() -> Vec<CompletionItem> {
        vec![
            Self::directive_item("v-if", "Conditional rendering", "v-if=\"$1\""),
            Self::directive_item("v-else-if", "Else-if block", "v-else-if=\"$1\""),
            Self::directive_item("v-else", "Else block", "v-else"),
            Self::directive_item("v-for", "List rendering", "v-for=\"$1 in $2\" :key=\"$3\""),
            Self::directive_item("v-on", "Event listener", "v-on:$1=\"$2\""),
            Self::directive_item("v-bind", "Attribute binding", "v-bind:$1=\"$2\""),
            Self::directive_item("v-model", "Two-way binding", "v-model=\"$1\""),
            Self::directive_item("v-slot", "Named slot", "v-slot:$1"),
            Self::directive_item("v-show", "Toggle visibility", "v-show=\"$1\""),
            Self::directive_item("v-pre", "Skip compilation", "v-pre"),
            Self::directive_item("v-once", "Render once", "v-once"),
            Self::directive_item("v-memo", "Memoize subtree", "v-memo=\"[$1]\""),
            Self::directive_item("v-cloak", "Hide until compiled", "v-cloak"),
            Self::directive_item("v-text", "Set text content", "v-text=\"$1\""),
            Self::directive_item("v-html", "Set innerHTML", "v-html=\"$1\""),
            // Shorthand completions
            Self::directive_item("@", "Event shorthand", "@$1=\"$2\""),
            Self::directive_item(":", "Bind shorthand", ":$1=\"$2\""),
            Self::directive_item("#", "Slot shorthand", "#$1"),
        ]
    }

    /// Create a directive completion item.
    fn directive_item(label: &str, description: &str, snippet: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(description.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**{}**\n\n{}\n\n[Vue Documentation](https://vuejs.org/api/built-in-directives.html)",
                    label, description
                ),
            })),
            ..Default::default()
        }
    }

    /// Built-in Vue component completions.
    fn builtin_component_completions() -> Vec<CompletionItem> {
        vec![
            Self::component_item("Transition", "Animate enter/leave", "<Transition name=\"$1\">\n\t$0\n</Transition>"),
            Self::component_item("TransitionGroup", "Animate list", "<TransitionGroup name=\"$1\" tag=\"$2\">\n\t$0\n</TransitionGroup>"),
            Self::component_item("KeepAlive", "Cache components", "<KeepAlive>\n\t$0\n</KeepAlive>"),
            Self::component_item("Teleport", "Teleport content", "<Teleport to=\"$1\">\n\t$0\n</Teleport>"),
            Self::component_item("Suspense", "Async dependencies", "<Suspense>\n\t<template #default>\n\t\t$0\n\t</template>\n\t<template #fallback>\n\t\tLoading...\n\t</template>\n</Suspense>"),
            Self::component_item("component", "Dynamic component", "<component :is=\"$1\" />"),
            Self::component_item("slot", "Slot outlet", "<slot name=\"$1\">$0</slot>"),
            Self::component_item("template", "Template fragment", "<template #$1>\n\t$0\n</template>"),
        ]
    }

    /// Create a component completion item.
    fn component_item(label: &str, description: &str, snippet: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(format!("Vue built-in: {}", description)),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**<{}>**\n\n{}\n\n[Vue Documentation](https://vuejs.org/api/built-in-components.html)",
                    label, description
                ),
            })),
            ..Default::default()
        }
    }

    /// Template snippet completions.
    fn template_snippets() -> Vec<CompletionItem> {
        vec![
            Self::snippet_item(
                "vfor",
                "v-for loop",
                "<$1 v-for=\"$2 in $3\" :key=\"$4\">\n\t$0\n</$1>",
            ),
            Self::snippet_item("vif", "v-if block", "<$1 v-if=\"$2\">\n\t$0\n</$1>"),
            Self::snippet_item("vshow", "v-show block", "<$1 v-show=\"$2\">\n\t$0\n</$1>"),
            Self::snippet_item(
                "vmodel",
                "v-model input",
                "<input v-model=\"$1\" type=\"$2\" />",
            ),
            Self::snippet_item("von", "v-on handler", "<$1 @$2=\"$3\">$0</$1>"),
            Self::snippet_item("vbind", "v-bind attribute", "<$1 :$2=\"$3\">$0</$1>"),
        ]
    }

    /// Create a snippet completion item.
    fn snippet_item(label: &str, description: &str, snippet: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(description.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }
    }

    /// Vue Composition API completions.
    fn composition_api_completions() -> Vec<CompletionItem> {
        vec![
            Self::api_item(
                "ref",
                "function ref<T>(value: T): Ref<T>",
                "Create a reactive reference",
            ),
            Self::api_item(
                "reactive",
                "function reactive<T>(target: T): T",
                "Create a reactive object",
            ),
            Self::api_item(
                "computed",
                "function computed<T>(getter: () => T): ComputedRef<T>",
                "Create a computed property",
            ),
            Self::api_item(
                "watch",
                "function watch(source, callback, options?)",
                "Watch reactive sources",
            ),
            Self::api_item(
                "watchEffect",
                "function watchEffect(effect: () => void)",
                "Run effect with auto-tracking",
            ),
            Self::api_item(
                "onMounted",
                "function onMounted(callback: () => void)",
                "Lifecycle: after mount",
            ),
            Self::api_item(
                "onUnmounted",
                "function onUnmounted(callback: () => void)",
                "Lifecycle: after unmount",
            ),
            Self::api_item(
                "onBeforeMount",
                "function onBeforeMount(callback: () => void)",
                "Lifecycle: before mount",
            ),
            Self::api_item(
                "onBeforeUnmount",
                "function onBeforeUnmount(callback: () => void)",
                "Lifecycle: before unmount",
            ),
            Self::api_item(
                "onUpdated",
                "function onUpdated(callback: () => void)",
                "Lifecycle: after update",
            ),
            Self::api_item(
                "onBeforeUpdate",
                "function onBeforeUpdate(callback: () => void)",
                "Lifecycle: before update",
            ),
            Self::api_item(
                "toRef",
                "function toRef<T>(object: T, key: K): Ref<T[K]>",
                "Create ref from reactive property",
            ),
            Self::api_item(
                "toRefs",
                "function toRefs<T>(object: T): ToRefs<T>",
                "Convert reactive to refs",
            ),
            Self::api_item(
                "unref",
                "function unref<T>(ref: T | Ref<T>): T",
                "Unwrap a ref",
            ),
            Self::api_item(
                "isRef",
                "function isRef(r): r is Ref",
                "Check if value is ref",
            ),
            Self::api_item(
                "shallowRef",
                "function shallowRef<T>(value: T): ShallowRef<T>",
                "Shallow reactive reference",
            ),
            Self::api_item(
                "shallowReactive",
                "function shallowReactive<T>(target: T): T",
                "Shallow reactive object",
            ),
            Self::api_item(
                "readonly",
                "function readonly<T>(target: T): DeepReadonly<T>",
                "Create readonly proxy",
            ),
            Self::api_item(
                "nextTick",
                "function nextTick(callback?): Promise<void>",
                "Wait for next DOM update",
            ),
            Self::api_item(
                "provide",
                "function provide<T>(key, value: T)",
                "Provide value to descendants",
            ),
            Self::api_item(
                "inject",
                "function inject<T>(key, defaultValue?): T",
                "Inject value from ancestor",
            ),
        ]
    }

    /// Create an API completion item.
    fn api_item(label: &str, signature: &str, description: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(signature.to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "```typescript\n{}\n```\n\n{}\n\n[Vue Documentation](https://vuejs.org/api/)",
                    signature, description
                ),
            })),
            ..Default::default()
        }
    }

    /// Vue macro completions (script setup only).
    fn macro_completions() -> Vec<CompletionItem> {
        vec![
            Self::macro_item(
                "defineProps",
                "defineProps<T>()",
                "Declare component props",
                "defineProps<{\n\t$1\n}>()",
            ),
            Self::macro_item(
                "defineEmits",
                "defineEmits<T>()",
                "Declare component emits",
                "defineEmits<{\n\t$1\n}>()",
            ),
            Self::macro_item(
                "defineExpose",
                "defineExpose(exposed)",
                "Expose properties via refs",
                "defineExpose({\n\t$1\n})",
            ),
            Self::macro_item(
                "defineOptions",
                "defineOptions(options)",
                "Declare component options",
                "defineOptions({\n\tname: '$1',\n})",
            ),
            Self::macro_item(
                "defineSlots",
                "defineSlots<T>()",
                "Declare typed slots",
                "defineSlots<{\n\t$1\n}>()",
            ),
            Self::macro_item(
                "defineModel",
                "defineModel<T>(name?, options?)",
                "Declare two-way binding prop",
                "defineModel<$1>()",
            ),
            Self::macro_item(
                "withDefaults",
                "withDefaults(props, defaults)",
                "Set prop defaults",
                "withDefaults(defineProps<{\n\t$1\n}>(), {\n\t$2\n})",
            ),
        ]
    }

    /// Create a macro completion item.
    fn macro_item(
        label: &str,
        signature: &str,
        description: &str,
        snippet: &str,
    ) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(format!("Macro: {}", signature)),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "```typescript\n{}\n```\n\n{}\n\n*Compiler macro - only usable in `<script setup>`*",
                    signature, description
                ),
            })),
            ..Default::default()
        }
    }

    /// Common import completions.
    fn import_completions() -> Vec<CompletionItem> {
        vec![
            Self::import_item("import vue", "Import from Vue", "import { $1 } from 'vue'"),
            Self::import_item(
                "import ref",
                "Import ref from Vue",
                "import { ref } from 'vue'",
            ),
            Self::import_item(
                "import reactive",
                "Import reactive from Vue",
                "import { reactive } from 'vue'",
            ),
            Self::import_item(
                "import computed",
                "Import computed from Vue",
                "import { computed } from 'vue'",
            ),
            Self::import_item(
                "import watch",
                "Import watch from Vue",
                "import { watch, watchEffect } from 'vue'",
            ),
            Self::import_item(
                "import lifecycle",
                "Import lifecycle hooks",
                "import { onMounted, onUnmounted } from 'vue'",
            ),
        ]
    }

    /// Create an import completion item.
    fn import_item(label: &str, description: &str, snippet: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some(description.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }
    }

    /// Vue CSS feature completions.
    fn vue_css_completions() -> Vec<CompletionItem> {
        vec![
            Self::css_item("v-bind", "v-bind()", "Dynamic CSS value", "v-bind($1)"),
            Self::css_item(
                ":deep",
                ":deep()",
                "Deep selector in scoped CSS",
                ":deep($1)",
            ),
            Self::css_item(
                ":slotted",
                ":slotted()",
                "Slotted content selector",
                ":slotted($1)",
            ),
            Self::css_item(":global", ":global()", "Global selector", ":global($1)"),
        ]
    }

    /// Create a CSS completion item.
    fn css_item(label: &str, signature: &str, description: &str, snippet: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(format!("Vue CSS: {}", signature)),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**{}**\n\n{}\n\n[Vue SFC CSS Features](https://vuejs.org/api/sfc-css-features.html)",
                    signature, description
                ),
            })),
            ..Default::default()
        }
    }
}

/// Completion trigger characters for Vue SFC.
pub const TRIGGER_CHARACTERS: &[char] = &[
    '<',  // HTML tags
    '.',  // Object property access
    ':',  // v-bind shorthand
    '@',  // v-on shorthand
    '#',  // v-slot shorthand
    '"',  // Attribute values
    '\'', // Attribute values
    '/',  // Closing tags
    ' ',  // Space for attribute completion
];

/// Get trigger characters as strings.
pub fn trigger_characters() -> Vec<String> {
    TRIGGER_CHARACTERS.iter().map(|c| c.to_string()).collect()
}

// =============================================================================
// Art file context detection helpers
// =============================================================================

/// Check if cursor is inside <art ...> opening tag.
fn is_inside_art_tag(before: &str) -> bool {
    // Find last <art and check if we're before the closing >
    if let Some(art_start) = before.rfind("<art") {
        let after_art = &before[art_start..];
        // Check if there's no closing > yet
        !after_art.contains('>')
    } else {
        false
    }
}

/// Check if cursor is inside <variant ...> opening tag.
fn is_inside_variant_tag(before: &str) -> bool {
    // Find last <variant and check if we're before the closing >
    if let Some(variant_start) = before.rfind("<variant") {
        let after_variant = &before[variant_start..];
        // Check if there's no closing > yet
        !after_variant.contains('>')
    } else {
        false
    }
}

/// Check if we should suggest <art> block at root level.
fn should_suggest_art_block(before: &str) -> bool {
    // Suggest art block if there's no <art> yet and we're at the start or after whitespace
    !before.contains("<art")
        && (before.trim().is_empty() || before.ends_with('\n') || before.ends_with('<'))
}

/// Check if we should suggest <variant> block inside <art>.
fn should_suggest_variant_block(before: &str) -> bool {
    // We're inside <art> if we found <art> but not </art> yet
    if let Some(art_start) = before.rfind("<art") {
        let after_art = &before[art_start..];
        // Check if we're past the opening tag and haven't closed yet
        after_art.contains('>') && !after_art.contains("</art>")
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directive_completions() {
        let items = CompletionService::directive_completions();
        assert!(!items.is_empty());

        // Check v-if is present
        let v_if = items.iter().find(|i| i.label == "v-if");
        assert!(v_if.is_some());
        assert_eq!(v_if.unwrap().kind, Some(CompletionItemKind::KEYWORD));
    }

    #[test]
    fn test_composition_api_completions() {
        let items = CompletionService::composition_api_completions();
        assert!(!items.is_empty());

        // Check ref is present
        let ref_item = items.iter().find(|i| i.label == "ref");
        assert!(ref_item.is_some());
        assert_eq!(ref_item.unwrap().kind, Some(CompletionItemKind::FUNCTION));
    }

    #[test]
    fn test_macro_completions() {
        let items = CompletionService::macro_completions();
        assert!(!items.is_empty());

        // Check defineProps is present
        let define_props = items.iter().find(|i| i.label == "defineProps");
        assert!(define_props.is_some());
    }

    #[test]
    fn test_vue_css_completions() {
        let items = CompletionService::vue_css_completions();
        assert_eq!(items.len(), 4);

        // Check :deep is present
        let deep = items.iter().find(|i| i.label == ":deep");
        assert!(deep.is_some());
    }

    #[test]
    fn test_trigger_characters() {
        let chars = trigger_characters();
        assert!(chars.contains(&"<".to_string()));
        assert!(chars.contains(&":".to_string()));
        assert!(chars.contains(&"@".to_string()));
    }

    #[test]
    fn test_binding_type_to_completion_info() {
        // Test ref binding
        let (kind, detail, _) =
            CompletionService::binding_type_to_completion_info(BindingType::SetupRef);
        assert_eq!(kind, CompletionItemKind::VARIABLE);
        assert!(detail.contains("ref"));

        // Test const binding
        let (kind, detail, _) =
            CompletionService::binding_type_to_completion_info(BindingType::SetupConst);
        assert_eq!(kind, CompletionItemKind::CONSTANT);
        assert!(detail.contains("const"));

        // Test props binding
        let (kind, detail, _) =
            CompletionService::binding_type_to_completion_info(BindingType::Props);
        assert_eq!(kind, CompletionItemKind::PROPERTY);
        assert!(detail.contains("prop"));
    }
}
