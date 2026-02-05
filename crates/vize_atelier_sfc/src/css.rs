//! CSS compilation using LightningCSS.
//!
//! Provides high-performance CSS parsing, transformation, and minification.
//! When the `native` feature is disabled (e.g., for wasm builds), a simple
//! passthrough implementation is used.

#[cfg(feature = "native")]
use lightningcss::printer::PrinterOptions;
#[cfg(feature = "native")]
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
#[cfg(feature = "native")]
use lightningcss::targets::{Browsers, Targets};
use serde::{Deserialize, Serialize};
use vize_carton::{Bump, BumpVec};

use crate::types::SfcStyleBlock;

/// CSS compilation options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssCompileOptions {
    /// Scope ID for scoped CSS (e.g., "data-v-abc123")
    #[serde(default)]
    pub scope_id: Option<String>,

    /// Whether to apply scoped CSS transformation
    #[serde(default)]
    pub scoped: bool,

    /// Whether to minify the output
    #[serde(default)]
    pub minify: bool,

    /// Whether to generate source maps
    #[serde(default)]
    pub source_map: bool,

    /// Browser targets for autoprefixing
    #[serde(default)]
    pub targets: Option<CssTargets>,

    /// Filename for error reporting
    #[serde(default)]
    pub filename: Option<String>,
}

/// Browser targets for CSS autoprefixing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssTargets {
    #[serde(default)]
    pub chrome: Option<u32>,
    #[serde(default)]
    pub firefox: Option<u32>,
    #[serde(default)]
    pub safari: Option<u32>,
    #[serde(default)]
    pub edge: Option<u32>,
    #[serde(default)]
    pub ios: Option<u32>,
    #[serde(default)]
    pub android: Option<u32>,
}

#[cfg(feature = "native")]
impl CssTargets {
    fn to_lightningcss_targets(&self) -> Targets {
        let mut browsers = Browsers::default();

        if let Some(v) = self.chrome {
            browsers.chrome = Some(version_to_u32(v));
        }
        if let Some(v) = self.firefox {
            browsers.firefox = Some(version_to_u32(v));
        }
        if let Some(v) = self.safari {
            browsers.safari = Some(version_to_u32(v));
        }
        if let Some(v) = self.edge {
            browsers.edge = Some(version_to_u32(v));
        }
        if let Some(v) = self.ios {
            browsers.ios_saf = Some(version_to_u32(v));
        }
        if let Some(v) = self.android {
            browsers.android = Some(version_to_u32(v));
        }

        Targets::from(browsers)
    }
}

/// Convert major version to LightningCSS format (major << 16)
#[cfg(feature = "native")]
fn version_to_u32(major: u32) -> u32 {
    major << 16
}

/// CSS compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssCompileResult {
    /// Compiled CSS code
    pub code: String,

    /// Source map (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,

    /// CSS variables found (from v-bind())
    #[serde(default)]
    pub css_vars: Vec<String>,

    /// Errors during compilation
    #[serde(default)]
    pub errors: Vec<String>,

    /// Warnings during compilation
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Compile CSS using LightningCSS (native feature enabled)
#[cfg(feature = "native")]
pub fn compile_css(css: &str, options: &CssCompileOptions) -> CssCompileResult {
    let bump = Bump::new();
    let filename = options.filename.as_deref().unwrap_or("style.css");

    // Extract v-bind() expressions before parsing
    let (processed_css, css_vars) = extract_and_transform_v_bind(&bump, css);

    // Apply scoped transformation if needed
    let scoped_css = if options.scoped {
        if let Some(ref scope_id) = options.scope_id {
            apply_scoped_css(&bump, processed_css, scope_id)
        } else {
            processed_css
        }
    } else {
        processed_css
    };

    // Apply targets for autoprefixing
    let targets = options
        .targets
        .as_ref()
        .map(|t| t.to_lightningcss_targets())
        .unwrap_or_default();

    // Parse and process CSS
    let (code, errors) = compile_css_internal(scoped_css, filename, options.minify, targets);

    CssCompileResult {
        code,
        map: None,
        css_vars,
        errors,
        warnings: vec![],
    }
}

/// Compile CSS (wasm fallback - no LightningCSS)
#[cfg(not(feature = "native"))]
pub fn compile_css(css: &str, options: &CssCompileOptions) -> CssCompileResult {
    let bump = Bump::new();

    // Extract v-bind() expressions before parsing
    let (processed_css, css_vars) = extract_and_transform_v_bind(&bump, css);

    // Apply scoped transformation if needed
    let scoped_css = if options.scoped {
        if let Some(ref scope_id) = options.scope_id {
            apply_scoped_css(&bump, processed_css, scope_id)
        } else {
            processed_css
        }
    } else {
        processed_css
    };

    CssCompileResult {
        code: scoped_css.to_string(),
        map: None,
        css_vars,
        errors: vec![],
        warnings: vec![],
    }
}

/// Internal CSS compilation with owned strings to avoid borrow issues
#[cfg(feature = "native")]
fn compile_css_internal(
    css: &str,
    filename: &str,
    minify: bool,
    targets: Targets,
) -> (String, Vec<String>) {
    let parser_options = ParserOptions {
        filename: filename.to_string(),
        ..Default::default()
    };

    let mut stylesheet = match StyleSheet::parse(css, parser_options) {
        Ok(ss) => ss,
        Err(e) => {
            let mut errors = Vec::with_capacity(1);
            let mut message = String::from("CSS parse error: ");
            message.push_str(&e.to_string());
            errors.push(message);
            return (css.to_string(), errors);
        }
    };

    // Minify if requested
    if minify {
        if let Err(e) = stylesheet.minify(lightningcss::stylesheet::MinifyOptions {
            targets,
            ..Default::default()
        }) {
            let mut errors = Vec::with_capacity(1);
            let mut message = String::from("CSS minify error: ");
            use std::fmt::Write as _;
            let _ = write!(&mut message, "{:?}", e);
            errors.push(message);
            return (css.to_string(), errors);
        }
    }

    // Print the CSS
    let printer_options = PrinterOptions {
        minify,
        targets,
        ..Default::default()
    };

    match stylesheet.to_css(printer_options) {
        Ok(result) => (result.code, vec![]),
        Err(e) => {
            let mut errors = Vec::with_capacity(1);
            let mut message = String::from("CSS print error: ");
            use std::fmt::Write as _;
            let _ = write!(&mut message, "{:?}", e);
            errors.push(message);
            (css.to_string(), errors)
        }
    }
}

/// Compile a style block
pub fn compile_style_block(style: &SfcStyleBlock, options: &CssCompileOptions) -> CssCompileResult {
    let mut opts = options.clone();
    opts.scoped = style.scoped || opts.scoped;
    compile_css(&style.content, &opts)
}

/// Extract v-bind() expressions and transform them to CSS variables
fn extract_and_transform_v_bind<'a>(bump: &'a Bump, css: &str) -> (&'a str, Vec<String>) {
    let css_bytes = css.as_bytes();
    let mut vars = Vec::new();
    let mut result = BumpVec::with_capacity_in(css_bytes.len() * 2, bump);
    let mut pos = 0;

    while pos < css_bytes.len() {
        if let Some(rel_pos) = find_bytes(&css_bytes[pos..], b"v-bind(") {
            let actual_pos = pos + rel_pos;
            let start = actual_pos + 7;

            if let Some(end) = find_byte(&css_bytes[start..], b')') {
                // Copy everything before v-bind(
                result.extend_from_slice(&css_bytes[pos..actual_pos]);

                // Extract expression
                let expr_bytes = &css_bytes[start..start + end];
                let expr_str = unsafe { std::str::from_utf8_unchecked(expr_bytes) }.trim();
                let expr_str = expr_str.trim_matches(|c| c == '"' || c == '\'');
                vars.push(expr_str.to_string());

                // Generate hash and write var(--hash-expr)
                result.extend_from_slice(b"var(--");
                write_v_bind_hash(&mut result, expr_str);
                result.push(b')');

                pos = start + end + 1;
            } else {
                result.extend_from_slice(&css_bytes[pos..]);
                break;
            }
        } else {
            result.extend_from_slice(&css_bytes[pos..]);
            break;
        }
    }

    // SAFETY: input is valid UTF-8, we only add ASCII bytes
    let result_str = unsafe { std::str::from_utf8_unchecked(bump.alloc_slice_copy(&result)) };
    (result_str, vars)
}

/// Write v-bind variable hash to output
fn write_v_bind_hash(out: &mut BumpVec<u8>, expr: &str) {
    let hash: u32 = expr
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));

    // Write hash as hex
    write_hex_u32(out, hash);
    out.push(b'-');

    // Write sanitized expression
    for b in expr.bytes() {
        match b {
            b'.' | b'[' | b']' | b'(' | b')' => out.push(b'_'),
            _ => out.push(b),
        }
    }
}

/// Write u32 as 8-digit hex
fn write_hex_u32(out: &mut BumpVec<u8>, val: u32) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push(HEX[((val >> 28) & 0xF) as usize]);
    out.push(HEX[((val >> 24) & 0xF) as usize]);
    out.push(HEX[((val >> 20) & 0xF) as usize]);
    out.push(HEX[((val >> 16) & 0xF) as usize]);
    out.push(HEX[((val >> 12) & 0xF) as usize]);
    out.push(HEX[((val >> 8) & 0xF) as usize]);
    out.push(HEX[((val >> 4) & 0xF) as usize]);
    out.push(HEX[(val & 0xF) as usize]);
}

/// Apply scoped CSS transformation
fn apply_scoped_css<'a>(bump: &'a Bump, css: &str, scope_id: &str) -> &'a str {
    let css_bytes = css.as_bytes();

    // Build attr_selector: [scope_id]
    let mut attr_selector = BumpVec::with_capacity_in(scope_id.len() + 2, bump);
    attr_selector.push(b'[');
    attr_selector.extend_from_slice(scope_id.as_bytes());
    attr_selector.push(b']');
    let attr_selector = bump.alloc_slice_copy(&attr_selector);

    let mut output = BumpVec::with_capacity_in(css_bytes.len() * 2, bump);
    let mut chars = css.char_indices().peekable();
    let mut in_selector = true;
    let mut in_string = false;
    let mut string_char = b'"';
    let mut in_comment = false;
    let mut brace_depth = 0u32;
    let mut last_selector_end = 0usize;
    let mut in_at_rule = false;
    let mut at_rule_depth = 0u32;

    while let Some((i, c)) = chars.next() {
        if in_comment {
            if c == '*' {
                if let Some(&(_, '/')) = chars.peek() {
                    chars.next();
                    in_comment = false;
                }
            }
            continue;
        }

        if in_string {
            if c as u8 == string_char {
                // Check for escape
                let prev_byte = if i > 0 { css_bytes[i - 1] } else { 0 };
                if prev_byte != b'\\' {
                    in_string = false;
                }
            }
            if !in_selector {
                output.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            }
            continue;
        }

        match c {
            '"' | '\'' => {
                in_string = true;
                string_char = c as u8;
                if !in_selector {
                    output.push(c as u8);
                }
            }
            '/' => {
                if let Some(&(_, '*')) = chars.peek() {
                    chars.next();
                    in_comment = true;
                } else if !in_selector {
                    output.push(b'/');
                }
            }
            '@' => {
                in_at_rule = true;
                output.push(b'@');
            }
            '{' => {
                brace_depth += 1;
                if in_at_rule {
                    at_rule_depth = brace_depth;
                    in_at_rule = false;
                    output.push(b'{');
                } else if in_selector && (brace_depth == 1 || brace_depth > at_rule_depth) {
                    // End of selector, apply scope
                    let selector_bytes = &css_bytes[last_selector_end..i];
                    let selector_str =
                        unsafe { std::str::from_utf8_unchecked(selector_bytes) }.trim();
                    scope_selector(&mut output, selector_str, attr_selector);
                    output.push(b'{');
                    in_selector = false;
                    last_selector_end = i + 1;
                } else {
                    output.push(b'{');
                }
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                output.push(b'}');
                if brace_depth == 0 || (at_rule_depth > 0 && brace_depth == at_rule_depth - 1) {
                    in_selector = true;
                    last_selector_end = i + 1;
                    if brace_depth < at_rule_depth {
                        at_rule_depth = 0;
                    }
                }
            }
            _ if in_selector => {
                // Still building selector, don't output yet
            }
            _ => {
                output.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
            }
        }
    }

    // Handle any remaining content
    if in_selector && last_selector_end < css_bytes.len() {
        output.extend_from_slice(&css_bytes[last_selector_end..]);
    }

    // SAFETY: input is valid UTF-8, we only add ASCII bytes
    unsafe { std::str::from_utf8_unchecked(bump.alloc_slice_copy(&output)) }
}

/// Add scope attribute to a selector
fn scope_selector(out: &mut BumpVec<u8>, selector: &str, attr_selector: &[u8]) {
    if selector.is_empty() {
        return;
    }

    // Handle at-rules that don't have selectors
    if selector.starts_with('@') {
        out.extend_from_slice(selector.as_bytes());
        return;
    }

    // Handle multiple selectors separated by comma
    let mut first = true;
    for part in selector.split(',') {
        if !first {
            out.extend_from_slice(b", ");
        }
        first = false;
        scope_single_selector(out, part.trim(), attr_selector);
    }
}

/// Add scope attribute to a single selector
fn scope_single_selector(out: &mut BumpVec<u8>, selector: &str, attr_selector: &[u8]) {
    if selector.is_empty() {
        return;
    }

    // Handle :deep(), :slotted(), :global()
    if let Some(pos) = selector.find(":deep(") {
        transform_deep(out, selector, pos, attr_selector);
        return;
    }

    if let Some(pos) = selector.find(":slotted(") {
        transform_slotted(out, selector, pos, attr_selector);
        return;
    }

    if let Some(pos) = selector.find(":global(") {
        transform_global(out, selector, pos);
        return;
    }

    // Find the last simple selector to append the attribute
    let parts: Vec<&str> = selector.split_whitespace().collect();
    if parts.is_empty() {
        out.extend_from_slice(selector.as_bytes());
        return;
    }

    // Add scope to the last part
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }

        if i == parts.len() - 1 {
            // Last part - add scope
            add_scope_to_element(out, part, attr_selector);
        } else {
            out.extend_from_slice(part.as_bytes());
        }
    }
}

/// Add scope attribute to an element selector
fn add_scope_to_element(out: &mut BumpVec<u8>, selector: &str, attr_selector: &[u8]) {
    let bytes = selector.as_bytes();

    // Handle pseudo-elements (::before, ::after, etc.)
    if let Some(pseudo_pos) = find_bytes(bytes, b"::") {
        out.extend_from_slice(&bytes[..pseudo_pos]);
        out.extend_from_slice(attr_selector);
        out.extend_from_slice(&bytes[pseudo_pos..]);
        return;
    }

    // Handle pseudo-classes (:hover, :focus, etc.)
    if let Some(pseudo_pos) = rfind_byte(bytes, b':') {
        if pseudo_pos > 0 && bytes[pseudo_pos - 1] != b'\\' {
            out.extend_from_slice(&bytes[..pseudo_pos]);
            out.extend_from_slice(attr_selector);
            out.extend_from_slice(&bytes[pseudo_pos..]);
            return;
        }
    }

    out.extend_from_slice(bytes);
    out.extend_from_slice(attr_selector);
}

/// Transform :deep() to descendant selector
fn transform_deep(out: &mut BumpVec<u8>, selector: &str, start: usize, attr_selector: &[u8]) {
    let before = &selector[..start];
    let after = &selector[start + 6..];

    if let Some(end) = find_matching_paren(after) {
        let inner = &after[..end];
        let rest = &after[end + 1..];

        if before.is_empty() {
            out.extend_from_slice(attr_selector);
        } else {
            out.extend_from_slice(before.trim().as_bytes());
            out.extend_from_slice(attr_selector);
        }
        out.push(b' ');
        out.extend_from_slice(inner.as_bytes());
        out.extend_from_slice(rest.as_bytes());
    } else {
        out.extend_from_slice(selector.as_bytes());
    }
}

/// Transform :slotted() for slot content
fn transform_slotted(out: &mut BumpVec<u8>, selector: &str, start: usize, attr_selector: &[u8]) {
    let after = &selector[start + 9..];

    if let Some(end) = find_matching_paren(after) {
        let inner = &after.as_bytes()[..end];
        let rest = &after.as_bytes()[end + 1..];

        out.extend_from_slice(inner);
        // Convert [data-v-xxx] to [data-v-xxx-s] for slotted styles
        if attr_selector.last() == Some(&b']') {
            out.extend_from_slice(&attr_selector[..attr_selector.len() - 1]);
            out.extend_from_slice(b"-s]");
        } else {
            out.extend_from_slice(attr_selector);
            out.extend_from_slice(b"-s");
        }
        out.extend_from_slice(rest);
    } else {
        out.extend_from_slice(selector.as_bytes());
    }
}

/// Transform :global() to unscoped
fn transform_global(out: &mut BumpVec<u8>, selector: &str, start: usize) {
    let before = &selector[..start];
    let after = &selector[start + 8..];

    if let Some(end) = find_matching_paren(after) {
        let inner = &after[..end];
        let rest = &after[end + 1..];

        out.extend_from_slice(before.as_bytes());
        out.extend_from_slice(inner.as_bytes());
        out.extend_from_slice(rest.as_bytes());
    } else {
        out.extend_from_slice(selector.as_bytes());
    }
}

/// Find the matching closing parenthesis
fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 1u32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Find byte sequence in slice
#[inline]
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Find single byte in slice
#[inline]
fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

/// Reverse find single byte in slice
#[inline]
fn rfind_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().rposition(|&b| b == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_css() {
        let css = ".foo { color: red; }";
        let result = compile_css(css, &CssCompileOptions::default());
        assert!(result.errors.is_empty());
        assert!(result.code.contains(".foo"));
        assert!(result.code.contains("color"));
    }

    #[test]
    fn test_compile_scoped_css() {
        let css = ".foo { color: red; }";
        let result = compile_css(
            css,
            &CssCompileOptions {
                scoped: true,
                scope_id: Some("data-v-123".to_string()),
                ..Default::default()
            },
        );
        assert!(result.errors.is_empty());
        assert!(result.code.contains("[data-v-123]"));
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_compile_minified_css() {
        let css = ".foo {\n  color: red;\n  background: blue;\n}";
        let result = compile_css(
            css,
            &CssCompileOptions {
                minify: true,
                ..Default::default()
            },
        );
        assert!(result.errors.is_empty());
        // Minified should have no newlines in simple case
        assert!(!result.code.contains('\n') || result.code.lines().count() == 1);
    }

    #[test]
    fn test_v_bind_extraction() {
        let bump = Bump::new();
        let css = ".foo { color: v-bind(color); background: v-bind('bgColor'); }";
        let (transformed, vars) = extract_and_transform_v_bind(&bump, css);
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&"color".to_string()));
        assert!(vars.contains(&"bgColor".to_string()));
        assert!(transformed.contains("var(--"));
    }

    #[test]
    fn test_scope_deep() {
        let bump = Bump::new();
        let mut out = BumpVec::new_in(&bump);
        transform_deep(&mut out, ":deep(.child)", 0, b"[data-v-123]");
        let result = unsafe { std::str::from_utf8_unchecked(&out) };
        assert_eq!(result, "[data-v-123] .child");
    }

    #[test]
    fn test_scope_global() {
        let bump = Bump::new();
        let mut out = BumpVec::new_in(&bump);
        transform_global(&mut out, ":global(.foo)", 0);
        let result = unsafe { std::str::from_utf8_unchecked(&out) };
        assert_eq!(result, ".foo");
    }

    #[test]
    fn test_scope_slotted() {
        let bump = Bump::new();
        let mut out = BumpVec::new_in(&bump);
        transform_slotted(&mut out, ":slotted(.child)", 0, b"[data-v-123]");
        let result = unsafe { std::str::from_utf8_unchecked(&out) };
        assert_eq!(result, ".child[data-v-123-s]");
    }

    #[test]
    fn test_scope_slotted_with_pseudo() {
        let bump = Bump::new();
        let mut out = BumpVec::new_in(&bump);
        transform_slotted(&mut out, ":slotted(.child):hover", 0, b"[data-v-abc]");
        let result = unsafe { std::str::from_utf8_unchecked(&out) };
        assert_eq!(result, ".child[data-v-abc-s]:hover");
    }

    #[test]
    fn test_scope_slotted_complex() {
        let bump = Bump::new();
        let mut out = BumpVec::new_in(&bump);
        transform_slotted(&mut out, ":slotted(div.foo)", 0, b"[data-v-12345678]");
        let result = unsafe { std::str::from_utf8_unchecked(&out) };
        assert_eq!(result, "div.foo[data-v-12345678-s]");
    }

    #[test]
    fn test_scope_with_pseudo_element() {
        let bump = Bump::new();
        let mut out = BumpVec::new_in(&bump);
        add_scope_to_element(&mut out, ".foo::before", b"[data-v-123]");
        let result = unsafe { std::str::from_utf8_unchecked(&out) };
        assert_eq!(result, ".foo[data-v-123]::before");
    }

    #[test]
    fn test_scope_with_pseudo_class() {
        let bump = Bump::new();
        let mut out = BumpVec::new_in(&bump);
        add_scope_to_element(&mut out, ".foo:hover", b"[data-v-123]");
        let result = unsafe { std::str::from_utf8_unchecked(&out) };
        assert_eq!(result, ".foo[data-v-123]:hover");
    }

    #[test]
    #[cfg(feature = "native")]
    fn test_compile_with_targets() {
        let css = ".foo { display: flex; }";
        let result = compile_css(
            css,
            &CssCompileOptions {
                targets: Some(CssTargets {
                    chrome: Some(80),
                    ..Default::default()
                }),
                ..Default::default()
            },
        );
        assert!(result.errors.is_empty());
        assert!(result.code.contains("flex"));
    }

    #[test]
    fn test_scoped_css_with_quoted_font_family() {
        let css = ".foo { font-family: 'JetBrains Mono', monospace; }";
        let result = compile_css(
            css,
            &CssCompileOptions {
                scoped: true,
                scope_id: Some("data-v-123".to_string()),
                ..Default::default()
            },
        );
        println!("Result: {}", result.code);
        assert!(result.errors.is_empty());
        // Note: LightningCSS may remove quotes from font names
        assert!(
            result.code.contains("JetBrains Mono"),
            "Expected font name in: {}",
            result.code
        );
        assert!(result.code.contains("monospace"));
    }

    #[test]
    fn test_apply_scoped_css_with_quoted_string() {
        let bump = Bump::new();
        // Test the raw scoping function without LightningCSS
        let css = ".foo { font-family: 'JetBrains Mono', monospace; }";
        let result = apply_scoped_css(&bump, css, "data-v-123");
        println!("Scoped result: {}", result);
        assert!(
            result.contains("'JetBrains Mono'"),
            "Expected quoted font name in: {}",
            result
        );
        assert!(result.contains("monospace"));
    }
}
