//! High-performance template formatting for Vue SFC.
//!
//! Features:
//! - Proper indentation and nesting
//! - Directive shorthand normalization (`v-bind:` → `:`, `v-on:` → `@`, `v-slot:` → `#`)
//! - Interpolation spacing normalization (`{{expr}}` → `{{ expr }}`)
//! - JS expression formatting in directive values via oxc_formatter
//! - Attribute sorting following Vue style guide order
//! - `single_attribute_per_line` support with `bracket_same_line`

use crate::error::FormatError;
use crate::options::{AttributeSortOrder, FormatOptions};
use crate::script;

/// Parsed attribute with structured information for sorting and rendering.
#[derive(Debug, Clone)]
struct ParsedAttribute {
    /// Normalized attribute name (after shorthand conversion)
    name: String,
    /// Attribute value (without quotes), None for boolean attrs like `v-else`
    value: Option<String>,
    /// Sort priority (lower = earlier in output)
    priority: u8,
    /// Original index in the source for stable sorting
    original_index: usize,
}

/// Format Vue template content
#[inline]
pub fn format_template_content(
    source: &str,
    options: &FormatOptions,
) -> Result<String, FormatError> {
    let bytes = source.as_bytes();

    // Fast path: all whitespace
    if bytes.iter().all(|&b| is_whitespace(b)) {
        return Ok(String::new());
    }

    let formatter = TemplateFormatter::new(options);
    formatter.format(bytes)
}

/// High-performance template formatter
struct TemplateFormatter<'a> {
    options: &'a FormatOptions,
    indent: &'static [u8],
    newline: &'static [u8],
}

impl<'a> TemplateFormatter<'a> {
    #[inline]
    fn new(options: &'a FormatOptions) -> Self {
        Self {
            options,
            indent: options.indent_bytes(),
            newline: options.newline_bytes(),
        }
    }

    fn format(&self, source: &[u8]) -> Result<String, FormatError> {
        let len = source.len();
        let mut output = Vec::with_capacity(len + len / 4);
        let mut pos = 0;
        let mut depth: usize = 0;
        let mut line_buffer = Vec::with_capacity(256);

        while pos < len {
            // Skip whitespace at line start (except newlines)
            while pos < len && is_whitespace(source[pos]) && source[pos] != b'\n' {
                pos += 1;
            }

            if pos >= len {
                break;
            }

            // Handle newlines
            if source[pos] == b'\n' {
                pos += 1;
                continue;
            }

            // HTML comment <!-- ... -->
            if pos + 3 < len && &source[pos..pos + 4] == b"<!--" {
                self.flush_text_buffer(&mut output, &mut line_buffer, depth);
                let comment_start = pos;
                if let Some(end_offset) = find_bytes(&source[pos..], b"-->") {
                    let comment_end = pos + end_offset + 3;
                    self.write_indent(&mut output, depth);
                    output.extend_from_slice(&source[comment_start..comment_end]);
                    output.extend_from_slice(self.newline);
                    pos = comment_end;
                } else {
                    // Unclosed comment - write remainder
                    self.write_indent(&mut output, depth);
                    output.extend_from_slice(&source[comment_start..]);
                    output.extend_from_slice(self.newline);
                    pos = len;
                }
                continue;
            }

            // Tag start
            if source[pos] == b'<' {
                self.flush_text_buffer(&mut output, &mut line_buffer, depth);

                // Closing tag
                if pos + 1 < len && source[pos + 1] == b'/' {
                    if let Some((tag_name, end_pos)) = parse_closing_tag(source, pos) {
                        depth = depth.saturating_sub(1);
                        self.write_indent(&mut output, depth);
                        output.extend_from_slice(b"</");
                        output.extend_from_slice(tag_name.as_bytes());
                        output.push(b'>');
                        output.extend_from_slice(self.newline);
                        pos = end_pos;
                        continue;
                    }
                }

                // Opening tag
                if let Some((tag_name, attrs, is_self_closing, end_pos)) =
                    self.parse_opening_tag(source, pos)
                {
                    // Sort attributes if enabled
                    let mut sorted_attrs = attrs;
                    if self.options.sort_attributes {
                        sort_attributes(&mut sorted_attrs, self.options);
                    }

                    self.write_indent(&mut output, depth);
                    output.push(b'<');
                    output.extend_from_slice(tag_name.as_bytes());

                    if !sorted_attrs.is_empty() {
                        let use_multiline =
                            self.should_use_multiline_attrs(&tag_name, &sorted_attrs, depth);

                        if use_multiline {
                            let max_per_line = self
                                .options
                                .max_attributes_per_line
                                .unwrap_or(1) // default 1 when multiline
                                .max(1) as usize;

                            let mut line_count = 0;
                            for attr in &sorted_attrs {
                                if line_count == 0 {
                                    // Start a new attribute line
                                    output.extend_from_slice(self.newline);
                                    self.write_indent(&mut output, depth + 1);
                                } else {
                                    output.push(b' ');
                                }
                                output.extend_from_slice(render_attribute(attr).as_bytes());
                                line_count += 1;
                                if line_count >= max_per_line {
                                    line_count = 0;
                                }
                            }
                            if !self.options.bracket_same_line {
                                output.extend_from_slice(self.newline);
                                self.write_indent(&mut output, depth);
                            }
                        } else {
                            for attr in &sorted_attrs {
                                output.push(b' ');
                                output.extend_from_slice(render_attribute(attr).as_bytes());
                            }
                        }
                    }

                    if is_self_closing {
                        output.extend_from_slice(b" />");
                    } else {
                        output.push(b'>');
                        if !is_void_element_str(&tag_name) {
                            depth += 1;
                        }
                    }
                    output.extend_from_slice(self.newline);
                    pos = end_pos;
                    continue;
                }
            }

            // Accumulate text content until newline or tag
            let content_start = pos;
            while pos < len && source[pos] != b'\n' && source[pos] != b'<' {
                pos += 1;
            }

            if pos > content_start {
                // Trim trailing whitespace from content
                let mut content_end = pos;
                while content_end > content_start && is_whitespace(source[content_end - 1]) {
                    content_end -= 1;
                }

                if content_end > content_start {
                    if !line_buffer.is_empty() {
                        line_buffer.push(b' ');
                    }
                    line_buffer.extend_from_slice(&source[content_start..content_end]);
                }
            }

            // Handle newline
            if pos < len && source[pos] == b'\n' {
                self.flush_text_buffer(&mut output, &mut line_buffer, depth);
                pos += 1;
            }
        }

        // Flush remaining content
        self.flush_text_buffer(&mut output, &mut line_buffer, depth);

        // Remove trailing newline for consistency
        while output.last().is_some_and(|&b| b == b'\n' || b == b'\r') {
            output.pop();
        }

        // SAFETY: We only wrote valid UTF-8 bytes
        Ok(unsafe { String::from_utf8_unchecked(output) })
    }

    /// Flush accumulated text content with interpolation formatting
    #[inline]
    fn flush_text_buffer(&self, output: &mut Vec<u8>, buffer: &mut Vec<u8>, depth: usize) {
        if buffer.is_empty() {
            return;
        }
        let text = std::str::from_utf8(buffer).unwrap_or("");
        let formatted = format_interpolations(text, self.options);
        self.write_indented_line(output, formatted.as_bytes(), depth);
        buffer.clear();
    }

    #[inline]
    fn write_indent(&self, output: &mut Vec<u8>, depth: usize) {
        for _ in 0..depth {
            output.extend_from_slice(self.indent);
        }
    }

    #[inline]
    fn write_indented_line(&self, output: &mut Vec<u8>, content: &[u8], depth: usize) {
        self.write_indent(output, depth);
        output.extend_from_slice(content);
        output.extend_from_slice(self.newline);
    }

    /// Determine whether attributes should be rendered in multiline mode.
    fn should_use_multiline_attrs(
        &self,
        tag_name: &str,
        attrs: &[ParsedAttribute],
        depth: usize,
    ) -> bool {
        if attrs.len() <= 1 {
            return false;
        }

        // Explicit max_attributes_per_line takes priority
        if let Some(max) = self.options.max_attributes_per_line {
            return attrs.len() > max as usize;
        }

        // single_attribute_per_line
        if self.options.single_attribute_per_line {
            return true;
        }

        // Check if all attributes on one line would exceed print_width
        let indent_len = self.indent.len() * depth;
        let tag_len = 1 + tag_name.len(); // '<' + tag_name
        let attrs_len: usize = attrs
            .iter()
            .map(|a| 1 + render_attribute(a).len()) // ' ' + attr
            .sum();
        let closing_len = 1; // '>'
        let total = indent_len + tag_len + attrs_len + closing_len;

        total > self.options.print_width as usize
    }

    /// Parse an opening tag into structured attributes
    fn parse_opening_tag(
        &self,
        source: &[u8],
        start: usize,
    ) -> Option<(String, Vec<ParsedAttribute>, bool, usize)> {
        let len = source.len();
        let mut pos = start + 1; // Skip '<'

        // Parse tag name
        let tag_start = pos;
        while pos < len && is_tag_name_char(source[pos]) {
            pos += 1;
        }
        if pos == tag_start {
            return None;
        }

        let tag_name = std::str::from_utf8(&source[tag_start..pos])
            .unwrap_or("")
            .to_string();

        // Parse attributes
        let mut attrs = Vec::new();
        let mut is_self_closing = false;
        let mut attr_index: usize = 0;

        while pos < len && source[pos] != b'>' {
            // Skip whitespace
            while pos < len && is_whitespace(source[pos]) {
                pos += 1;
            }
            if pos >= len {
                break;
            }

            // Check for self-closing or end
            if source[pos] == b'/' {
                is_self_closing = true;
                pos += 1;
                continue;
            }
            if source[pos] == b'>' {
                break;
            }

            // Parse single attribute
            let (attr, new_pos) = self.parse_single_attribute(source, pos, attr_index);
            if let Some(attr) = attr {
                attrs.push(attr);
                attr_index += 1;
            }
            pos = new_pos;
        }

        // Skip '>'
        if pos < len && source[pos] == b'>' {
            pos += 1;
        }

        Some((tag_name, attrs, is_self_closing, pos))
    }

    /// Parse a single attribute: name, optional `="value"`
    fn parse_single_attribute(
        &self,
        source: &[u8],
        start: usize,
        index: usize,
    ) -> (Option<ParsedAttribute>, usize) {
        let len = source.len();
        let mut pos = start;

        // Parse attribute name (may include :, @, #, ., v-, etc.)
        let name_start = pos;
        while pos < len {
            let b = source[pos];
            if is_whitespace(b) || b == b'>' || b == b'/' || b == b'=' {
                break;
            }
            pos += 1;
        }

        if pos == name_start {
            // Skip unknown byte to avoid infinite loop
            return (None, pos + 1);
        }

        let raw_name = std::str::from_utf8(&source[name_start..pos])
            .unwrap_or("")
            .to_string();

        // Skip whitespace before '='
        let mut val_pos = pos;
        while val_pos < len && (source[val_pos] == b' ' || source[val_pos] == b'\t') {
            val_pos += 1;
        }

        // Check for '=' and value
        let value = if val_pos < len && source[val_pos] == b'=' {
            val_pos += 1; // skip '='

            // Skip whitespace after '='
            while val_pos < len && (source[val_pos] == b' ' || source[val_pos] == b'\t') {
                val_pos += 1;
            }

            if val_pos < len && (source[val_pos] == b'"' || source[val_pos] == b'\'') {
                // Quoted value
                let quote = source[val_pos];
                val_pos += 1;
                let value_start = val_pos;
                while val_pos < len && source[val_pos] != quote {
                    val_pos += 1;
                }
                let value = std::str::from_utf8(&source[value_start..val_pos])
                    .unwrap_or("")
                    .to_string();
                if val_pos < len {
                    val_pos += 1; // skip closing quote
                }
                pos = val_pos;
                Some(value)
            } else {
                // Unquoted value
                let value_start = val_pos;
                while val_pos < len
                    && !is_whitespace(source[val_pos])
                    && source[val_pos] != b'>'
                    && source[val_pos] != b'/'
                {
                    val_pos += 1;
                }
                let value = std::str::from_utf8(&source[value_start..val_pos])
                    .unwrap_or("")
                    .to_string();
                pos = val_pos;
                Some(value)
            }
        } else {
            // Boolean attribute (no value)
            None
        };

        // Normalize directives and determine priority
        let (name, value, priority) = normalize_attribute(&raw_name, value, self.options);

        (
            Some(ParsedAttribute {
                name,
                value,
                priority,
                original_index: index,
            }),
            pos,
        )
    }
}

// ---------------------------------------------------------------------------
// Directive normalization & attribute helpers
// ---------------------------------------------------------------------------

/// Normalize directive shorthands and assign sort priority
fn normalize_attribute(
    name: &str,
    value: Option<String>,
    options: &FormatOptions,
) -> (String, Option<String>, u8) {
    // Normalize directive shorthands (only if enabled)
    let normalized_name = if options.normalize_directive_shorthands {
        if let Some(rest) = name.strip_prefix("v-bind:") {
            format!(":{rest}")
        } else if let Some(rest) = name.strip_prefix("v-on:") {
            format!("@{rest}")
        } else if let Some(rest) = name.strip_prefix("v-slot:") {
            format!("#{rest}")
        } else {
            name.to_string()
        }
    } else {
        name.to_string()
    };

    // Format JS expressions in directive values
    let formatted_value = value.map(|v| {
        if should_format_expression(&normalized_name) {
            format_directive_value(&normalized_name, &v, options)
        } else {
            v
        }
    });

    let priority = if let Some(ref groups) = options.attribute_groups {
        custom_attribute_priority(&normalized_name, groups)
    } else {
        attribute_priority(&normalized_name)
    };

    (normalized_name, formatted_value, priority)
}

/// Determine if an attribute's value should be formatted as a JS expression
fn should_format_expression(name: &str) -> bool {
    name.starts_with(':')
        || name.starts_with('@')
        || name.starts_with("v-if")
        || name.starts_with("v-else-if")
        || name.starts_with("v-show")
        || name.starts_with("v-for")
        || name.starts_with("v-model")
        || name.starts_with("v-bind")
        || name.starts_with("v-on")
        || name == "v-html"
        || name == "v-text"
}

/// Format a directive value expression
fn format_directive_value(name: &str, value: &str, options: &FormatOptions) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return value.to_string();
    }

    // v-for has special syntax: "(item, index) in items"
    if name == "v-for" {
        return format_v_for_expression(trimmed);
    }

    // Try to format as JS expression via oxc_formatter
    script::format_js_expression(trimmed, options).unwrap_or_else(|| value.to_string())
}

/// Format `v-for` expression: normalize spacing in `(item, index) in items`
fn format_v_for_expression(expr: &str) -> String {
    // Split on " in " or " of " (respecting nested parens/brackets)
    let (iterator_part, keyword, collection_part) =
        if let Some(idx) = find_v_for_keyword(expr, " in ") {
            (&expr[..idx], " in ", &expr[idx + 4..])
        } else if let Some(idx) = find_v_for_keyword(expr, " of ") {
            (&expr[..idx], " of ", &expr[idx + 4..])
        } else {
            return expr.to_string();
        };

    let iter_trimmed = iterator_part.trim();
    let collection_trimmed = collection_part.trim();

    // Normalize parenthesized destructuring: "(item,index)" → "(item, index)"
    let normalized_iter = if iter_trimmed.starts_with('(') && iter_trimmed.ends_with(')') {
        let inner = &iter_trimmed[1..iter_trimmed.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
        format!("({})", parts.join(", "))
    } else {
        iter_trimmed.to_string()
    };

    format!("{normalized_iter}{keyword}{collection_trimmed}")
}

/// Find `keyword` in a v-for expression while respecting nested parens/brackets
fn find_v_for_keyword(expr: &str, keyword: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let kw_bytes = keyword.as_bytes();
    let mut depth = 0i32;

    for i in 0..bytes.len() {
        match bytes[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0
            && i + kw_bytes.len() <= bytes.len()
            && &bytes[i..i + kw_bytes.len()] == kw_bytes
        {
            return Some(i);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Interpolation formatting
// ---------------------------------------------------------------------------

/// Format interpolations in text content: `{{expr}}` → `{{ expr }}`
fn format_interpolations(text: &str, options: &FormatOptions) -> String {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len + 16);
    let mut pos = 0;

    while pos < len {
        if pos + 1 < len && bytes[pos] == b'{' && bytes[pos + 1] == b'{' {
            // Find closing }}
            let expr_start = pos + 2;
            let mut depth = 1;
            let mut expr_end = expr_start;

            while expr_end + 1 < len {
                if bytes[expr_end] == b'{' && bytes[expr_end + 1] == b'{' {
                    depth += 1;
                    expr_end += 2;
                } else if bytes[expr_end] == b'}' && bytes[expr_end + 1] == b'}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    expr_end += 2;
                } else {
                    expr_end += 1;
                }
            }

            if depth == 0 {
                let expr = &text[expr_start..expr_end];
                let formatted_expr = script::format_js_expression(expr, options)
                    .unwrap_or_else(|| expr.trim().to_string());
                result.push_str("{{ ");
                result.push_str(&formatted_expr);
                result.push_str(" }}");
                pos = expr_end + 2;
            } else {
                // Unclosed interpolation – keep as-is
                result.push('{');
                pos += 1;
            }
        } else {
            // Push one UTF-8 character
            if let Some(ch) = text[pos..].chars().next() {
                result.push(ch);
                pos += ch.len_utf8();
            } else {
                pos += 1;
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Attribute sorting
// ---------------------------------------------------------------------------

/// Sort attributes based on the configured options.
fn sort_attributes(attrs: &mut [ParsedAttribute], options: &FormatOptions) {
    match options.attribute_sort_order {
        AttributeSortOrder::Alphabetical => {
            attrs.sort_by(|a, b| {
                // Primary: priority group
                let cmp = a.priority.cmp(&b.priority);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
                // Secondary: alphabetical within same group
                let a_key = attr_sort_key(&a.name, options.merge_bind_and_non_bind_attrs);
                let b_key = attr_sort_key(&b.name, options.merge_bind_and_non_bind_attrs);
                let alpha_cmp = a_key.cmp(&b_key);
                if alpha_cmp != std::cmp::Ordering::Equal {
                    return alpha_cmp;
                }
                // Tertiary: original order for stability
                a.original_index.cmp(&b.original_index)
            });
        }
        AttributeSortOrder::AsWritten => {
            // Only sort by priority group, keep original order within groups
            attrs.sort_by(|a, b| {
                let cmp = a.priority.cmp(&b.priority);
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
                a.original_index.cmp(&b.original_index)
            });
        }
    }
}

/// Generate a sort key for alphabetical ordering within a group.
///
/// When `merge_bind` is false, non-bind attrs come before bind attrs,
/// then each sub-group is sorted alphabetically:
///   `class`, `id`, `:class`, `:id`
///
/// When `merge_bind` is true, bind prefix is stripped so `:class` and
/// `class` are sorted together:
///   `class`, `:class`, `id`, `:id`
fn attr_sort_key(name: &str, merge_bind: bool) -> (u8, String) {
    if merge_bind {
        // Strip bind prefix for comparison
        let base = name
            .strip_prefix(':')
            .or_else(|| name.strip_prefix("v-bind:"))
            .unwrap_or(name);
        (0, base.to_ascii_lowercase())
    } else {
        // Non-bind first (0), then bind (1)
        let is_bind = name.starts_with(':') || name.starts_with("v-bind:");
        let base = name
            .strip_prefix(':')
            .or_else(|| name.strip_prefix("v-bind:"))
            .unwrap_or(name);
        let group = if is_bind { 1 } else { 0 };
        (group, base.to_ascii_lowercase())
    }
}

/// Attribute sort priority based on the Vue.js style guide:
///
/// 0. `is`
/// 1. `v-for`
/// 2. `v-if` / `v-else-if` / `v-else`
/// 3. `v-show`
/// 4. `id`
/// 5. `ref`
/// 6. `key` / `:key`
/// 7. `v-model`
/// 8. props & attributes — both bound (`:class`) and static (`class`) share the
///    same priority so that related pairs like `class`/`:class` stay adjacent.
/// 9. events (`@xxx`)
/// 10. `v-slot` / `#xxx`
/// 11. `v-html` / `v-text`
fn attribute_priority(name: &str) -> u8 {
    if name == "is" || name == ":is" || name == "v-is" {
        return 0;
    }
    if name == "v-for" {
        return 1;
    }
    if name == "v-if" || name == "v-else-if" || name == "v-else" {
        return 2;
    }
    if name == "v-show" {
        return 3;
    }
    if name == "id" {
        return 4;
    }
    if name == "ref" {
        return 5;
    }
    if name == "key" || name == ":key" {
        return 6;
    }
    if name.starts_with("v-model") {
        return 7;
    }
    // Events
    if name.starts_with('@') || name.starts_with("v-on") {
        return 9;
    }
    // Slots
    if name.starts_with('#') || name.starts_with("v-slot") {
        return 10;
    }
    if name == "v-html" || name == "v-text" {
        return 11;
    }
    // Both bound props (:class, :style, :xxx) and regular attributes (class, style, xxx)
    // share the same priority so that related pairs stay adjacent.
    8
}

/// Determine attribute priority based on custom attribute groups.
///
/// Each group in `groups` is a list of patterns. Groups are matched in order (index = priority).
/// Patterns: exact name (`id`), prefix glob (`v-*`, `:*`, `@*`), or `*` catch-all.
/// Unmatched attributes get priority `groups.len()` (last).
fn custom_attribute_priority(name: &str, groups: &[Vec<String>]) -> u8 {
    for (i, group) in groups.iter().enumerate() {
        for pattern in group {
            if matches_attr_pattern(name, pattern) {
                return i as u8;
            }
        }
    }
    groups.len() as u8
}

/// Match an attribute name against a pattern.
///
/// - `*` matches everything
/// - `prefix*` matches names starting with `prefix`
/// - exact string matches the name exactly
fn matches_attr_pattern(name: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return name.starts_with(prefix);
    }
    name == pattern
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render an attribute back to its string representation
fn render_attribute(attr: &ParsedAttribute) -> String {
    match &attr.value {
        Some(value) => format!("{}=\"{}\"", attr.name, value),
        None => attr.name.clone(),
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Find a byte subsequence in a slice
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Parse a closing tag, returns `(tag_name, end_pos)`
fn parse_closing_tag(source: &[u8], start: usize) -> Option<(String, usize)> {
    let len = source.len();
    let mut pos = start + 2; // skip '</'

    let tag_start = pos;
    while pos < len && is_tag_name_char(source[pos]) {
        pos += 1;
    }
    if pos == tag_start {
        return None;
    }

    let tag_name = std::str::from_utf8(&source[tag_start..pos])
        .unwrap_or("")
        .to_string();

    // Skip whitespace and find '>'
    while pos < len && source[pos] != b'>' {
        pos += 1;
    }
    if pos < len && source[pos] == b'>' {
        pos += 1;
    }

    Some((tag_name, pos))
}

/// Check if a byte is a valid tag name character
#[inline(always)]
fn is_tag_name_char(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b':')
}

/// Check if a byte is whitespace
#[inline(always)]
fn is_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Check if an element is a void element (self-closing in HTML)
fn is_void_element_str(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_template() {
        let source = "<div>Hello</div>";
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains("<div>"));
        assert!(result.contains("</div>"));
    }

    #[test]
    fn test_format_nested_template() {
        let source = "<div><span>Hello</span></div>";
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains("<div>"));
        assert!(result.contains("  <span>"));
        assert!(result.contains("</div>"));
    }

    #[test]
    fn test_format_with_attributes() {
        let source = r#"<div class="container" id="main">Content</div>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains(r#"class="container""#));
        assert!(result.contains(r#"id="main""#));
    }

    #[test]
    fn test_format_self_closing() {
        let source = "<input type=\"text\" />";
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains("<input"));
        assert!(result.contains("/>"));
    }

    #[test]
    fn test_directive_shorthand_v_bind() {
        let source = r#"<div v-bind:class="active"></div>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains(":class="));
        assert!(!result.contains("v-bind:class"));
    }

    #[test]
    fn test_directive_shorthand_v_on() {
        let source = r#"<div v-on:click="handler"></div>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains("@click="));
        assert!(!result.contains("v-on:click"));
    }

    #[test]
    fn test_directive_shorthand_v_slot() {
        let source = r#"<template v-slot:default="props"></template>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains("#default="));
        assert!(!result.contains("v-slot:default"));
    }

    #[test]
    fn test_interpolation_spacing_normalized() {
        let options = FormatOptions::default();
        let result = format_interpolations("{{count}}", &options);
        assert!(result.contains("{{ "));
        assert!(result.contains(" }}"));
    }

    #[test]
    fn test_interpolation_already_spaced() {
        let options = FormatOptions::default();
        let result = format_interpolations("{{ count }}", &options);
        assert!(result.contains("{{ "));
        assert!(result.contains(" }}"));
    }

    #[test]
    fn test_interpolation_in_text() {
        let options = FormatOptions::default();
        let result = format_interpolations("Hello {{name}} world", &options);
        assert!(result.starts_with("Hello "));
        assert!(result.contains("{{ "));
        assert!(result.contains(" }}"));
        assert!(result.ends_with(" world"));
    }

    #[test]
    fn test_v_for_normalization() {
        let result = format_v_for_expression("(item,index) in items");
        assert_eq!(result, "(item, index) in items");
    }

    #[test]
    fn test_v_for_simple() {
        let result = format_v_for_expression("item in items");
        assert_eq!(result, "item in items");
    }

    #[test]
    fn test_attribute_sorting() {
        let source =
            r#"<div :class="cls" v-if="show" v-for="item in items" @click="handle"></div>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        let vfor_pos = result.find("v-for").unwrap();
        let vif_pos = result.find("v-if").unwrap();
        let class_pos = result.find(":class").unwrap();
        let click_pos = result.find("@click").unwrap();

        assert!(vfor_pos < vif_pos, "v-for should come before v-if");
        assert!(vif_pos < class_pos, "v-if should come before :class");
        assert!(class_pos < click_pos, ":class should come before @click");
    }

    #[test]
    fn test_multiline_attributes() {
        let source = r#"<div class="container" id="main" @click="handler">Content</div>"#;
        let mut options = FormatOptions::default();
        options.single_attribute_per_line = true;
        let result = format_template_content(source, &options).unwrap();

        // Each attribute should be on its own line
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() > 2, "Should have multiple lines for attributes");
    }

    #[test]
    fn test_void_elements() {
        assert!(is_void_element_str("br"));
        assert!(is_void_element_str("img"));
        assert!(is_void_element_str("input"));
        assert!(!is_void_element_str("div"));
        assert!(!is_void_element_str("span"));
    }

    #[test]
    fn test_is_tag_name_char() {
        assert!(is_tag_name_char(b'a'));
        assert!(is_tag_name_char(b'Z'));
        assert!(is_tag_name_char(b'0'));
        assert!(is_tag_name_char(b'-'));
        assert!(is_tag_name_char(b'_'));
        assert!(!is_tag_name_char(b' '));
        assert!(!is_tag_name_char(b'>'));
    }

    #[test]
    fn test_v_else_boolean_attribute() {
        let source = r#"<div v-if="show">A</div><div v-else>B</div>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains("v-else"));
    }

    #[test]
    fn test_html_comment() {
        let source = "<!-- This is a comment -->\n<div>Content</div>";
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        assert!(result.contains("<!-- This is a comment -->"));
        assert!(result.contains("<div>"));
    }

    #[test]
    fn test_attribute_priority_order() {
        assert!(attribute_priority("is") < attribute_priority("v-for"));
        assert!(attribute_priority("v-for") < attribute_priority("v-if"));
        assert!(attribute_priority("v-if") < attribute_priority("v-show"));
        assert!(attribute_priority("v-show") < attribute_priority("id"));
        assert!(attribute_priority("id") < attribute_priority("ref"));
        assert!(attribute_priority("ref") < attribute_priority(":key"));
        assert!(attribute_priority(":key") < attribute_priority("v-model"));
        assert!(attribute_priority("v-model") < attribute_priority(":class"));
        // :class and class share the same priority so they stay adjacent
        assert_eq!(attribute_priority(":class"), attribute_priority("class"));
        assert_eq!(attribute_priority(":style"), attribute_priority("style"));
        assert!(attribute_priority("class") < attribute_priority("@click"));
        assert!(attribute_priority("@click") < attribute_priority("#default"));
        assert!(attribute_priority("#default") < attribute_priority("v-html"));
    }

    // ---------------------------------------------------------------
    // Tests for new configuration options
    // ---------------------------------------------------------------

    #[test]
    fn test_sort_attributes_disabled() {
        // When sort_attributes is false, keep original order
        let source =
            r#"<div @click="handle" :class="cls" v-if="show" v-for="item in items"></div>"#;
        let mut options = FormatOptions::default();
        options.sort_attributes = false;
        let result = format_template_content(source, &options).unwrap();

        let click_pos = result.find("@click").unwrap();
        let class_pos = result.find(":class").unwrap();
        let vif_pos = result.find("v-if").unwrap();
        let vfor_pos = result.find("v-for").unwrap();

        // Original order preserved: @click, :class, v-if, v-for
        assert!(click_pos < class_pos);
        assert!(class_pos < vif_pos);
        assert!(vif_pos < vfor_pos);
    }

    #[test]
    fn test_alphabetical_sort_within_group() {
        // Props (same priority group 8) should be sorted alphabetically
        let source = r#"<div title="t" class="c" aria-label="a"></div>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        let aria_pos = result.find("aria-label").unwrap();
        let class_pos = result.find("class").unwrap();
        let title_pos = result.find("title").unwrap();

        assert!(
            aria_pos < class_pos,
            "aria-label should come before class alphabetically"
        );
        assert!(
            class_pos < title_pos,
            "class should come before title alphabetically"
        );
    }

    #[test]
    fn test_as_written_sort_within_group() {
        // When attribute_sort_order is AsWritten, keep original order within group
        let source = r#"<div title="t" class="c" aria-label="a"></div>"#;
        let mut options = FormatOptions::default();
        options.attribute_sort_order = AttributeSortOrder::AsWritten;
        let result = format_template_content(source, &options).unwrap();

        let title_pos = result.find("title").unwrap();
        let class_pos = result.find("class").unwrap();
        let aria_pos = result.find("aria-label").unwrap();

        // Original order within group preserved: title, class, aria-label
        assert!(title_pos < class_pos);
        assert!(class_pos < aria_pos);
    }

    #[test]
    fn test_merge_bind_and_non_bind_false() {
        // Default: non-bind attrs first, then bind attrs, each sorted alphabetically
        let source = r#"<div :class="cls" class="base" :style="s" style="color:red"></div>"#;
        let options = FormatOptions::default();
        let result = format_template_content(source, &options).unwrap();

        let class_pos = result.find("class=").unwrap();
        let style_pos = result.find("style=").unwrap();
        let bind_class_pos = result.find(":class=").unwrap();
        let bind_style_pos = result.find(":style=").unwrap();

        // Non-bind first: class, style, :class, :style
        assert!(
            class_pos < bind_class_pos,
            "class should come before :class"
        );
        assert!(
            style_pos < bind_style_pos,
            "style should come before :style"
        );
    }

    #[test]
    fn test_merge_bind_and_non_bind_true() {
        // When merge_bind_and_non_bind_attrs is true, bind and non-bind are merged
        let source = r#"<div :class="cls" class="base" :style="s" style="color:red"></div>"#;
        let mut options = FormatOptions::default();
        options.merge_bind_and_non_bind_attrs = true;
        let result = format_template_content(source, &options).unwrap();

        // With merging: class and :class sort together by base name "class",
        // style and :style sort together by base name "style"
        // Expected order: class, :class, style, :style (or :class, class, :style, style)
        // Since both have the same base, stable sort by original_index applies
        let class_pos = result.find("class=").unwrap();
        let bind_class_pos = result.find(":class=").unwrap();
        let style_pos = result.find("style=").unwrap();
        let bind_style_pos = result.find(":style=").unwrap();

        // "class" and ":class" should be adjacent, "style" and ":style" should be adjacent
        // All "class*" come before "style*" alphabetically
        assert!(
            class_pos.min(bind_class_pos) < style_pos.min(bind_style_pos),
            "class group should come before style group"
        );
    }

    #[test]
    fn test_max_attributes_per_line() {
        let source = r#"<div class="c" id="main" title="t" aria-label="a" role="button"></div>"#;
        let mut options = FormatOptions::default();
        options.max_attributes_per_line = Some(2);
        let result = format_template_content(source, &options).unwrap();

        // Should wrap: multiple lines with at most 2 attrs per line
        let lines: Vec<&str> = result.lines().collect();
        assert!(
            lines.len() >= 3,
            "Should have at least 3 lines with max 2 attrs per line for 5 attrs"
        );
    }

    #[test]
    fn test_max_attributes_per_line_no_wrap_if_within() {
        let source = r#"<div class="c" id="main"></div>"#;
        let mut options = FormatOptions::default();
        options.max_attributes_per_line = Some(3);
        let result = format_template_content(source, &options).unwrap();

        // 2 attrs <= 3 limit, no wrapping
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2, "Should not wrap (div + closing)");
    }

    #[test]
    fn test_custom_attribute_groups() {
        // Custom groups: [["id"], ["class", ":class"], ["@*"], ["*"]]
        let source = r#"<div @click="h" class="c" id="main" title="t"></div>"#;
        let mut options = FormatOptions::default();
        options.attribute_groups = Some(vec![
            vec!["id".to_string()],
            vec!["class".to_string(), ":class".to_string()],
            vec!["@*".to_string()],
            vec!["*".to_string()],
        ]);
        let result = format_template_content(source, &options).unwrap();

        let id_pos = result.find("id=").unwrap();
        let class_pos = result.find("class=").unwrap();
        let click_pos = result.find("@click=").unwrap();
        let title_pos = result.find("title=").unwrap();

        assert!(id_pos < class_pos, "id should come first (group 0)");
        assert!(
            class_pos < click_pos,
            "class (group 1) before @click (group 2)"
        );
        assert!(
            click_pos < title_pos,
            "@click (group 2) before title (group 3)"
        );
    }

    #[test]
    fn test_normalize_directive_shorthands_disabled() {
        let source = r#"<div v-bind:class="active" v-on:click="handler"></div>"#;
        let mut options = FormatOptions::default();
        options.normalize_directive_shorthands = false;
        let result = format_template_content(source, &options).unwrap();

        // Shorthands should NOT be applied
        assert!(
            result.contains("v-bind:class"),
            "v-bind:class should be preserved"
        );
        assert!(
            result.contains("v-on:click"),
            "v-on:click should be preserved"
        );
    }

    #[test]
    fn test_custom_attribute_priority() {
        let groups = vec![
            vec!["v-for".to_string()],
            vec!["v-if".to_string(), "v-else".to_string()],
            vec![":*".to_string()],
            vec!["@*".to_string()],
        ];

        assert_eq!(custom_attribute_priority("v-for", &groups), 0);
        assert_eq!(custom_attribute_priority("v-if", &groups), 1);
        assert_eq!(custom_attribute_priority("v-else", &groups), 1);
        assert_eq!(custom_attribute_priority(":class", &groups), 2);
        assert_eq!(custom_attribute_priority(":style", &groups), 2);
        assert_eq!(custom_attribute_priority("@click", &groups), 3);
        // Unmatched gets groups.len()
        assert_eq!(custom_attribute_priority("id", &groups), 4);
    }

    #[test]
    fn test_matches_attr_pattern() {
        assert!(matches_attr_pattern("class", "class"));
        assert!(!matches_attr_pattern("class", "id"));
        assert!(matches_attr_pattern(":class", ":*"));
        assert!(matches_attr_pattern(":style", ":*"));
        assert!(matches_attr_pattern("@click", "@*"));
        assert!(matches_attr_pattern("v-for", "v-*"));
        assert!(matches_attr_pattern("anything", "*"));
    }

    #[test]
    fn test_print_width_triggers_multiline() {
        // Very narrow print_width should trigger multiline
        let source = r#"<div class="container" id="main" title="tooltip"></div>"#;
        let mut options = FormatOptions::default();
        options.print_width = 30;
        let result = format_template_content(source, &options).unwrap();

        let lines: Vec<&str> = result.lines().collect();
        assert!(
            lines.len() > 2,
            "Narrow print_width should trigger multiline attributes"
        );
    }
}
