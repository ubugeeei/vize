//! Hover information provider.
//!
//! Provides contextual hover information for:
//! - Template expressions and bindings
//! - Vue directives
//! - Script bindings and imports
//! - CSS properties and Vue-specific selectors
//! - TypeScript type information from croquis analysis
//! - Real type information from tsgo (when available)

use std::sync::Arc;

use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use vize_croquis::{Analyzer, AnalyzerOptions};
use vize_relief::BindingType;

#[cfg(feature = "native")]
use vize_canon::{LspHover, LspHoverContents, LspMarkedString, TsgoBridge};

use super::IdeContext;
use crate::virtual_code::BlockType;

/// Hover service for providing contextual information.
pub struct HoverService;

impl HoverService {
    /// Get hover information for the given context.
    pub fn hover(ctx: &IdeContext) -> Option<Hover> {
        match ctx.block_type? {
            BlockType::Template => Self::hover_template(ctx),
            BlockType::Script => Self::hover_script(ctx, false),
            BlockType::ScriptSetup => Self::hover_script(ctx, true),
            BlockType::Style(index) => Self::hover_style(ctx, index),
            BlockType::Art(_) => None,
        }
    }

    /// Get hover information with tsgo support (async version).
    ///
    /// This method first tries to get type information from tsgo,
    /// then falls back to the synchronous analysis.
    #[cfg(feature = "native")]
    pub async fn hover_with_tsgo(
        ctx: &IdeContext<'_>,
        tsgo_bridge: Option<Arc<TsgoBridge>>,
    ) -> Option<Hover> {
        match ctx.block_type? {
            BlockType::Template => Self::hover_template_with_tsgo(ctx, tsgo_bridge).await,
            BlockType::Script => Self::hover_script_with_tsgo(ctx, false, tsgo_bridge).await,
            BlockType::ScriptSetup => Self::hover_script_with_tsgo(ctx, true, tsgo_bridge).await,
            BlockType::Style(index) => Self::hover_style(ctx, index),
            BlockType::Art(_) => None,
        }
    }

    /// Get hover for template context with tsgo support.
    #[cfg(feature = "native")]
    async fn hover_template_with_tsgo(
        ctx: &IdeContext<'_>,
        tsgo_bridge: Option<Arc<TsgoBridge>>,
    ) -> Option<Hover> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset);

        if word.is_empty() {
            return None;
        }

        // Check for Vue directives first (these don't need tsgo)
        if let Some(hover) = Self::hover_directive(&word) {
            return Some(hover);
        }

        // Try to get type information from tsgo via virtual TypeScript
        if let Some(bridge) = tsgo_bridge {
            if let Some(ref virtual_docs) = ctx.virtual_docs {
                if let Some(ref template) = virtual_docs.template {
                    // Calculate position in virtual TS
                    if let Some(vts_offset) = Self::sfc_to_virtual_ts_offset(ctx, ctx.offset) {
                        let (line, character) =
                            super::offset_to_position(&template.content, vts_offset);
                        let uri = format!("vize-virtual://{}.template.ts", ctx.uri.path());

                        // Open/update virtual document
                        if bridge.is_initialized() {
                            let _ = bridge
                                .open_or_update_virtual_document(
                                    &format!("{}.template.ts", ctx.uri.path()),
                                    &template.content,
                                )
                                .await;

                            // Request hover from tsgo
                            if let Ok(Some(hover)) = bridge.hover(&uri, line, character).await {
                                return Some(Self::convert_lsp_hover(hover));
                            }
                        }
                    }
                }
            }
        }

        // Fall back to croquis analysis
        Self::hover_template(ctx)
    }

    /// Get hover for script context with tsgo support.
    #[cfg(feature = "native")]
    async fn hover_script_with_tsgo(
        ctx: &IdeContext<'_>,
        is_setup: bool,
        tsgo_bridge: Option<Arc<TsgoBridge>>,
    ) -> Option<Hover> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset);

        if word.is_empty() {
            return None;
        }

        // Check for Vue Composition API and macros first
        if let Some(hover) = Self::hover_vue_api(&word) {
            return Some(hover);
        }

        if is_setup {
            if let Some(hover) = Self::hover_vue_macro(&word) {
                return Some(hover);
            }
        }

        // Try to get type information from tsgo via virtual TypeScript
        if let Some(bridge) = tsgo_bridge {
            if let Some(ref virtual_docs) = ctx.virtual_docs {
                let script_doc = if is_setup {
                    virtual_docs.script_setup.as_ref()
                } else {
                    virtual_docs.script.as_ref()
                };

                if let Some(script) = script_doc {
                    // Calculate position in virtual TS
                    if let Some(vts_offset) = Self::sfc_to_virtual_ts_script_offset(ctx, ctx.offset)
                    {
                        let (line, character) =
                            super::offset_to_position(&script.content, vts_offset);
                        let suffix = if is_setup { "setup.ts" } else { "script.ts" };
                        let uri = format!("vize-virtual://{}.{}", ctx.uri.path(), suffix);

                        // Open/update virtual document
                        if bridge.is_initialized() {
                            let _ = bridge
                                .open_or_update_virtual_document(
                                    &format!("{}.{}", ctx.uri.path(), suffix),
                                    &script.content,
                                )
                                .await;

                            // Request hover from tsgo
                            if let Ok(Some(hover)) = bridge.hover(&uri, line, character).await {
                                return Some(Self::convert_lsp_hover(hover));
                            }
                        }
                    }
                }
            }
        }

        // Fall back to croquis analysis
        Self::hover_script(ctx, is_setup)
    }

    /// Convert SFC offset to virtual TS template offset.
    #[cfg(feature = "native")]
    pub(crate) fn sfc_to_virtual_ts_offset(
        ctx: &IdeContext<'_>,
        sfc_offset: usize,
    ) -> Option<usize> {
        let virtual_docs = ctx.virtual_docs.as_ref()?;
        let template = virtual_docs.template.as_ref()?;

        // Get template block start offset in SFC
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;
        let template_block = descriptor.template.as_ref()?;
        let template_start = template_block.loc.start;

        // Check if offset is within template
        if sfc_offset < template_start || sfc_offset > template_block.loc.end {
            return None;
        }

        // Calculate relative offset
        let relative_offset = sfc_offset - template_start;

        // Use source map to convert offset
        template
            .source_map
            .to_generated(relative_offset as u32)
            .map(|o| o as usize)
            .or(Some(relative_offset))
    }

    /// Convert SFC offset to virtual TS script offset.
    #[cfg(feature = "native")]
    pub(crate) fn sfc_to_virtual_ts_script_offset(
        ctx: &IdeContext<'_>,
        sfc_offset: usize,
    ) -> Option<usize> {
        let virtual_docs = ctx.virtual_docs.as_ref()?;

        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;

        // Try script setup first
        if let Some(ref script_setup) = descriptor.script_setup {
            if sfc_offset >= script_setup.loc.start && sfc_offset <= script_setup.loc.end {
                let relative_offset = sfc_offset - script_setup.loc.start;
                if let Some(ref script_setup_doc) = virtual_docs.script_setup {
                    return script_setup_doc
                        .source_map
                        .to_generated(relative_offset as u32)
                        .map(|o| o as usize)
                        .or(Some(relative_offset));
                }
                return Some(relative_offset);
            }
        }

        // Try regular script
        if let Some(ref script) = descriptor.script {
            if sfc_offset >= script.loc.start && sfc_offset <= script.loc.end {
                let relative_offset = sfc_offset - script.loc.start;
                if let Some(ref script_doc) = virtual_docs.script {
                    return script_doc
                        .source_map
                        .to_generated(relative_offset as u32)
                        .map(|o| o as usize)
                        .or(Some(relative_offset));
                }
                return Some(relative_offset);
            }
        }

        None
    }

    /// Convert tsgo LspHover to tower-lsp Hover.
    #[cfg(feature = "native")]
    fn convert_lsp_hover(lsp_hover: LspHover) -> Hover {
        let contents = match lsp_hover.contents {
            LspHoverContents::Markup(markup) => {
                let value = if markup.kind == "markdown" {
                    markup.value
                } else {
                    // Wrap plaintext TypeScript type info in a code block for better rendering
                    Self::wrap_type_info_in_codeblock(&markup.value)
                };
                HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                })
            }
            LspHoverContents::String(s) => {
                // Wrap plaintext in a TypeScript code block
                HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: Self::wrap_type_info_in_codeblock(&s),
                })
            }
            LspHoverContents::Array(items) => {
                let value = items
                    .into_iter()
                    .map(|item| match item {
                        LspMarkedString::String(s) => Self::wrap_type_info_in_codeblock(&s),
                        LspMarkedString::LanguageString { language, value } => {
                            format!("```{}\n{}\n```", language, value)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                })
            }
        };

        let range = lsp_hover.range.map(|r| Range {
            start: tower_lsp::lsp_types::Position {
                line: r.start.line,
                character: r.start.character,
            },
            end: tower_lsp::lsp_types::Position {
                line: r.end.line,
                character: r.end.character,
            },
        });

        Hover { contents, range }
    }

    /// Wrap TypeScript type information in a code block for proper markdown rendering.
    #[cfg(feature = "native")]
    fn wrap_type_info_in_codeblock(text: &str) -> String {
        let text = text.trim();
        // If already wrapped in code block, return as-is
        if text.starts_with("```") {
            return text.to_string();
        }
        // Check if this looks like TypeScript type info
        // Common patterns: (const), (let), (var), (function), (method), (property), type, interface, etc.
        let looks_like_type_info = text.starts_with('(')
            || text.starts_with("type ")
            || text.starts_with("interface ")
            || text.starts_with("class ")
            || text.starts_with("enum ")
            || text.starts_with("function ")
            || text.starts_with("const ")
            || text.starts_with("let ")
            || text.starts_with("var ")
            || text.starts_with("import ")
            || text.contains(": ")
            || text.contains("=>")
            || text.contains(" | ")
            || text.contains(" & ");

        if looks_like_type_info {
            format!("```typescript\n{}\n```", text)
        } else {
            text.to_string()
        }
    }

    /// Get hover for template context.
    fn hover_template(ctx: &IdeContext) -> Option<Hover> {
        // Try to find what's under the cursor
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset);

        if word.is_empty() {
            return None;
        }

        // Check for Vue directives
        if let Some(hover) = Self::hover_directive(&word) {
            return Some(hover);
        }

        // Try to get TypeScript type information from croquis analysis
        if let Some(hover) = Self::hover_ts_binding(ctx, &word) {
            return Some(hover);
        }

        // Try to get type information from vize_canon
        if let Some(type_info) = super::TypeService::get_type_at(ctx) {
            let mut value = format!("**{}**\n\n```typescript\n{}\n```", word, type_info.display);

            if let Some(ref doc) = type_info.documentation {
                value.push_str(&format!("\n\n{}", doc));
            }

            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: None,
            });
        }

        // Check for template bindings from script setup
        if let Some(ref virtual_docs) = ctx.virtual_docs {
            if let Some(ref script_setup) = virtual_docs.script_setup {
                let bindings =
                    crate::virtual_code::extract_simple_bindings(&script_setup.content, true);
                if bindings.contains(&word) {
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("**{}**\n\n*Binding from `<script setup>`*", word),
                        }),
                        range: None,
                    });
                }
            }
        }

        // Default: show it's a template expression
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**{}**\n\n*Template expression*", word),
            }),
            range: None,
        })
    }

    /// Get hover for TypeScript binding using croquis analysis.
    fn hover_ts_binding(ctx: &IdeContext, word: &str) -> Option<Hover> {
        // Parse SFC to get script content
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;

        // Get the script content for type inference
        let script_content = descriptor
            .script_setup
            .as_ref()
            .map(|s| s.content.as_ref())
            .or_else(|| descriptor.script.as_ref().map(|s| s.content.as_ref()));

        // Create analyzer and analyze script
        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());

        if let Some(ref script_setup) = descriptor.script_setup {
            analyzer.analyze_script_setup(&script_setup.content);
        } else if let Some(ref script) = descriptor.script {
            analyzer.analyze_script_plain(&script.content);
        }

        // Analyze template if present
        if let Some(ref template) = descriptor.template {
            let allocator = vize_carton::Bump::new();
            let (root, _) = vize_armature::parse(&allocator, &template.content);
            analyzer.analyze_template(&root);
        }

        let summary = analyzer.finish();

        // Look up the binding in the analysis summary
        let binding_type = summary.get_binding_type(word)?;

        // Try to infer a more specific type from the script content
        let inferred_type = script_content
            .and_then(|content| Self::infer_type_from_script(content, word, binding_type))
            .unwrap_or_else(|| Self::binding_type_to_ts_display(binding_type).to_string());

        // Format the hover content
        let kind_desc = Self::binding_type_to_description(binding_type);

        let value = format!(
            "```typescript\n{}: {}\n```\n\n{}\n\n*Source: `<script setup>`*",
            word, inferred_type, kind_desc
        );

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        })
    }

    /// Infer a more specific type from the script content.
    fn infer_type_from_script(
        content: &str,
        name: &str,
        binding_type: BindingType,
    ) -> Option<String> {
        // Patterns to look for initialization
        let patterns = [
            format!("const {} = ref(", name),
            format!("const {} = ref<", name),
            format!("let {} = ref(", name),
            format!("const {} = shallowRef(", name),
            format!("const {} = reactive(", name),
            format!("const {} = reactive<", name),
            format!("const {} = computed(", name),
            format!("const {} = computed<", name),
        ];

        for pattern in &patterns {
            if let Some(pos) = content.find(pattern.as_str()) {
                let after_pattern = &content[pos + pattern.len()..];

                // Check if it's a generic type annotation: ref<Type>
                if pattern.ends_with('<') {
                    // Find the closing >
                    if let Some(end) = Self::find_matching_bracket(after_pattern, '<', '>') {
                        let type_arg = &after_pattern[..end];
                        return Some(Self::format_wrapper_type(pattern, type_arg));
                    }
                }

                // Try to infer from the argument
                if let Some(arg_type) = Self::infer_type_from_arg(after_pattern) {
                    return Some(Self::format_wrapper_type(pattern, &arg_type));
                }
            }
        }

        // Check for explicit type annotation: const name: Type = ...
        let type_annotation_patterns = [format!("const {}: ", name), format!("let {}: ", name)];

        for pattern in &type_annotation_patterns {
            if let Some(pos) = content.find(pattern.as_str()) {
                let after_pattern = &content[pos + pattern.len()..];
                // Find = or end of type
                if let Some(type_str) = Self::extract_type_annotation(after_pattern) {
                    return Some(type_str);
                }
            }
        }

        // For Props, try to get the actual prop type
        if binding_type == BindingType::Props {
            return Self::infer_prop_type(content, name);
        }

        None
    }

    /// Format the wrapper type (Ref, Reactive, etc.) with the inner type.
    fn format_wrapper_type(pattern: &str, inner_type: &str) -> String {
        if pattern.contains("ref(") || pattern.contains("ref<") || pattern.contains("shallowRef(") {
            format!("Ref<{}>", inner_type)
        } else if pattern.contains("reactive(") || pattern.contains("reactive<") {
            format!("Reactive<{}>", inner_type)
        } else if pattern.contains("computed(") || pattern.contains("computed<") {
            format!("ComputedRef<{}>", inner_type)
        } else {
            inner_type.to_string()
        }
    }

    /// Infer type from an argument value (literal or expression).
    fn infer_type_from_arg(arg_str: &str) -> Option<String> {
        let arg_str = arg_str.trim();

        // Number literal
        if arg_str.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
            let num_end = arg_str
                .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-' && c != 'e' && c != 'E')
                .unwrap_or(arg_str.len());
            let num_str = &arg_str[..num_end];
            if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
                return Some("number".to_string());
            }
            return Some("number".to_string());
        }

        // String literal
        if arg_str.starts_with('"') || arg_str.starts_with('\'') || arg_str.starts_with('`') {
            return Some("string".to_string());
        }

        // Boolean literal
        if arg_str.starts_with("true") || arg_str.starts_with("false") {
            return Some("boolean".to_string());
        }

        // Array literal
        if arg_str.starts_with('[') {
            // Try to infer array element type
            if arg_str.starts_with("[]") {
                return Some("unknown[]".to_string());
            }
            return Some("unknown[]".to_string());
        }

        // Object literal
        if arg_str.starts_with('{') {
            // Could try to infer object structure, but keep it simple for now
            return Some("object".to_string());
        }

        // null/undefined
        if arg_str.starts_with("null") {
            return Some("null".to_string());
        }
        if arg_str.starts_with("undefined") {
            return Some("undefined".to_string());
        }

        None
    }

    /// Extract type annotation from a string like "Type = ..."
    fn extract_type_annotation(s: &str) -> Option<String> {
        let s = s.trim();
        let mut depth = 0;
        let mut end = 0;

        for (i, c) in s.chars().enumerate() {
            match c {
                '<' | '(' | '[' | '{' => depth += 1,
                '>' | ')' | ']' | '}' => depth -= 1,
                '=' if depth == 0 => {
                    end = i;
                    break;
                }
                ';' | '\n' if depth == 0 => {
                    end = i;
                    break;
                }
                _ => {}
            }
            end = i + 1;
        }

        if end > 0 {
            let type_str = s[..end].trim();
            if !type_str.is_empty() {
                return Some(type_str.to_string());
            }
        }

        None
    }

    /// Find matching bracket position.
    fn find_matching_bracket(s: &str, open: char, close: char) -> Option<usize> {
        let mut depth = 1;
        for (i, c) in s.chars().enumerate() {
            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Infer prop type from defineProps.
    fn infer_prop_type(content: &str, prop_name: &str) -> Option<String> {
        // Look for defineProps<{ propName: Type }>
        if let Some(props_start) = content.find("defineProps<") {
            let after = &content[props_start + "defineProps<".len()..];
            if let Some(end) = Self::find_matching_bracket(after, '<', '>') {
                let props_type = &after[..end];
                // Look for the property
                let prop_pattern = format!("{}: ", prop_name);
                if let Some(prop_pos) = props_type.find(&prop_pattern) {
                    let after_prop = &props_type[prop_pos + prop_pattern.len()..];
                    if let Some(type_str) = Self::extract_prop_type(after_prop) {
                        return Some(type_str);
                    }
                }
                // Also check for optional: propName?: Type
                let opt_pattern = format!("{}?: ", prop_name);
                if let Some(prop_pos) = props_type.find(&opt_pattern) {
                    let after_prop = &props_type[prop_pos + opt_pattern.len()..];
                    if let Some(type_str) = Self::extract_prop_type(after_prop) {
                        return Some(format!("{} | undefined", type_str));
                    }
                }
            }
        }
        None
    }

    /// Extract a prop type from the remaining string.
    fn extract_prop_type(s: &str) -> Option<String> {
        let s = s.trim();
        let mut depth = 0;
        let mut end = 0;

        for (i, c) in s.chars().enumerate() {
            match c {
                '<' | '(' | '[' | '{' => depth += 1,
                '>' | ')' | ']' | '}' => {
                    if depth == 0 {
                        end = i;
                        break;
                    }
                    depth -= 1;
                }
                ',' | ';' | '\n' if depth == 0 => {
                    end = i;
                    break;
                }
                _ => {}
            }
            end = i + 1;
        }

        if end > 0 {
            let type_str = s[..end].trim();
            if !type_str.is_empty() {
                return Some(type_str.to_string());
            }
        }

        None
    }

    /// Convert BindingType to TypeScript type display string.
    fn binding_type_to_ts_display(binding_type: BindingType) -> &'static str {
        match binding_type {
            BindingType::SetupRef => "Ref<unknown>",
            BindingType::SetupMaybeRef => "MaybeRef<unknown>",
            BindingType::SetupReactiveConst => "Reactive<unknown>",
            BindingType::SetupConst => "const",
            BindingType::SetupLet => "let",
            BindingType::Props => "Props",
            BindingType::PropsAliased => "Props (aliased)",
            BindingType::Data => "data",
            BindingType::Options => "options",
            BindingType::LiteralConst => "literal const",
            BindingType::JsGlobalUniversal => "global (universal)",
            BindingType::JsGlobalBrowser => "global (browser)",
            BindingType::JsGlobalNode => "global (node)",
            BindingType::JsGlobalDeno => "global (deno)",
            BindingType::JsGlobalBun => "global (bun)",
            BindingType::VueGlobal => "Vue global",
            BindingType::ExternalModule => "imported module",
        }
    }

    /// Convert BindingType to human-readable description.
    fn binding_type_to_description(binding_type: BindingType) -> &'static str {
        match binding_type {
            BindingType::SetupRef => "Reactive reference created with `ref()`. Access `.value` in script, auto-unwrapped in template.",
            BindingType::SetupMaybeRef => "Value that may be a ref. Use `unref()` or `toValue()` to access in script.",
            BindingType::SetupReactiveConst => "Reactive object created with `reactive()`. Properties are reactive.",
            BindingType::SetupConst => "Constant binding from script setup. Non-reactive unless wrapped.",
            BindingType::SetupLet => "Mutable binding from script setup. Changes won't trigger reactivity.",
            BindingType::Props => "Component prop. Read-only in the component.",
            BindingType::PropsAliased => "Destructured prop with alias. Read-only.",
            BindingType::Data => "Reactive data from Options API `data()` function.",
            BindingType::Options => "Binding from Options API (methods, computed, etc.).",
            BindingType::LiteralConst => "Literal constant value, hoisted for optimization.",
            BindingType::JsGlobalUniversal => "JavaScript global available in all environments.",
            BindingType::JsGlobalBrowser => "Browser-specific global (window, document, etc.).",
            BindingType::JsGlobalNode => "Node.js-specific global (process, Buffer, etc.).",
            BindingType::JsGlobalDeno => "Deno-specific global.",
            BindingType::JsGlobalBun => "Bun-specific global.",
            BindingType::VueGlobal => "Vue template global ($slots, $emit, $attrs, etc.).",
            BindingType::ExternalModule => "Imported from external module.",
        }
    }

    /// Get hover for script context.
    fn hover_script(ctx: &IdeContext, is_setup: bool) -> Option<Hover> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset);

        if word.is_empty() {
            return None;
        }

        // Check for Vue Composition API
        if let Some(hover) = Self::hover_vue_api(&word) {
            return Some(hover);
        }

        // Check for Vue macros (script setup only)
        if is_setup {
            if let Some(hover) = Self::hover_vue_macro(&word) {
                return Some(hover);
            }
        }

        // Try to get TypeScript type information from croquis analysis
        if let Some(hover) = Self::hover_ts_binding_in_script(ctx, &word) {
            return Some(hover);
        }

        None
    }

    /// Get hover for TypeScript binding in script using croquis analysis.
    fn hover_ts_binding_in_script(ctx: &IdeContext, word: &str) -> Option<Hover> {
        // Parse SFC to get script content
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;

        // Get the script content for type inference
        let script_content = descriptor
            .script_setup
            .as_ref()
            .map(|s| s.content.as_ref())
            .or_else(|| descriptor.script.as_ref().map(|s| s.content.as_ref()));

        // Create analyzer and analyze script
        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());

        if let Some(ref script_setup) = descriptor.script_setup {
            analyzer.analyze_script_setup(&script_setup.content);
        } else if let Some(ref script) = descriptor.script {
            analyzer.analyze_script_plain(&script.content);
        }

        let summary = analyzer.finish();

        // Look up the binding in the analysis summary
        let binding_type = summary.get_binding_type(word)?;

        // Try to infer a more specific type from the script content
        let inferred_type = script_content
            .and_then(|content| Self::infer_type_from_script(content, word, binding_type))
            .unwrap_or_else(|| Self::binding_type_to_ts_display(binding_type).to_string());

        // Format the hover content with reactivity hints for script context
        let kind_desc = Self::binding_type_to_description(binding_type);

        // Add .value hint for refs in script
        let value_hint = if summary.needs_value_in_script(word) {
            format!(
                "\n\n**Tip:** Use `{}.value` to access the value in script.",
                word
            )
        } else {
            String::new()
        };

        let value = format!(
            "```typescript\n{}: {}\n```\n\n{}{}",
            word, inferred_type, kind_desc, value_hint
        );

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        })
    }

    /// Get hover for style context.
    fn hover_style(ctx: &IdeContext, _index: usize) -> Option<Hover> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset);

        if word.is_empty() {
            return None;
        }

        // Check for Vue-specific CSS features
        if let Some(hover) = Self::hover_vue_css(&word) {
            return Some(hover);
        }

        None
    }

    /// Get hover for Vue directives.
    fn hover_directive(word: &str) -> Option<Hover> {
        let (title, description) = match word {
            "v-if" => ("v-if", "Conditionally render the element based on the truthy-ness of the expression value."),
            "v-else-if" => ("v-else-if", "Denote the \"else if block\" for `v-if`. Can be chained."),
            "v-else" => ("v-else", "Denote the \"else block\" for `v-if` or `v-if`/`v-else-if` chain."),
            "v-for" => ("v-for", "Render the element or template block multiple times based on the source data."),
            "v-on" | "@" => ("v-on", "Attach an event listener to the element. The event type is denoted by the argument."),
            "v-bind" | ":" => ("v-bind", "Dynamically bind one or more attributes, or a component prop to an expression."),
            "v-model" => ("v-model", "Create a two-way binding on a form input element or a component."),
            "v-slot" | "#" => ("v-slot", "Denote named slots or scoped slots that expect to receive props."),
            "v-pre" => ("v-pre", "Skip compilation for this element and all its children."),
            "v-once" => ("v-once", "Render the element and component once only, and skip future updates."),
            "v-memo" => ("v-memo", "Memoize a sub-tree of the template. Can be used on both elements and components."),
            "v-cloak" => ("v-cloak", "Used to hide un-compiled template until it is ready."),
            "v-show" => ("v-show", "Toggle the element's visibility based on the truthy-ness of the expression value."),
            "v-text" => ("v-text", "Update the element's text content."),
            "v-html" => ("v-html", "Update the element's innerHTML."),
            _ => return None,
        };

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**{}**\n\n{}\n\n[Vue Documentation](https://vuejs.org/api/built-in-directives.html)", title, description),
            }),
            range: None,
        })
    }

    /// Get hover for Vue Composition API.
    fn hover_vue_api(word: &str) -> Option<Hover> {
        let (signature, description) = match word {
            "ref" => (
                "function ref<T>(value: T): Ref<T>",
                "Takes an inner value and returns a reactive and mutable ref object, which has a single property `.value` that points to the inner value.",
            ),
            "reactive" => (
                "function reactive<T extends object>(target: T): T",
                "Returns a reactive proxy of the object. The reactive conversion is \"deep\": it affects all nested properties.",
            ),
            "computed" => (
                "function computed<T>(getter: () => T): ComputedRef<T>",
                "Takes a getter function and returns a readonly reactive ref object for the returned value from the getter.",
            ),
            "watch" => (
                "function watch<T>(source: WatchSource<T>, callback: WatchCallback<T>): WatchStopHandle",
                "Watches one or more reactive data sources and invokes a callback function when the sources change.",
            ),
            "watchEffect" => (
                "function watchEffect(effect: () => void): WatchStopHandle",
                "Runs a function immediately while reactively tracking its dependencies and re-runs it whenever the dependencies are changed.",
            ),
            "onMounted" => (
                "function onMounted(callback: () => void): void",
                "Registers a callback to be called after the component has been mounted.",
            ),
            "onUnmounted" => (
                "function onUnmounted(callback: () => void): void",
                "Registers a callback to be called after the component has been unmounted.",
            ),
            "onBeforeMount" => (
                "function onBeforeMount(callback: () => void): void",
                "Registers a hook to be called right before the component is to be mounted.",
            ),
            "onBeforeUnmount" => (
                "function onBeforeUnmount(callback: () => void): void",
                "Registers a hook to be called right before a component instance is to be unmounted.",
            ),
            "onUpdated" => (
                "function onUpdated(callback: () => void): void",
                "Registers a callback to be called after the component has updated its DOM tree due to a reactive state change.",
            ),
            "onBeforeUpdate" => (
                "function onBeforeUpdate(callback: () => void): void",
                "Registers a hook to be called right before the component is about to update its DOM tree due to a reactive state change.",
            ),
            "toRef" => (
                "function toRef<T extends object, K extends keyof T>(object: T, key: K): Ref<T[K]>",
                "Creates a ref that is synced with a property of a reactive object.",
            ),
            "toRefs" => (
                "function toRefs<T extends object>(object: T): ToRefs<T>",
                "Converts a reactive object to a plain object where each property is a ref pointing to the corresponding property of the original object.",
            ),
            "unref" => (
                "function unref<T>(ref: T | Ref<T>): T",
                "Returns the inner value if the argument is a ref, otherwise return the argument itself.",
            ),
            "isRef" => (
                "function isRef<T>(r: Ref<T> | unknown): r is Ref<T>",
                "Checks if a value is a ref object.",
            ),
            "shallowRef" => (
                "function shallowRef<T>(value: T): ShallowRef<T>",
                "Shallow version of `ref()`. The inner value is stored and exposed as-is, and will not be made deeply reactive.",
            ),
            "shallowReactive" => (
                "function shallowReactive<T extends object>(target: T): T",
                "Shallow version of `reactive()`. Only the root level is reactive, nested objects are not converted.",
            ),
            "readonly" => (
                "function readonly<T extends object>(target: T): DeepReadonly<T>",
                "Takes an object and returns a readonly proxy of the original.",
            ),
            "nextTick" => (
                "function nextTick(callback?: () => void): Promise<void>",
                "Utility for waiting for the next DOM update flush.",
            ),
            _ => return None,
        };

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "```typescript\n{}\n```\n\n{}\n\n[Vue Documentation](https://vuejs.org/api/)",
                    signature, description
                ),
            }),
            range: None,
        })
    }

    /// Get hover for Vue macros.
    fn hover_vue_macro(word: &str) -> Option<Hover> {
        let (signature, description) = match word {
            "defineProps" => (
                "function defineProps<T>(): T",
                "Compiler macro to declare component props. Only usable inside `<script setup>`.",
            ),
            "defineEmits" => (
                "function defineEmits<T>(): T",
                "Compiler macro to declare component emits. Only usable inside `<script setup>`.",
            ),
            "defineExpose" => (
                "function defineExpose(exposed: Record<string, any>): void",
                "Compiler macro to explicitly expose properties to the parent via template refs.",
            ),
            "defineOptions" => (
                "function defineOptions(options: ComponentOptions): void",
                "Compiler macro to declare component options. Only usable inside `<script setup>`.",
            ),
            "defineSlots" => (
                "function defineSlots<T>(): T",
                "Compiler macro for typed slots. Only usable inside `<script setup>`.",
            ),
            "defineModel" => (
                "function defineModel<T>(name?: string, options?: DefineModelOptions): ModelRef<T>",
                "Compiler macro to declare a two-way binding prop with corresponding update event.",
            ),
            "withDefaults" => (
                "function withDefaults<T>(props: T, defaults: Partial<T>): T",
                "Provides default values for props when using type-only props declaration.",
            ),
            _ => return None,
        };

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "```typescript\n{}\n```\n\n{}\n\n*Compiler macro - only usable inside `<script setup>`*",
                    signature, description
                ),
            }),
            range: None,
        })
    }

    /// Get hover for Vue CSS features.
    fn hover_vue_css(word: &str) -> Option<Hover> {
        let (title, description) = match word {
            "v-bind" => (
                "v-bind() in CSS",
                "Link CSS values to dynamic component state. The value will be compiled into a hashed CSS custom property.",
            ),
            ":deep" => (
                ":deep()",
                "Affects child component styles in scoped CSS. The selector inside `:deep()` will be compiled with the scoped attribute.",
            ),
            ":slotted" => (
                ":slotted()",
                "Target content passed via slots in scoped CSS. Only works inside scoped `<style>` blocks.",
            ),
            ":global" => (
                ":global()",
                "Apply styles globally, escaping the scoped CSS encapsulation.",
            ),
            _ => return None,
        };

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**{}**\n\n{}\n\n[Vue SFC CSS Features](https://vuejs.org/api/sfc-css-features.html)",
                    title, description
                ),
            }),
            range: None,
        })
    }

    /// Get the word at a given offset.
    fn get_word_at_offset(content: &str, offset: usize) -> String {
        if offset >= content.len() {
            return String::new();
        }

        let bytes = content.as_bytes();

        // If the character at offset is not a word character, return empty
        if !Self::is_word_char(bytes[offset]) {
            return String::new();
        }

        // Find word start
        let mut start = offset;
        while start > 0 {
            let c = bytes[start - 1];
            if !Self::is_word_char(c) {
                break;
            }
            start -= 1;
        }

        // Find word end
        let mut end = offset;
        while end < bytes.len() {
            let c = bytes[end];
            if !Self::is_word_char(c) {
                break;
            }
            end += 1;
        }

        if start == end {
            return String::new();
        }

        String::from_utf8_lossy(&bytes[start..end]).to_string()
    }

    /// Check if a byte is a valid word character.
    #[inline]
    fn is_word_char(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'$' || c == b':'
    }
}

/// Hover content builder for creating rich hover information.
pub struct HoverBuilder {
    sections: Vec<String>,
}

impl HoverBuilder {
    /// Create a new hover builder.
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    /// Add a title.
    pub fn title(mut self, title: &str) -> Self {
        self.sections.push(format!("**{}**", title));
        self
    }

    /// Add a code block.
    pub fn code(mut self, language: &str, code: &str) -> Self {
        self.sections
            .push(format!("```{}\n{}\n```", language, code));
        self
    }

    /// Add a description.
    pub fn description(mut self, text: &str) -> Self {
        self.sections.push(text.to_string());
        self
    }

    /// Add a documentation link.
    pub fn link(mut self, text: &str, url: &str) -> Self {
        self.sections.push(format!("[{}]({})", text, url));
        self
    }

    /// Build the hover.
    pub fn build(self) -> Hover {
        Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: self.sections.join("\n\n"),
            }),
            range: None,
        }
    }

    /// Build the hover with a range.
    pub fn build_with_range(self, range: Range) -> Hover {
        Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: self.sections.join("\n\n"),
            }),
            range: Some(range),
        }
    }
}

impl Default for HoverBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_word_at_offset() {
        let content = "const message = 'hello'";

        assert_eq!(HoverService::get_word_at_offset(content, 0), "const");
        assert_eq!(HoverService::get_word_at_offset(content, 6), "message");
        assert_eq!(HoverService::get_word_at_offset(content, 5), "");
    }

    #[test]
    fn test_hover_directive() {
        let hover = HoverService::hover_directive("v-if");
        assert!(hover.is_some());

        let hover = HoverService::hover_directive("unknown");
        assert!(hover.is_none());
    }

    #[test]
    fn test_hover_vue_api() {
        let hover = HoverService::hover_vue_api("ref");
        assert!(hover.is_some());

        let hover = HoverService::hover_vue_api("unknown");
        assert!(hover.is_none());
    }

    #[test]
    fn test_hover_builder() {
        let hover = HoverBuilder::new()
            .title("ref")
            .code("typescript", "function ref<T>(value: T): Ref<T>")
            .description("Creates a reactive reference.")
            .link("Documentation", "https://vuejs.org")
            .build();

        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("**ref**"));
            assert!(content.value.contains("```typescript"));
        } else {
            panic!("Expected Markup content");
        }
    }

    #[test]
    fn test_binding_type_to_ts_display() {
        assert_eq!(
            HoverService::binding_type_to_ts_display(BindingType::SetupRef),
            "Ref<unknown>"
        );
        assert_eq!(
            HoverService::binding_type_to_ts_display(BindingType::SetupReactiveConst),
            "Reactive<unknown>"
        );
        assert_eq!(
            HoverService::binding_type_to_ts_display(BindingType::Props),
            "Props"
        );
        assert_eq!(
            HoverService::binding_type_to_ts_display(BindingType::SetupConst),
            "const"
        );
    }

    #[test]
    fn test_binding_type_to_description() {
        let desc = HoverService::binding_type_to_description(BindingType::SetupRef);
        assert!(desc.contains("ref()"));
        assert!(desc.contains(".value"));

        let desc = HoverService::binding_type_to_description(BindingType::Props);
        assert!(desc.contains("prop"));
        assert!(desc.contains("Read-only"));
    }
}
