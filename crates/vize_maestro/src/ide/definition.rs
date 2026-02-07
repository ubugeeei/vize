//! Definition provider for Vue SFC files.
//!
//! Provides go-to-definition for:
//! - Template expressions -> script bindings
//! - Component usages -> component definitions
//! - Import statements -> imported files
//! - Real definitions from tsgo (when available)

use std::path::PathBuf;
use std::sync::Arc;

use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};
use vize_croquis::{Analyzer, AnalyzerOptions};
use vize_relief::BindingType;

#[cfg(feature = "native")]
use vize_canon::TsgoBridge;

use super::{is_component_tag, kebab_to_pascal, IdeContext};
use crate::virtual_code::BlockType;

/// Definition service for providing go-to-definition functionality.
pub struct DefinitionService;

impl DefinitionService {
    /// Get definition for the symbol at the current position.
    pub fn definition(ctx: &IdeContext) -> Option<GotoDefinitionResponse> {
        match ctx.block_type? {
            BlockType::Template => Self::definition_in_template(ctx),
            BlockType::Script | BlockType::ScriptSetup => Self::definition_in_script(ctx),
            BlockType::Style(_) => Self::definition_in_style(ctx),
            BlockType::Art(_) => None,
        }
    }

    /// Get definition with tsgo support (async version).
    #[cfg(feature = "native")]
    pub async fn definition_with_tsgo(
        ctx: &IdeContext<'_>,
        tsgo_bridge: Option<Arc<TsgoBridge>>,
    ) -> Option<GotoDefinitionResponse> {
        match ctx.block_type? {
            BlockType::Template => Self::definition_in_template_with_tsgo(ctx, tsgo_bridge).await,
            BlockType::Script | BlockType::ScriptSetup => {
                Self::definition_in_script_with_tsgo(ctx, tsgo_bridge).await
            }
            BlockType::Style(_) => Self::definition_in_style(ctx),
            BlockType::Art(_) => None,
        }
    }

    /// Find definition in template with tsgo and component jump support.
    #[cfg(feature = "native")]
    async fn definition_in_template_with_tsgo(
        ctx: &IdeContext<'_>,
        tsgo_bridge: Option<Arc<TsgoBridge>>,
    ) -> Option<GotoDefinitionResponse> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() {
            return None;
        }

        // Check if this is a component tag
        if let Some(tag_name) = Self::get_tag_at_offset(&ctx.content, ctx.offset) {
            if is_component_tag(&tag_name) {
                if let Some(def) = Self::find_component_definition(ctx, &tag_name) {
                    return Some(def);
                }
            }
        }

        // Check if this is a props property access (e.g., props.title -> defineProps)
        if let Some(def) = Self::find_props_property_definition(ctx, &word) {
            return Some(def);
        }

        // Check if this is a component attribute (e.g., :disabled -> component's props)
        if let Some(def) = Self::find_component_prop_definition(ctx) {
            return Some(def);
        }

        // Check if this is a prop name used directly in template
        // Only check if we're inside a Vue directive expression, not a plain HTML attribute
        if Self::is_in_vue_directive_expression(ctx) {
            let options = vize_atelier_sfc::SfcParseOptions {
                filename: ctx.uri.path().to_string(),
                ..Default::default()
            };
            if let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&ctx.content, options) {
                if let Some(def) = Self::find_prop_definition_by_name(ctx, &descriptor, &word) {
                    return Some(def);
                }
            }
        }

        // Try tsgo definition
        if let Some(bridge) = tsgo_bridge {
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

                            if let Ok(locations) = bridge.definition(&uri, line, character).await {
                                if !locations.is_empty() {
                                    return Some(Self::convert_lsp_locations(locations, ctx));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fall back to synchronous definition
        Self::definition_in_template(ctx)
    }

    /// Find definition in script with tsgo support.
    #[cfg(feature = "native")]
    async fn definition_in_script_with_tsgo(
        ctx: &IdeContext<'_>,
        tsgo_bridge: Option<Arc<TsgoBridge>>,
    ) -> Option<GotoDefinitionResponse> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() {
            return None;
        }

        let is_setup = matches!(ctx.block_type, Some(BlockType::ScriptSetup));

        // Try tsgo definition
        if let Some(bridge) = tsgo_bridge {
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
                        let (line, character) =
                            super::offset_to_position(&script.content, vts_offset);
                        let suffix = if is_setup { "setup.ts" } else { "script.ts" };
                        let uri = format!("vize-virtual://{}.{}", ctx.uri.path(), suffix);

                        if bridge.is_initialized() {
                            let _ = bridge
                                .open_or_update_virtual_document(
                                    &format!("{}.{}", ctx.uri.path(), suffix),
                                    &script.content,
                                )
                                .await;

                            if let Ok(locations) = bridge.definition(&uri, line, character).await {
                                if !locations.is_empty() {
                                    return Some(Self::convert_lsp_locations(locations, ctx));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fall back to synchronous definition
        Self::definition_in_script(ctx)
    }

    /// Get the tag name at the given offset (if cursor is on a tag).
    fn get_tag_at_offset(content: &str, offset: usize) -> Option<String> {
        if offset >= content.len() {
            return None;
        }

        let bytes = content.as_bytes();

        // Look backwards for '<'
        let mut tag_start = None;
        let mut i = offset;
        while i > 0 {
            i -= 1;
            if bytes[i] == b'<' {
                tag_start = Some(i + 1);
                break;
            }
            if bytes[i] == b'>' || bytes[i] == b'\n' {
                break;
            }
        }

        let start = tag_start?;

        // Find the end of the tag name
        let mut end = start;
        while end < bytes.len() {
            let c = bytes[end];
            if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' {
                end += 1;
            } else {
                break;
            }
        }

        if end > start {
            Some(String::from_utf8_lossy(&bytes[start..end]).to_string())
        } else {
            None
        }
    }

    /// Find the definition of a props property (e.g., props.title -> defineProps<{ title: ... }>).
    fn find_props_property_definition(
        ctx: &IdeContext<'_>,
        property_name: &str,
    ) -> Option<GotoDefinitionResponse> {
        // Check if cursor is after "props." by looking backwards
        // Find the word start position
        let mut word_start = ctx.offset;
        while word_start > 0 && Self::is_word_char(ctx.content.as_bytes()[word_start - 1]) {
            word_start -= 1;
        }

        // Check if preceded by "props."
        if word_start < 6 {
            return None;
        }

        let prefix = &ctx.content[word_start.saturating_sub(6)..word_start];
        if prefix != "props." {
            return None;
        }

        // Parse SFC to find defineProps location
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;

        // Look in script setup for defineProps
        if let Some(ref script_setup) = descriptor.script_setup {
            let content = &script_setup.content;

            // Find defineProps call or type
            // Pattern 1: defineProps<{ propName: Type }>()
            // Pattern 2: defineProps({ propName: { type: Type } })
            // Pattern 3: const props = defineProps<{ propName: Type }>()

            // First, try to find the property name in defineProps type argument
            if let Some(define_props_pos) = content.find("defineProps") {
                // Try to find the property within the type/object
                let after_define_props = &content[define_props_pos..];

                // Look for the property name in the type definition
                if let Some(prop_pos) =
                    Self::find_prop_in_define_props(after_define_props, property_name)
                {
                    let sfc_offset = script_setup.loc.start + define_props_pos + prop_pos;
                    let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: ctx.uri.clone(),
                        range: Range {
                            start: Position { line, character },
                            end: Position {
                                line,
                                character: character + property_name.len() as u32,
                            },
                        },
                    }));
                }

                // Fallback: jump to defineProps call itself
                let sfc_offset = script_setup.loc.start + define_props_pos;
                let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: ctx.uri.clone(),
                    range: Range {
                        start: Position { line, character },
                        end: Position {
                            line,
                            character: character + "defineProps".len() as u32,
                        },
                    },
                }));
            }
        }

        None
    }

    /// Find component prop definition from an attribute like :disabled or v-bind:disabled.
    fn find_component_prop_definition(ctx: &IdeContext<'_>) -> Option<GotoDefinitionResponse> {
        // Check if cursor is on an attribute name
        let (attr_name, component_name) = Self::get_attribute_and_component_at_offset(ctx)?;

        // Skip HTML elements
        if !is_component_tag(&component_name) {
            return None;
        }

        // Find the component's import path
        let import_path = Self::find_import_path(ctx, &component_name)?;

        // Resolve to file path
        let resolved_path = Self::resolve_import_path(ctx.uri, &import_path)?;

        // Read the component file and find the prop definition
        let component_content = std::fs::read_to_string(&resolved_path).ok()?;

        // Parse the component to find defineProps
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: resolved_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&component_content, options).ok()?;

        // Normalize attribute name (kebab-case to camelCase)
        let prop_name = Self::kebab_to_camel(&attr_name);

        // Look in script setup for defineProps
        if let Some(ref script_setup) = descriptor.script_setup {
            let content = &script_setup.content;

            if let Some(define_props_pos) = content.find("defineProps") {
                let after_define_props = &content[define_props_pos..];

                // Look for the property name in the type definition
                if let Some(prop_pos) =
                    Self::find_prop_in_define_props(after_define_props, &prop_name)
                {
                    let sfc_offset = script_setup.loc.start + define_props_pos + prop_pos;
                    let (line, character) =
                        Self::offset_to_position(&component_content, sfc_offset);

                    let file_uri = Url::from_file_path(&resolved_path).ok()?;
                    return Some(GotoDefinitionResponse::Scalar(Location {
                        uri: file_uri,
                        range: Range {
                            start: Position { line, character },
                            end: Position {
                                line,
                                character: character + prop_name.len() as u32,
                            },
                        },
                    }));
                }

                // Fallback: jump to defineProps
                let sfc_offset = script_setup.loc.start + define_props_pos;
                let (line, character) = Self::offset_to_position(&component_content, sfc_offset);

                let file_uri = Url::from_file_path(&resolved_path).ok()?;
                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: file_uri,
                    range: Range {
                        start: Position { line, character },
                        end: Position {
                            line,
                            character: character + "defineProps".len() as u32,
                        },
                    },
                }));
            }
        }

        None
    }

    /// Get the attribute name and component name at the cursor position.
    fn get_attribute_and_component_at_offset(ctx: &IdeContext<'_>) -> Option<(String, String)> {
        let content = &ctx.content;
        let offset = ctx.offset;

        // Find the start of the current line or tag
        let mut tag_start = offset;
        let mut depth = 0;

        // Scan backwards to find the opening tag
        while tag_start > 0 {
            let c = content.as_bytes()[tag_start - 1];
            if c == b'>' {
                depth += 1;
            } else if c == b'<' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            tag_start -= 1;
        }

        if tag_start == 0 {
            return None;
        }

        // Find the end of the tag (closing >)
        let tag_end = content[offset..].find('>')? + offset;
        let tag_content = &content[tag_start..tag_end];

        // Extract tag name
        let tag_name_end = tag_content.find(|c: char| c.is_whitespace() || c == '>' || c == '/')?;
        let tag_name = &tag_content[..tag_name_end];

        // Check if cursor is on an attribute
        let cursor_in_tag = offset - tag_start;
        let before_cursor = &tag_content[..cursor_in_tag];

        // Find the attribute we're on
        // Look for patterns: :attr, v-bind:attr, @attr, v-on:attr, attr
        let attr_start =
            before_cursor.rfind(|c: char| c.is_whitespace() || c == ':' || c == '@')?;
        let after_attr_start = &before_cursor[attr_start..].trim_start();

        // Extract attribute name
        let attr_end = after_attr_start
            .find(|c: char| c == '=' || c.is_whitespace())
            .unwrap_or(after_attr_start.len());
        let mut attr_name = &after_attr_start[..attr_end];

        // Handle directive prefixes
        if let Some(stripped) = attr_name.strip_prefix(':') {
            attr_name = stripped;
        } else if let Some(stripped) = attr_name.strip_prefix("v-bind:") {
            attr_name = stripped;
        } else if attr_name.starts_with('@')
            || attr_name.starts_with("v-on:")
            || attr_name.starts_with("v-")
        {
            // Event handlers and other directives - not props
            return None;
        }

        if attr_name.is_empty() {
            return None;
        }

        Some((attr_name.to_string(), tag_name.to_string()))
    }

    /// Convert kebab-case to camelCase.
    fn kebab_to_camel(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut capitalize_next = false;

        for c in s.chars() {
            if c == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Find a property name within defineProps type/object definition.
    fn find_prop_in_define_props(content: &str, property_name: &str) -> Option<usize> {
        // Look for patterns like:
        // - { propName: Type }  (type parameter)
        // - { propName?: Type } (optional type parameter)
        // - { propName: { type: ... } } (runtime declaration)

        let patterns = [
            format!("{}: ", property_name),
            format!("{}?: ", property_name),
            format!("{} :", property_name),
            format!("{}?:", property_name),
        ];

        for pattern in &patterns {
            if let Some(pos) = content.find(pattern.as_str()) {
                // Verify it's within angle brackets or curly braces (inside defineProps)
                let before = &content[..pos];
                let open_angle = before.matches('<').count();
                let close_angle = before.matches('>').count();
                let open_curly = before.matches('{').count();
                let close_curly = before.matches('}').count();

                // If we're inside angle brackets or curly braces, this is valid
                if open_angle > close_angle || open_curly > close_curly {
                    return Some(pos);
                }
            }
        }

        None
    }

    /// Find definition for a prop name used directly in template.
    /// Props are available directly in template (e.g., {{ propName }} or :attr="propName")
    fn find_prop_definition_by_name(
        ctx: &IdeContext<'_>,
        descriptor: &vize_atelier_sfc::SfcDescriptor,
        prop_name: &str,
    ) -> Option<GotoDefinitionResponse> {
        let script_setup = descriptor.script_setup.as_ref()?;

        // Analyze script to get prop definitions
        let mut analyzer = Analyzer::with_options(AnalyzerOptions {
            analyze_script: true,
            ..Default::default()
        });
        analyzer.analyze_script_setup(&script_setup.content);
        let croquis = analyzer.finish();

        // Check if this is a prop name
        let props = croquis.macros.props();
        let is_prop = props.iter().any(|p| p.name.as_str() == prop_name);

        if !is_prop {
            return None;
        }

        // Find the prop in defineProps type definition
        let content = &script_setup.content;
        if let Some(define_props_pos) = content.find("defineProps") {
            let after_define_props = &content[define_props_pos..];

            if let Some(prop_pos) = Self::find_prop_in_define_props(after_define_props, prop_name) {
                let sfc_offset = script_setup.loc.start + define_props_pos + prop_pos;
                let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: ctx.uri.clone(),
                    range: Range {
                        start: Position { line, character },
                        end: Position {
                            line,
                            character: character + prop_name.len() as u32,
                        },
                    },
                }));
            }

            // Fallback: jump to defineProps
            let sfc_offset = script_setup.loc.start + define_props_pos;
            let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

            return Some(GotoDefinitionResponse::Scalar(Location {
                uri: ctx.uri.clone(),
                range: Range {
                    start: Position { line, character },
                    end: Position {
                        line,
                        character: character + "defineProps".len() as u32,
                    },
                },
            }));
        }

        None
    }

    /// Check if the cursor is inside a Vue directive expression.
    /// Returns true for :prop="...", v-bind:prop="...", @event="...", v-on:event="...",
    /// v-if="...", v-for="...", v-show="...", v-model="...", {{ ... }}, etc.
    /// Returns false for plain HTML attributes like id="...", class="...", etc.
    fn is_in_vue_directive_expression(ctx: &IdeContext) -> bool {
        let content = &ctx.content;
        let offset = ctx.offset;

        // Check if we're inside a mustache expression {{ ... }}
        // Look backwards for {{ and forwards for }}
        let before = &content[..offset];
        let after = &content[offset..];

        // Find the last {{ before cursor
        if let Some(mustache_start) = before.rfind("{{") {
            // Check if there's a }} between the {{ and cursor
            let between = &content[mustache_start + 2..offset];
            if !between.contains("}}") {
                // Check if there's a }} after cursor
                if after.contains("}}") {
                    return true;
                }
            }
        }

        // Check if we're inside an attribute value
        // Scan backwards to find the attribute pattern
        let mut pos = offset;
        let mut in_quotes = false;
        let mut quote_char = '"';

        // Find the opening quote
        while pos > 0 {
            let c = content.as_bytes()[pos - 1] as char;
            if c == '"' || c == '\'' {
                in_quotes = true;
                quote_char = c;
                pos -= 1;
                break;
            }
            if c == '>' || c == '<' {
                return false;
            }
            pos -= 1;
        }

        if !in_quotes {
            return false;
        }

        // Now find what's before the quote (the attribute name and =)
        // Skip the = sign
        while pos > 0 && content.as_bytes()[pos - 1] == b'=' {
            pos -= 1;
        }

        // Get the attribute name by scanning backwards
        let attr_end = pos;
        while pos > 0 {
            let c = content.as_bytes()[pos - 1] as char;
            if c.is_whitespace() || c == '<' || c == '>' {
                break;
            }
            pos -= 1;
        }

        let attr_name = &content[pos..attr_end];

        // Check if this is a Vue directive
        // Directives start with: v-, :, @, #
        // Also include v-bind, v-on, v-if, v-for, v-show, v-model, v-slot, etc.
        if attr_name.starts_with(':')
            || attr_name.starts_with('@')
            || attr_name.starts_with('#')
            || attr_name.starts_with("v-")
        {
            // Verify we're still inside the quotes (not past the closing quote)
            let quote_start = attr_end + 1; // +1 for =
            if let Some(quote_end) = content[quote_start + 1..].find(quote_char) {
                let abs_quote_end = quote_start + 1 + quote_end;
                return offset <= abs_quote_end;
            }
        }

        false
    }

    /// Find the definition of a component by its tag name.
    fn find_component_definition(
        ctx: &IdeContext<'_>,
        tag_name: &str,
    ) -> Option<GotoDefinitionResponse> {
        // Parse SFC to get script content
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;

        // Create analyzer and analyze script
        let mut analyzer = Analyzer::with_options(AnalyzerOptions::full());

        if let Some(ref script_setup) = descriptor.script_setup {
            analyzer.analyze_script_setup(&script_setup.content);
        } else if let Some(ref script) = descriptor.script {
            analyzer.analyze_script_plain(&script.content);
        }

        let summary = analyzer.finish();

        // Try both PascalCase and kebab-case versions
        let pascal_name = kebab_to_pascal(tag_name);
        let names_to_try = [tag_name.to_string(), pascal_name];

        for name in &names_to_try {
            if let Some(binding_type) = summary.get_binding_type(name) {
                if binding_type == BindingType::ExternalModule {
                    // This is an imported component, find its import path
                    if let Some(import_path) = Self::find_import_path(ctx, name) {
                        // Resolve the import path to an absolute path
                        if let Some(resolved) = Self::resolve_import_path(ctx.uri, &import_path) {
                            if let Ok(file_uri) = Url::from_file_path(&resolved) {
                                return Some(GotoDefinitionResponse::Scalar(Location {
                                    uri: file_uri,
                                    range: Range {
                                        start: Position {
                                            line: 0,
                                            character: 0,
                                        },
                                        end: Position {
                                            line: 0,
                                            character: 0,
                                        },
                                    },
                                }));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Find the import path for a given component name.
    fn find_import_path(ctx: &IdeContext<'_>, component_name: &str) -> Option<String> {
        // Look for import statement pattern: import ComponentName from 'path'
        // or: import { ComponentName } from 'path'
        let content = &ctx.content;

        // Pattern 1: import ComponentName from 'path'
        let default_import_pattern = format!("import {} from", component_name);
        if let Some(pos) = content.find(&default_import_pattern) {
            return Self::extract_import_path_from_pos(content, pos + default_import_pattern.len());
        }

        // Pattern 2: import { ComponentName } from 'path'
        // or: import { X, ComponentName, Y } from 'path'
        let import_positions: Vec<_> = content.match_indices("import ").collect();
        for (pos, _) in import_positions {
            let rest = &content[pos..];
            if let Some(from_pos) = rest.find(" from") {
                let import_clause = &rest[7..from_pos]; // Skip "import "
                if import_clause.contains(&format!("{{ {}", component_name))
                    || import_clause.contains(&format!("{} }}", component_name))
                    || import_clause.contains(&format!(", {}", component_name))
                    || import_clause.contains(&format!("{},", component_name))
                    || import_clause == format!("{{ {} }}", component_name)
                {
                    return Self::extract_import_path_from_pos(rest, from_pos + 5);
                }
            }
        }

        None
    }

    /// Extract import path from a position after 'from'.
    fn extract_import_path_from_pos(content: &str, pos: usize) -> Option<String> {
        let rest = content[pos..].trim_start();

        // Find the quote character
        let quote_char = rest.chars().next()?;
        if quote_char != '\'' && quote_char != '"' {
            return None;
        }

        // Find the closing quote
        let path_start = 1;
        let path_end = rest[path_start..].find(quote_char)?;

        Some(rest[path_start..path_start + path_end].to_string())
    }

    /// Resolve an import path relative to the current file.
    fn resolve_import_path(current_uri: &Url, import_path: &str) -> Option<PathBuf> {
        let current_path = PathBuf::from(current_uri.path());
        let current_dir = current_path.parent()?;

        if import_path.starts_with("./") || import_path.starts_with("../") {
            // Relative import
            let resolved = current_dir.join(import_path);

            // Try adding extensions if not present
            if !resolved.exists() {
                let extensions = [".vue", ".ts", ".tsx", ".js", ".jsx"];
                for ext in extensions {
                    let with_ext = resolved.with_extension(&ext[1..]);
                    if with_ext.exists() {
                        return Some(with_ext);
                    }
                }
                // Try index files
                for ext in extensions {
                    let index_file = resolved.join(format!("index{}", ext));
                    if index_file.exists() {
                        return Some(index_file);
                    }
                }
            }

            Some(resolved.canonicalize().unwrap_or(resolved))
        } else {
            // Could be an alias or node_modules import
            // For now, we don't resolve these
            None
        }
    }

    /// Convert tsgo LspLocation to tower-lsp Location.
    #[cfg(feature = "native")]
    fn convert_lsp_locations(
        locations: Vec<vize_canon::LspLocation>,
        ctx: &IdeContext<'_>,
    ) -> GotoDefinitionResponse {
        if locations.len() == 1 {
            let loc = &locations[0];
            let uri = if loc.uri.starts_with("vize-virtual://") {
                // Map virtual URI back to SFC
                ctx.uri.clone()
            } else if let Ok(u) = Url::parse(&loc.uri) {
                u
            } else {
                ctx.uri.clone()
            };

            GotoDefinitionResponse::Scalar(Location {
                uri,
                range: Range {
                    start: Position {
                        line: loc.range.start.line,
                        character: loc.range.start.character,
                    },
                    end: Position {
                        line: loc.range.end.line,
                        character: loc.range.end.character,
                    },
                },
            })
        } else {
            let locs: Vec<Location> = locations
                .into_iter()
                .map(|loc| {
                    let uri = if loc.uri.starts_with("vize-virtual://") {
                        ctx.uri.clone()
                    } else if let Ok(u) = Url::parse(&loc.uri) {
                        u
                    } else {
                        ctx.uri.clone()
                    };
                    Location {
                        uri,
                        range: Range {
                            start: Position {
                                line: loc.range.start.line,
                                character: loc.range.start.character,
                            },
                            end: Position {
                                line: loc.range.end.line,
                                character: loc.range.end.character,
                            },
                        },
                    }
                })
                .collect();

            GotoDefinitionResponse::Array(locs)
        }
    }

    /// Find definition for a symbol in template context.
    fn definition_in_template(ctx: &IdeContext) -> Option<GotoDefinitionResponse> {
        // Get the word at the cursor position
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() {
            return None;
        }

        // Check if this is a props property access (e.g., props.title -> defineProps)
        if let Some(def) = Self::find_props_property_definition(ctx, &word) {
            return Some(def);
        }

        // Check if this is a component attribute (e.g., :disabled -> component's props)
        if let Some(def) = Self::find_component_prop_definition(ctx) {
            return Some(def);
        }

        // Parse SFC to get the actual script content (not virtual code)
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;

        // Check if this word is a prop name (props are available directly in template)
        // Only check if we're inside a Vue directive expression, not a plain HTML attribute
        if Self::is_in_vue_directive_expression(ctx) {
            if let Some(def) = Self::find_prop_definition_by_name(ctx, &descriptor, &word) {
                return Some(def);
            }
        }

        // Try to find the binding in script setup
        if let Some(ref script_setup) = descriptor.script_setup {
            let content = script_setup.content.as_ref();
            if let Some(binding_loc) = Self::find_binding_location_raw(content, &word) {
                // Convert offset within script content to SFC position
                // script_setup.loc.start is the offset of the first character of content
                let sfc_offset = script_setup.loc.start + binding_loc.offset;
                let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: ctx.uri.clone(),
                    range: Range {
                        start: Position { line, character },
                        end: Position {
                            line,
                            character: character + word.len() as u32,
                        },
                    },
                }));
            }
        }

        // Try regular script block
        if let Some(ref script) = descriptor.script {
            let content = script.content.as_ref();
            if let Some(binding_loc) = Self::find_binding_location_raw(content, &word) {
                let sfc_offset = script.loc.start + binding_loc.offset;
                let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: ctx.uri.clone(),
                    range: Range {
                        start: Position { line, character },
                        end: Position {
                            line,
                            character: character + word.len() as u32,
                        },
                    },
                }));
            }
        }

        None
    }

    /// Find definition for a symbol in script context.
    fn definition_in_script(ctx: &IdeContext) -> Option<GotoDefinitionResponse> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() {
            return None;
        }

        // Parse SFC to get the actual script content
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: ctx.uri.path().to_string(),
            ..Default::default()
        };

        let descriptor = vize_atelier_sfc::parse_sfc(&ctx.content, options).ok()?;

        // Determine which script block we're in
        let is_setup = matches!(ctx.block_type, Some(BlockType::ScriptSetup));

        let script_block = if is_setup {
            descriptor.script_setup.as_ref()
        } else {
            descriptor.script.as_ref()
        };

        if let Some(script) = script_block {
            let content = script.content.as_ref();
            if let Some(binding_loc) = Self::find_binding_location_raw(content, &word) {
                let sfc_offset = script.loc.start + binding_loc.offset;
                let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

                return Some(GotoDefinitionResponse::Scalar(Location {
                    uri: ctx.uri.clone(),
                    range: Range {
                        start: Position { line, character },
                        end: Position {
                            line,
                            character: character + word.len() as u32,
                        },
                    },
                }));
            }
        }

        None
    }

    /// Find definition for a symbol in style context.
    fn definition_in_style(ctx: &IdeContext) -> Option<GotoDefinitionResponse> {
        let word = Self::get_word_at_offset(&ctx.content, ctx.offset)?;

        if word.is_empty() {
            return None;
        }

        // Check for v-bind() references to script variables
        // Look backwards to see if we're inside v-bind()
        let before_cursor = &ctx.content[..ctx.offset];
        if before_cursor.contains("v-bind(") {
            // Parse SFC to get the actual script content
            let options = vize_atelier_sfc::SfcParseOptions {
                filename: ctx.uri.path().to_string(),
                ..Default::default()
            };

            if let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&ctx.content, options) {
                // Try to find the binding in script setup
                if let Some(ref script_setup) = descriptor.script_setup {
                    let content = script_setup.content.as_ref();
                    if let Some(binding_loc) = Self::find_binding_location_raw(content, &word) {
                        let sfc_offset = script_setup.loc.start + binding_loc.offset;
                        let (line, character) = Self::offset_to_position(&ctx.content, sfc_offset);

                        return Some(GotoDefinitionResponse::Scalar(Location {
                            uri: ctx.uri.clone(),
                            range: Range {
                                start: Position { line, character },
                                end: Position {
                                    line,
                                    character: character + word.len() as u32,
                                },
                            },
                        }));
                    }
                }
            }
        }

        None
    }

    /// Get the word at a given offset.
    fn get_word_at_offset(content: &str, offset: usize) -> Option<String> {
        if offset >= content.len() {
            return None;
        }

        let bytes = content.as_bytes();

        // If the character at offset is not a word character, return None
        if !Self::is_word_char(bytes[offset]) {
            return None;
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
            return None;
        }

        Some(String::from_utf8_lossy(&bytes[start..end]).to_string())
    }

    /// Check if a byte is a valid word character.
    #[inline]
    fn is_word_char(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
    }

    /// Find the location of a binding definition in raw script content (not virtual code).
    /// This is used for definition lookup from template to script.
    fn find_binding_location_raw(content: &str, name: &str) -> Option<BindingLocation> {
        // Search patterns for binding definitions
        let patterns = [
            format!("const {} ", name),
            format!("const {}=", name),
            format!("const {}:", name), // TypeScript annotation
            format!("let {} ", name),
            format!("let {}=", name),
            format!("let {}:", name),
            format!("var {} ", name),
            format!("var {}=", name),
            format!("function {}(", name),
            format!("function {} (", name),
        ];

        for pattern in &patterns {
            if let Some(pos) = content.find(pattern.as_str()) {
                // Find the actual name position within the pattern
                let name_offset = pattern.find(name).unwrap_or(0);
                let actual_offset = pos + name_offset;

                return Some(BindingLocation {
                    name: name.to_string(),
                    offset: actual_offset,
                    kind: BindingKind::from_pattern(pattern),
                });
            }
        }

        // Check for destructuring patterns: const { name } = ...
        let destructure_patterns = [
            format!("{{ {} }}", name),
            format!("{{ {}, ", name),
            format!("{{ {} ,", name),
            format!(", {} }}", name),
            format!(", {}, ", name),
            format!(" {} }}", name),
            format!(" {}, ", name),
        ];

        for pattern in &destructure_patterns {
            if let Some(pos) = content.find(pattern.as_str()) {
                let name_offset = pattern.find(name).unwrap_or(0);
                let actual_offset = pos + name_offset;

                return Some(BindingLocation {
                    name: name.to_string(),
                    offset: actual_offset,
                    kind: BindingKind::Destructure,
                });
            }
        }

        // Check for import patterns: import Name from 'path' or import { Name } from 'path'
        let import_patterns = [
            format!("import {} from", name),
            format!("import {{ {} }}", name),
            format!("import {{ {}, ", name),
            format!("import {{ {} ,", name),
            format!(", {} }}", name),
        ];

        for pattern in &import_patterns {
            if let Some(pos) = content.find(pattern.as_str()) {
                let name_offset = pattern.find(name).unwrap_or(0);
                let actual_offset = pos + name_offset;

                return Some(BindingLocation {
                    name: name.to_string(),
                    offset: actual_offset,
                    kind: BindingKind::Import,
                });
            }
        }

        None
    }

    /// Find the location of a binding definition in script content.
    /// Used by `extract_bindings_with_locations` and tests.
    #[allow(dead_code)]
    fn find_binding_location(
        content: &str,
        name: &str,
        _is_setup: bool,
    ) -> Option<BindingLocation> {
        // Skip the header comments in virtual code
        let content_start = Self::skip_virtual_header(content);
        let search_content = &content[content_start..];

        // Search patterns for binding definitions
        let patterns = [
            format!("const {} ", name),
            format!("const {}=", name),
            format!("let {} ", name),
            format!("let {}=", name),
            format!("var {} ", name),
            format!("var {}=", name),
            format!("function {}(", name),
            format!("function {} (", name),
        ];

        for pattern in &patterns {
            if let Some(pos) = search_content.find(pattern.as_str()) {
                // Find the actual name position within the pattern
                let name_offset = pattern.find(name).unwrap_or(0);
                let actual_offset = content_start + pos + name_offset;

                return Some(BindingLocation {
                    name: name.to_string(),
                    offset: actual_offset,
                    kind: BindingKind::from_pattern(pattern),
                });
            }
        }

        // Check for destructuring patterns: const { name } = ...
        let destructure_pattern = format!("{{ {}", name);
        if let Some(pos) = search_content.find(destructure_pattern.as_str()) {
            let name_offset = destructure_pattern.find(name).unwrap_or(0);
            let actual_offset = content_start + pos + name_offset;

            return Some(BindingLocation {
                name: name.to_string(),
                offset: actual_offset,
                kind: BindingKind::Destructure,
            });
        }

        // Check for: { name, ... } pattern with possible whitespace
        let destructure_patterns = [
            format!("{{ {}, ", name),
            format!("{{ {} }}", name),
            format!(", {} }}", name),
            format!(", {}, ", name),
        ];

        for pattern in &destructure_patterns {
            if let Some(pos) = search_content.find(pattern.as_str()) {
                let name_offset = pattern.find(name).unwrap_or(0);
                let actual_offset = content_start + pos + name_offset;

                return Some(BindingLocation {
                    name: name.to_string(),
                    offset: actual_offset,
                    kind: BindingKind::Destructure,
                });
            }
        }

        None
    }

    /// Skip virtual code header comments.
    fn skip_virtual_header(content: &str) -> usize {
        let mut offset = 0;
        for line in content.lines() {
            if line.starts_with("//") || line.trim().is_empty() {
                offset += line.len() + 1; // +1 for newline
            } else {
                break;
            }
        }
        offset
    }

    /// Convert byte offset to (line, character) position.
    fn offset_to_position(content: &str, offset: usize) -> (u32, u32) {
        let mut line = 0u32;
        let mut col = 0u32;
        let mut current_offset = 0usize;

        for ch in content.chars() {
            if current_offset >= offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }

            current_offset += ch.len_utf8();
        }

        (line, col)
    }
}

/// Location of a binding definition.
#[derive(Debug, Clone)]
pub struct BindingLocation {
    /// The binding name.
    pub name: String,
    /// Byte offset in the content.
    pub offset: usize,
    /// Kind of binding.
    pub kind: BindingKind,
}

/// Kind of binding definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// const declaration
    Const,
    /// let declaration
    Let,
    /// var declaration
    Var,
    /// function declaration
    Function,
    /// Destructuring pattern
    Destructure,
    /// Import binding
    Import,
    /// Unknown
    Unknown,
}

impl BindingKind {
    fn from_pattern(pattern: &str) -> Self {
        if pattern.starts_with("const") {
            BindingKind::Const
        } else if pattern.starts_with("let") {
            BindingKind::Let
        } else if pattern.starts_with("var") {
            BindingKind::Var
        } else if pattern.starts_with("function") {
            BindingKind::Function
        } else {
            BindingKind::Unknown
        }
    }
}

/// Extract bindings with their locations from script content.
pub fn extract_bindings_with_locations(content: &str, is_setup: bool) -> Vec<BindingLocation> {
    let mut bindings = Vec::new();

    if !is_setup {
        return bindings;
    }

    let content_start = DefinitionService::skip_virtual_header(content);
    let search_content = &content[content_start..];

    for line in search_content.lines() {
        let trimmed = line.trim();
        let line_start = search_content[..search_content.find(line).unwrap_or(0)].len();

        // const/let/var declarations
        for keyword in &["const ", "let ", "var "] {
            if trimmed.starts_with(keyword) {
                if let Some(rest) = trimmed.strip_prefix(keyword) {
                    // Handle destructuring: { a, b }
                    if rest.starts_with('{') {
                        if let Some(end) = rest.find('}') {
                            let inner = &rest[1..end];
                            for part in inner.split(',') {
                                let name = part.split(':').next().unwrap_or("").trim();
                                if !name.is_empty() && is_valid_identifier(name) {
                                    if let Some(name_pos) = line.find(name) {
                                        bindings.push(BindingLocation {
                                            name: name.to_string(),
                                            offset: content_start + line_start + name_pos,
                                            kind: BindingKind::Destructure,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    // Simple: const x = ...
                    else if let Some(name) = rest.split(['=', ':', ' ']).next() {
                        let name = name.trim();
                        if is_valid_identifier(name) {
                            if let Some(name_pos) = line.find(name) {
                                let kind = match *keyword {
                                    "const " => BindingKind::Const,
                                    "let " => BindingKind::Let,
                                    "var " => BindingKind::Var,
                                    _ => BindingKind::Unknown,
                                };
                                bindings.push(BindingLocation {
                                    name: name.to_string(),
                                    offset: content_start + line_start + name_pos,
                                    kind,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Function declarations
        if trimmed.starts_with("function ") {
            if let Some(rest) = trimmed.strip_prefix("function ") {
                if let Some(name) = rest.split('(').next() {
                    let name = name.trim();
                    if is_valid_identifier(name) {
                        if let Some(name_pos) = line.find(name) {
                            bindings.push(BindingLocation {
                                name: name.to_string(),
                                offset: content_start + line_start + name_pos,
                                kind: BindingKind::Function,
                            });
                        }
                    }
                }
            }
        }
    }

    bindings
}

/// Check if a string is a valid JavaScript identifier.
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_binding_location_const() {
        let content = r#"// Virtual TypeScript
// Generated

const message = ref('hello')
const count = ref(0)
"#;

        let loc = DefinitionService::find_binding_location(content, "message", true);
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.name, "message");
        assert_eq!(loc.kind, BindingKind::Const);
    }

    #[test]
    fn test_find_binding_location_function() {
        let content = r#"// Virtual TypeScript
// Generated

function handleClick() {
  console.log('clicked')
}
"#;

        let loc = DefinitionService::find_binding_location(content, "handleClick", true);
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.name, "handleClick");
        assert_eq!(loc.kind, BindingKind::Function);
    }

    #[test]
    fn test_find_binding_location_destructure() {
        let content = r#"// Virtual TypeScript
// Generated

const { data, error } = useFetch('/api')
"#;

        let loc = DefinitionService::find_binding_location(content, "data", true);
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.name, "data");
        assert_eq!(loc.kind, BindingKind::Destructure);
    }

    #[test]
    fn test_offset_to_position() {
        let content = "line1\nline2\nline3";

        // Start of line1
        let (line, col) = DefinitionService::offset_to_position(content, 0);
        assert_eq!(line, 0);
        assert_eq!(col, 0);

        // Middle of line1
        let (line, col) = DefinitionService::offset_to_position(content, 3);
        assert_eq!(line, 0);
        assert_eq!(col, 3);

        // Start of line2
        let (line, col) = DefinitionService::offset_to_position(content, 6);
        assert_eq!(line, 1);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_get_word_at_offset() {
        let content = "const message = 'hello'";

        let word = DefinitionService::get_word_at_offset(content, 6);
        assert_eq!(word, Some("message".to_string()));

        let word = DefinitionService::get_word_at_offset(content, 5);
        assert_eq!(word, None); // space

        let word = DefinitionService::get_word_at_offset(content, 0);
        assert_eq!(word, Some("const".to_string()));
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("foo"));
        assert!(is_valid_identifier("_foo"));
        assert!(is_valid_identifier("$foo"));
        assert!(is_valid_identifier("foo123"));
        assert!(!is_valid_identifier("123foo"));
        assert!(!is_valid_identifier(""));
    }

    #[test]
    fn test_find_binding_location_raw_const() {
        let content = r#"
import { ref } from 'vue'

const message = ref('hello')
const count = ref(0)
"#;

        let loc = DefinitionService::find_binding_location_raw(content, "message");
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.name, "message");
        assert_eq!(loc.kind, BindingKind::Const);

        // Verify the offset points to the actual 'message' position
        assert_eq!(&content[loc.offset..loc.offset + 7], "message");
    }

    #[test]
    fn test_find_binding_location_raw_import() {
        let content = r#"import { ref } from 'vue'
import MyComponent from './MyComponent.vue'
"#;

        let loc = DefinitionService::find_binding_location_raw(content, "MyComponent");
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.name, "MyComponent");
        assert_eq!(loc.kind, BindingKind::Import);

        // Verify the offset points to the actual 'MyComponent' position
        assert_eq!(&content[loc.offset..loc.offset + 11], "MyComponent");
    }

    #[test]
    fn test_find_binding_location_raw_destructure() {
        let content = r#"const { data, error } = useFetch('/api')
"#;

        let loc = DefinitionService::find_binding_location_raw(content, "data");
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.name, "data");
        assert_eq!(loc.kind, BindingKind::Destructure);

        // Verify the offset points to the actual 'data' position
        assert_eq!(&content[loc.offset..loc.offset + 4], "data");
    }

    #[test]
    fn test_find_prop_in_define_props() {
        let content = r#"defineProps<{
  title: string
  isSubmitting?: boolean
  count: number
}>()"#;

        // Find title
        let pos = DefinitionService::find_prop_in_define_props(content, "title");
        assert!(pos.is_some());

        // Find isSubmitting
        let pos = DefinitionService::find_prop_in_define_props(content, "isSubmitting");
        assert!(pos.is_some());

        // Non-existent prop
        let pos = DefinitionService::find_prop_in_define_props(content, "nonExistent");
        assert!(pos.is_none());
    }

    #[test]
    fn test_is_in_vue_directive_expression_detection() {
        // This is a unit-level conceptual test
        // The actual is_in_vue_directive_expression requires IdeContext
        // which we can't easily construct in tests.
        // We verify the attribute pattern matching logic here.

        // Vue directive patterns that should be detected:
        // :disabled="value", v-bind:disabled="value"
        // @click="handler", v-on:click="handler"
        // v-if="condition", v-for="item in items", v-show="visible"
        // v-model="data", v-slot:name="props"
        // #default="{ item }"

        // Plain HTML attributes that should NOT be detected:
        // id="value", class="value", href="value", src="value"

        let vue_attrs = [
            ":disabled",
            "@click",
            "v-if",
            "v-for",
            "v-model",
            "#default",
        ];
        let html_attrs = ["id", "class", "href", "src", "title"];

        for attr in vue_attrs {
            assert!(
                attr.starts_with(':')
                    || attr.starts_with('@')
                    || attr.starts_with('#')
                    || attr.starts_with("v-"),
                "Vue directive {} should match pattern",
                attr
            );
        }

        for attr in html_attrs {
            assert!(
                !attr.starts_with(':')
                    && !attr.starts_with('@')
                    && !attr.starts_with('#')
                    && !attr.starts_with("v-"),
                "HTML attribute {} should NOT match Vue pattern",
                attr
            );
        }
    }
}
