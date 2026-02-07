//! IDE features for the LSP server.
//!
//! This module provides core IDE functionality including:
//! - Diagnostics aggregation from multiple sources
//! - Hover information provider
//! - Code completion provider
//! - Go to definition
//! - Find references
//! - Code actions (quick fixes)
//! - Type checking and type information
//! - Rename refactoring
//! - Semantic tokens
//! - Code lens
//! - Workspace symbols

pub mod code_action;
pub mod code_lens;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod document_link;
pub mod hover;
pub mod inlay_hint;
pub mod references;
pub mod rename;
pub mod semantic_tokens;
pub mod type_service;
pub mod workspace_symbols;

pub use code_action::CodeActionService;
pub use code_lens::CodeLensService;
pub use completion::{trigger_characters, CompletionService, TRIGGER_CHARACTERS};
pub use definition::{BindingKind, BindingLocation, DefinitionService};
pub use diagnostics::{sources, DiagnosticBuilder, DiagnosticService, Severity};
pub use document_link::DocumentLinkService;
pub use hover::{HoverBuilder, HoverService};
pub use inlay_hint::InlayHintService;
pub use references::ReferencesService;
pub use rename::RenameService;
pub use semantic_tokens::{SemanticTokensService, TokenModifier, TokenType};
pub use type_service::{LspTypeCheckOptions, TypeService};
pub use workspace_symbols::WorkspaceSymbolsService;

use tower_lsp::lsp_types::Url;

use crate::server::ServerState;
use crate::virtual_code::{find_block_at_offset, BlockType, VirtualDocuments};

// =============================================================================
// Position conversion utilities
// =============================================================================

/// Convert byte offset to (line, character) position in a document.
#[inline]
pub fn offset_to_position(content: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut current = 0usize;

    for ch in content.chars() {
        if current >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current += ch.len_utf8();
    }

    (line, col)
}

/// Convert (line, character) position to byte offset in a document.
#[inline]
pub fn position_to_offset(content: &str, line: u32, character: u32) -> Option<usize> {
    let mut current_line = 0u32;
    let mut current_col = 0u32;
    let mut offset = 0usize;

    for ch in content.chars() {
        if current_line == line && current_col == character {
            return Some(offset);
        }
        if ch == '\n' {
            if current_line == line {
                // Reached end of target line
                return Some(offset);
            }
            current_line += 1;
            current_col = 0;
        } else {
            current_col += 1;
        }
        offset += ch.len_utf8();
    }

    // Handle end of file
    if current_line == line && current_col == character {
        return Some(offset);
    }

    None
}

// =============================================================================
// Component name conversion utilities
// =============================================================================

/// Convert kebab-case to PascalCase.
/// Example: "my-component" -> "MyComponent"
pub fn kebab_to_pascal(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut capitalize_next = true;

    for ch in name.chars() {
        if ch == '-' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert PascalCase to kebab-case.
/// Example: "MyComponent" -> "my-component"
pub fn pascal_to_kebab(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);

    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }

    result
}

/// Check if a tag name is a component (starts with uppercase or contains hyphen).
#[inline]
pub fn is_component_tag(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let first = name.chars().next().unwrap();
    first.is_ascii_uppercase() || name.contains('-')
}

/// Context for IDE operations.
pub struct IdeContext<'a> {
    /// Server state
    pub state: &'a ServerState,
    /// Document URI
    pub uri: &'a Url,
    /// Document content
    pub content: String,
    /// Cursor offset in the document
    pub offset: usize,
    /// Which block the cursor is in
    pub block_type: Option<BlockType>,
    /// Virtual documents for this file
    pub virtual_docs: Option<dashmap::mapref::one::Ref<'a, Url, VirtualDocuments>>,
}

impl<'a> IdeContext<'a> {
    /// Create a new IDE context.
    pub fn new(state: &'a ServerState, uri: &'a Url, offset: usize) -> Option<Self> {
        let doc = state.documents.get(uri)?;
        let content = doc.text();

        // Parse SFC to determine block type
        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let block_type = if let Ok(descriptor) = vize_atelier_sfc::parse_sfc(&content, options) {
            find_block_at_offset(&descriptor, offset)
        } else {
            None
        };

        let virtual_docs = state.get_virtual_docs(uri);

        Some(Self {
            state,
            uri,
            content,
            offset,
            block_type,
            virtual_docs,
        })
    }

    /// Check if cursor is in template block.
    #[inline]
    pub fn is_in_template(&self) -> bool {
        matches!(self.block_type, Some(BlockType::Template))
    }

    /// Check if cursor is in script block.
    #[inline]
    pub fn is_in_script(&self) -> bool {
        matches!(
            self.block_type,
            Some(BlockType::Script) | Some(BlockType::ScriptSetup)
        )
    }

    /// Check if cursor is in style block.
    #[inline]
    pub fn is_in_style(&self) -> bool {
        matches!(self.block_type, Some(BlockType::Style(_)))
    }

    /// Check if cursor is in an art custom block.
    #[inline]
    pub fn is_in_art(&self) -> bool {
        matches!(self.block_type, Some(BlockType::Art(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_position() {
        let content = "line1\nline2\nline3";

        assert_eq!(offset_to_position(content, 0), (0, 0));
        assert_eq!(offset_to_position(content, 5), (0, 5));
        assert_eq!(offset_to_position(content, 6), (1, 0));
        assert_eq!(offset_to_position(content, 8), (1, 2));
        assert_eq!(offset_to_position(content, 12), (2, 0));
    }

    #[test]
    fn test_position_to_offset() {
        let content = "line1\nline2\nline3";

        assert_eq!(position_to_offset(content, 0, 0), Some(0));
        assert_eq!(position_to_offset(content, 0, 5), Some(5));
        assert_eq!(position_to_offset(content, 1, 0), Some(6));
        assert_eq!(position_to_offset(content, 1, 2), Some(8));
        assert_eq!(position_to_offset(content, 2, 0), Some(12));
    }

    #[test]
    fn test_kebab_to_pascal() {
        assert_eq!(kebab_to_pascal("my-component"), "MyComponent");
        assert_eq!(kebab_to_pascal("button"), "Button");
        assert_eq!(kebab_to_pascal("v-for-item"), "VForItem");
        assert_eq!(kebab_to_pascal("a-b-c"), "ABC");
    }

    #[test]
    fn test_pascal_to_kebab() {
        assert_eq!(pascal_to_kebab("MyComponent"), "my-component");
        assert_eq!(pascal_to_kebab("Button"), "button");
        assert_eq!(pascal_to_kebab("VForItem"), "v-for-item");
        assert_eq!(pascal_to_kebab("ABC"), "a-b-c");
    }

    #[test]
    fn test_is_component_tag() {
        // PascalCase components
        assert!(is_component_tag("MyComponent"));
        assert!(is_component_tag("Button"));

        // kebab-case components
        assert!(is_component_tag("my-component"));
        assert!(is_component_tag("v-button"));

        // HTML elements (not components)
        assert!(!is_component_tag("div"));
        assert!(!is_component_tag("span"));
        assert!(!is_component_tag("button"));
    }
}
