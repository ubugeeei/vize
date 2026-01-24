//! Document link provider.
//!
//! Provides clickable links for:
//! - Import statements in script blocks
//! - src attributes on script/style/template blocks
//! - CSS @import statements

use std::path::Path;

use tower_lsp::lsp_types::{DocumentLink, Position, Range, Url};

use super::offset_to_position;

/// Document link service.
pub struct DocumentLinkService;

impl DocumentLinkService {
    /// Get document links for a file.
    pub fn get_links(content: &str, uri: &Url) -> Vec<DocumentLink> {
        let mut links = Vec::new();

        let options = vize_atelier_sfc::SfcParseOptions {
            filename: uri.path().to_string(),
            ..Default::default()
        };

        let Ok(descriptor) = vize_atelier_sfc::parse_sfc(content, options) else {
            return links;
        };

        let base_path = uri.to_file_path().ok();

        // Collect links from script setup
        if let Some(ref script_setup) = descriptor.script_setup {
            // src attribute
            if let Some(ref src) = script_setup.src {
                if let Some((start, end)) =
                    Self::find_src_attr_range(content, script_setup.loc.start)
                {
                    if let Some(target) = Self::resolve_path(src, base_path.as_deref()) {
                        links.push(Self::create_link(content, start, end, target));
                    }
                }
            }

            // Import statements
            Self::collect_import_links(
                &script_setup.content,
                script_setup.loc.start,
                content,
                base_path.as_deref(),
                &mut links,
            );
        }

        // Collect links from script
        if let Some(ref script) = descriptor.script {
            // src attribute
            if let Some(ref src) = script.src {
                if let Some((start, end)) = Self::find_src_attr_range(content, script.loc.start) {
                    if let Some(target) = Self::resolve_path(src, base_path.as_deref()) {
                        links.push(Self::create_link(content, start, end, target));
                    }
                }
            }

            // Import statements
            Self::collect_import_links(
                &script.content,
                script.loc.start,
                content,
                base_path.as_deref(),
                &mut links,
            );
        }

        // Collect links from template src
        if let Some(ref template) = descriptor.template {
            if let Some(ref src) = template.src {
                if let Some((start, end)) = Self::find_src_attr_range(content, template.loc.start) {
                    if let Some(target) = Self::resolve_path(src, base_path.as_deref()) {
                        links.push(Self::create_link(content, start, end, target));
                    }
                }
            }
        }

        // Collect links from styles
        for style in &descriptor.styles {
            // src attribute
            if let Some(ref src) = style.src {
                if let Some((start, end)) = Self::find_src_attr_range(content, style.loc.start) {
                    if let Some(target) = Self::resolve_path(src, base_path.as_deref()) {
                        links.push(Self::create_link(content, start, end, target));
                    }
                }
            }

            // @import statements
            Self::collect_css_import_links(
                &style.content,
                style.loc.start,
                content,
                base_path.as_deref(),
                &mut links,
            );
        }

        links
    }

    /// Collect import statement links from script content.
    fn collect_import_links(
        script: &str,
        base_offset: usize,
        full_content: &str,
        base_path: Option<&Path>,
        links: &mut Vec<DocumentLink>,
    ) {
        // Match: import ... from "path" or import ... from 'path'
        // Match: import "path" or import 'path' (side-effect imports)
        // Match: export ... from "path" or export ... from 'path'
        let mut pos = 0;
        let bytes = script.as_bytes();

        while pos < script.len() {
            // Find "import" or "export"
            let import_start = script[pos..].find("import");
            let export_start = script[pos..].find("export");

            let keyword_start = match (import_start, export_start) {
                (Some(i), Some(e)) => Some(pos + i.min(e)),
                (Some(i), None) => Some(pos + i),
                (None, Some(e)) => Some(pos + e),
                (None, None) => None,
            };

            let Some(start) = keyword_start else {
                break;
            };

            // Check it's at word boundary
            if start > 0 && Self::is_ident_char(bytes[start - 1] as char) {
                pos = start + 1;
                continue;
            }

            // Find the end of this statement (semicolon or newline)
            let stmt_end = script[start..]
                .find([';', '\n'])
                .map(|i| start + i)
                .unwrap_or(script.len());

            let stmt = &script[start..stmt_end];

            // Find string literal (the path)
            if let Some((path, rel_start, rel_end)) = Self::extract_import_path(stmt) {
                // Only link relative imports (start with . or /)
                if path.starts_with('.') || path.starts_with('/') {
                    if let Some(target) = Self::resolve_path(&path, base_path) {
                        let abs_start = base_offset + start + rel_start;
                        let abs_end = base_offset + start + rel_end;
                        links.push(Self::create_link(full_content, abs_start, abs_end, target));
                    }
                }
            }

            pos = stmt_end + 1;
        }
    }

    /// Extract import path from an import/export statement.
    /// Returns (path, start, end_offset) where offsets are relative to stmt start.
    fn extract_import_path(stmt: &str) -> Option<(String, usize, usize)> {
        // Look for 'from' keyword followed by string
        if let Some(from_pos) = stmt.find(" from ") {
            let after_from = &stmt[from_pos + 6..];
            return Self::extract_string_literal(after_from)
                .map(|(path, s, e)| (path, from_pos + 6 + s, from_pos + 6 + e));
        }

        // Side-effect import: import "path"
        if stmt.starts_with("import ") || stmt.starts_with("import\t") {
            let after_import = &stmt[7..];
            // Check if it's a side-effect import (no identifier before the string)
            let trimmed = after_import.trim_start();
            if trimmed.starts_with('"') || trimmed.starts_with('\'') {
                let ws_len = after_import.len() - trimmed.len();
                return Self::extract_string_literal(trimmed)
                    .map(|(path, s, e)| (path, 7 + ws_len + s, 7 + ws_len + e));
            }
        }

        None
    }

    /// Extract string literal from text.
    /// Returns (content, start, end_offset) where offsets include quotes.
    fn extract_string_literal(text: &str) -> Option<(String, usize, usize)> {
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return None;
        }

        let quote = bytes[0] as char;
        if quote != '"' && quote != '\'' {
            return None;
        }

        // Find closing quote
        let mut i = 1;
        while i < bytes.len() {
            if bytes[i] == quote as u8 && (i == 1 || bytes[i - 1] != b'\\') {
                let content = text[1..i].to_string();
                return Some((content, 0, i + 1));
            }
            i += 1;
        }

        None
    }

    /// Collect CSS @import links.
    fn collect_css_import_links(
        css: &str,
        base_offset: usize,
        full_content: &str,
        base_path: Option<&Path>,
        links: &mut Vec<DocumentLink>,
    ) {
        // Match: @import "path" or @import 'path' or @import url("path")
        let mut pos = 0;

        while let Some(import_pos) = css[pos..].find("@import") {
            let start = pos + import_pos;
            let after_import = &css[start + 7..];
            let trimmed = after_import.trim_start();
            let ws_len = after_import.len() - trimmed.len();

            // @import url("path") or @import url('path')
            if let Some(url_content) = trimmed.strip_prefix("url(") {
                if let Some((path, s, e)) = Self::extract_string_literal(url_content.trim_start()) {
                    let inner_ws = url_content.len() - url_content.trim_start().len();
                    if path.starts_with('.') || path.starts_with('/') {
                        if let Some(target) = Self::resolve_path(&path, base_path) {
                            let abs_start = base_offset + start + 7 + ws_len + 4 + inner_ws + s;
                            let abs_end = base_offset + start + 7 + ws_len + 4 + inner_ws + e;
                            links.push(Self::create_link(full_content, abs_start, abs_end, target));
                        }
                    }
                }
            }
            // @import "path" or @import 'path'
            else if let Some((path, s, e)) = Self::extract_string_literal(trimmed) {
                if path.starts_with('.') || path.starts_with('/') {
                    if let Some(target) = Self::resolve_path(&path, base_path) {
                        let abs_start = base_offset + start + 7 + ws_len + s;
                        let abs_end = base_offset + start + 7 + ws_len + e;
                        links.push(Self::create_link(full_content, abs_start, abs_end, target));
                    }
                }
            }

            pos = start + 8;
        }
    }

    /// Find src attribute range in the opening tag.
    fn find_src_attr_range(content: &str, tag_start: usize) -> Option<(usize, usize)> {
        // Find the end of opening tag (>)
        let tag_content = &content[tag_start..];
        let tag_end = tag_content.find('>')?;
        let tag = &tag_content[..tag_end];

        // Find src="..." or src='...'
        let src_pos = tag.find("src=")?;
        let after_src = &tag[src_pos + 4..];

        let quote = after_src.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }

        let value_start = src_pos + 5; // src=" plus quote
        let value_end = after_src[1..].find(quote)? + 1; // content length

        Some((tag_start + value_start, tag_start + value_start + value_end))
    }

    /// Resolve a relative path to an absolute URL.
    fn resolve_path(path: &str, base_path: Option<&Path>) -> Option<Url> {
        let base = base_path?;
        let parent = base.parent()?;

        // Clean up the path
        let clean_path = path.trim_matches(|c| c == '"' || c == '\'');

        let resolved = if let Some(stripped) = clean_path.strip_prefix('/') {
            // Absolute path from project root - try to find it
            // For now, treat as relative to current file's directory
            parent.join(stripped)
        } else {
            parent.join(clean_path)
        };

        // Try common extensions if file doesn't exist
        let candidates = [
            resolved.clone(),
            resolved.with_extension("ts"),
            resolved.with_extension("js"),
            resolved.with_extension("vue"),
            resolved.with_extension("tsx"),
            resolved.with_extension("jsx"),
            resolved.join("index.ts"),
            resolved.join("index.js"),
            resolved.join("index.vue"),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                return Url::from_file_path(candidate.canonicalize().ok()?).ok();
            }
        }

        // Return original path even if it doesn't exist (user might create it)
        Url::from_file_path(&resolved).ok()
    }

    /// Create a document link.
    fn create_link(content: &str, start: usize, end: usize, target: Url) -> DocumentLink {
        let (start_line, start_char) = offset_to_position(content, start);
        let (end_line, end_char) = offset_to_position(content, end);

        DocumentLink {
            range: Range {
                start: Position {
                    line: start_line,
                    character: start_char,
                },
                end: Position {
                    line: end_line,
                    character: end_char,
                },
            },
            target: Some(target),
            tooltip: None,
            data: None,
        }
    }

    fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_' || c == '$'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_import_path() {
        // Regular import
        let stmt = r#"import { ref } from "./utils""#;
        let (path, _, _) = DocumentLinkService::extract_import_path(stmt).unwrap();
        assert_eq!(path, "./utils");

        // Side-effect import
        let stmt = r#"import "./styles.css""#;
        let (path, _, _) = DocumentLinkService::extract_import_path(stmt).unwrap();
        assert_eq!(path, "./styles.css");

        // Export from
        let stmt = r#"export { foo } from './foo'"#;
        let (path, _, _) = DocumentLinkService::extract_import_path(stmt).unwrap();
        assert_eq!(path, "./foo");
    }

    #[test]
    fn test_extract_string_literal() {
        let (content, start, end) =
            DocumentLinkService::extract_string_literal(r#""hello""#).unwrap();
        assert_eq!(content, "hello");
        assert_eq!(start, 0);
        assert_eq!(end, 7);

        let (content, _, _) = DocumentLinkService::extract_string_literal("'world'").unwrap();
        assert_eq!(content, "world");
    }
}
