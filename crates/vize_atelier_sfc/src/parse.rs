//! SFC parsing implementation.
//!
//! Zero-copy design with byte-level operations for maximum performance.
//! Uses Cow<str> to avoid string allocations during parsing.

use crate::types::*;
use memchr::memchr;
use std::borrow::Cow;
use vize_carton::FxHashMap;

// Static closing tags for fast comparison (avoid format!)
const CLOSING_SCRIPT: &[u8] = b"</script>";
const CLOSING_STYLE: &[u8] = b"</style>";

// Tag name bytes for fast comparison
const TAG_TEMPLATE: &[u8] = b"template";
const TAG_SCRIPT: &[u8] = b"script";
const TAG_STYLE: &[u8] = b"style";

/// Parse a Vue SFC into a descriptor with zero-copy strings
pub fn parse_sfc<'a>(
    source: &'a str,
    options: SfcParseOptions,
) -> Result<SfcDescriptor<'a>, SfcError> {
    let mut descriptor = SfcDescriptor {
        filename: Cow::Owned(options.filename),
        source: Cow::Borrowed(source),
        ..Default::default()
    };

    let bytes = source.as_bytes();
    let len = bytes.len();

    let mut pos = 0;
    let mut line = 1;
    let mut column = 1;

    while pos < len {
        // Skip whitespace using byte comparison
        while pos < len {
            let c = bytes[pos];
            if c == b' ' || c == b'\t' || c == b'\r' {
                pos += 1;
                column += 1;
            } else if c == b'\n' {
                pos += 1;
                line += 1;
                column = 1;
            } else {
                break;
            }
        }

        if pos >= len {
            break;
        }

        // Use memchr to find next '<' quickly
        if bytes[pos] != b'<' {
            if let Some(next_lt) = memchr(b'<', &bytes[pos..]) {
                // Update line/column for skipped content
                for &b in &bytes[pos..pos + next_lt] {
                    if b == b'\n' {
                        line += 1;
                        column = 1;
                    } else {
                        column += 1;
                    }
                }
                pos += next_lt;
            } else {
                break;
            }
        }

        if pos >= len {
            break;
        }

        // Parse block starting at '<'
        if let Some(block_result) = parse_block_fast(bytes, source, pos, line) {
            let (tag_name, attrs, content, content_start, content_end, end_pos, end_line, end_col) =
                block_result;

            let loc = BlockLocation {
                start: content_start,
                end: content_end,
                tag_start: pos,
                tag_end: end_pos,
                start_line: line,
                start_column: column,
                end_line,
                end_column: end_col,
            };

            // Match tag name using byte comparison
            if tag_name_eq(tag_name, TAG_TEMPLATE) {
                if descriptor.template.is_some() {
                    return Err(SfcError {
                        message: "SFC can only contain one <template> block".into(),
                        code: Some("DUPLICATE_TEMPLATE".into()),
                        loc: Some(loc.clone()),
                    });
                }
                descriptor.template = Some(SfcTemplateBlock {
                    content,
                    loc,
                    lang: attrs.get("lang").cloned(),
                    src: attrs.get("src").cloned(),
                    attrs,
                });
            } else if tag_name_eq(tag_name, TAG_SCRIPT) {
                let is_setup = attrs.contains_key("setup");
                let script_block = SfcScriptBlock {
                    content,
                    loc,
                    lang: attrs.get("lang").cloned(),
                    src: attrs.get("src").cloned(),
                    setup: is_setup,
                    attrs,
                    bindings: None,
                };

                if is_setup {
                    if descriptor.script_setup.is_some() {
                        return Err(SfcError {
                            message: "SFC can only contain one <script setup> block".into(),
                            code: Some("DUPLICATE_SCRIPT_SETUP".into()),
                            loc: Some(script_block.loc),
                        });
                    }
                    descriptor.script_setup = Some(script_block);
                } else {
                    if descriptor.script.is_some() {
                        return Err(SfcError {
                            message: "SFC can only contain one <script> block".into(),
                            code: Some("DUPLICATE_SCRIPT".into()),
                            loc: Some(script_block.loc),
                        });
                    }
                    descriptor.script = Some(script_block);
                }
            } else if tag_name_eq(tag_name, TAG_STYLE) {
                let scoped = attrs.contains_key("scoped");
                let module = if attrs.contains_key("module") {
                    Some(
                        attrs
                            .get("module")
                            .filter(|v| !v.is_empty())
                            .cloned()
                            .unwrap_or_else(|| Cow::Borrowed("$style")),
                    )
                } else {
                    None
                };

                descriptor.styles.push(SfcStyleBlock {
                    content,
                    loc,
                    lang: attrs.get("lang").cloned(),
                    src: attrs.get("src").cloned(),
                    scoped,
                    module,
                    attrs,
                });
            } else {
                // Custom block - use borrowed tag name
                let tag_str = unsafe { std::str::from_utf8_unchecked(tag_name) };
                descriptor.custom_blocks.push(SfcCustomBlock {
                    block_type: Cow::Borrowed(tag_str),
                    content,
                    loc,
                    attrs,
                });
            }

            pos = end_pos;
            line = end_line;
            column = end_col;
        } else {
            pos += 1;
            column += 1;
        }
    }

    Ok(descriptor)
}

/// Fast tag name comparison using byte slices
#[inline(always)]
fn tag_name_eq(name: &[u8], expected: &[u8]) -> bool {
    name.len() == expected.len() && name.eq_ignore_ascii_case(expected)
}

/// Parse a single block from the source using byte operations
/// Returns borrowed strings using Cow for zero-copy
fn parse_block_fast<'a>(
    bytes: &[u8],
    source: &'a str,
    start: usize,
    start_line: usize,
) -> Option<(
    &'a [u8],                              // tag name as bytes
    FxHashMap<Cow<'a, str>, Cow<'a, str>>, // attrs with borrowed strings
    Cow<'a, str>,                          // content as borrowed string
    usize,                                 // content start
    usize,                                 // content end
    usize,                                 // end position
    usize,                                 // end line
    usize,                                 // end column
)> {
    let len = bytes.len();

    // Skip '<'
    let mut pos = start + 1;
    if pos >= len {
        return None;
    }

    // Parse tag name - find end of tag name
    let tag_start = pos;
    while pos < len && is_tag_name_char_fast(bytes[pos]) {
        pos += 1;
    }

    if pos == tag_start {
        return None;
    }

    let tag_name = &source.as_bytes()[tag_start..pos];

    // Parse attributes with zero-copy
    let mut attrs: FxHashMap<Cow<'a, str>, Cow<'a, str>> = FxHashMap::default();

    while pos < len && bytes[pos] != b'>' {
        // Skip whitespace
        while pos < len && is_whitespace_fast(bytes[pos]) {
            pos += 1;
        }

        if pos >= len || bytes[pos] == b'>' || bytes[pos] == b'/' {
            break;
        }

        // Parse attribute name
        let attr_start = pos;
        while pos < len {
            let c = bytes[pos];
            if c == b'='
                || c == b' '
                || c == b'>'
                || c == b'/'
                || c == b'\t'
                || c == b'\n'
                || c == b'\r'
            {
                break;
            }
            pos += 1;
        }

        if pos == attr_start {
            pos += 1;
            continue;
        }

        // Zero-copy: borrow from source
        let attr_name: Cow<'a, str> = Cow::Borrowed(&source[attr_start..pos]);

        // Skip whitespace
        while pos < len && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            pos += 1;
        }

        let attr_value: Cow<'a, str> = if pos < len && bytes[pos] == b'=' {
            pos += 1;

            // Skip whitespace
            while pos < len && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }

            if pos < len && (bytes[pos] == b'"' || bytes[pos] == b'\'') {
                let quote_char = bytes[pos];
                pos += 1;
                let value_start = pos;

                // Use memchr for fast quote finding
                if let Some(quote_pos) = memchr(quote_char, &bytes[pos..]) {
                    pos += quote_pos;
                    let value = Cow::Borrowed(&source[value_start..pos]);
                    pos += 1; // Skip closing quote
                    value
                } else {
                    // No closing quote found
                    while pos < len && bytes[pos] != quote_char {
                        pos += 1;
                    }
                    let value = Cow::Borrowed(&source[value_start..pos]);
                    if pos < len {
                        pos += 1;
                    }
                    value
                }
            } else {
                // Unquoted value
                let value_start = pos;
                while pos < len {
                    let c = bytes[pos];
                    if c == b' ' || c == b'>' || c == b'/' || c == b'\t' || c == b'\n' {
                        break;
                    }
                    pos += 1;
                }
                Cow::Borrowed(&source[value_start..pos])
            }
        } else {
            // Boolean attribute
            Cow::Borrowed("")
        };

        if !attr_name.is_empty() {
            attrs.insert(attr_name, attr_value);
        }
    }

    // Handle self-closing tag
    let is_self_closing = pos > 0 && pos < len && bytes[pos - 1] == b'/';

    if is_self_closing {
        if pos < len && bytes[pos] == b'>' {
            pos += 1;
        }
        return Some((
            tag_name,
            attrs,
            Cow::Borrowed(""),
            pos,
            pos,
            pos,
            start_line,
            pos - start,
        ));
    }

    // Skip '>'
    if pos < len && bytes[pos] == b'>' {
        pos += 1;
    } else {
        return None;
    }

    let content_start = pos;

    // Find closing tag based on tag type
    let mut line = start_line;
    let mut last_newline = start;

    // Handle known tags with static closing tags
    if tag_name.eq_ignore_ascii_case(TAG_TEMPLATE) {
        // Template block: handle nested template tags
        let mut depth = 1;

        // Check for closing template tag, handling whitespace before the closing '>'
        // This handles cases like:
        //   </template>       - normal
        //   </template\n   >  - closing '>' on next line
        fn is_closing_template_tag(bytes: &[u8], pos: usize, len: usize) -> Option<usize> {
            // Check if we have "</template" (without the final ">")
            const CLOSING_TAG_PREFIX: &[u8] = b"</template";
            if pos + CLOSING_TAG_PREFIX.len() > len {
                return None;
            }
            if !bytes[pos..pos + CLOSING_TAG_PREFIX.len()].eq_ignore_ascii_case(CLOSING_TAG_PREFIX)
            {
                return None;
            }
            // Find the closing '>' allowing whitespace
            let mut check_pos = pos + CLOSING_TAG_PREFIX.len();
            while check_pos < len {
                match bytes[check_pos] {
                    b'>' => return Some(check_pos + 1), // Return position after '>'
                    b' ' | b'\t' | b'\n' | b'\r' => check_pos += 1,
                    _ => return None, // Invalid character in closing tag
                }
            }
            None
        }

        while pos < len {
            if bytes[pos] == b'\n' {
                line += 1;
                last_newline = pos;
            }

            if bytes[pos] == b'<' {
                // Check for closing tag using byte comparison
                if let Some(end_tag_pos) = is_closing_template_tag(bytes, pos, len) {
                    depth -= 1;
                    if depth == 0 {
                        let content_end = pos;
                        let end_pos = end_tag_pos;
                        let col = pos - last_newline + (end_pos - pos);
                        let content = Cow::Borrowed(&source[content_start..content_end]);
                        return Some((
                            tag_name,
                            attrs,
                            content,
                            content_start,
                            content_end,
                            end_pos,
                            line,
                            col,
                        ));
                    }
                    pos = end_tag_pos;
                    continue;
                }

                // Check for nested opening tag
                if starts_with_bytes(&bytes[pos + 1..], TAG_TEMPLATE) {
                    let tag_check_pos = pos + 1 + TAG_TEMPLATE.len();
                    if tag_check_pos < len {
                        let next_char = bytes[tag_check_pos];
                        if next_char == b' '
                            || next_char == b'>'
                            || next_char == b'\n'
                            || next_char == b'\t'
                            || next_char == b'\r'
                        {
                            // Check if self-closing
                            let mut check_pos = tag_check_pos;
                            let mut is_self_closing_nested = false;
                            while check_pos < len && bytes[check_pos] != b'>' {
                                if bytes[check_pos] == b'/'
                                    && check_pos + 1 < len
                                    && bytes[check_pos + 1] == b'>'
                                {
                                    is_self_closing_nested = true;
                                    break;
                                }
                                check_pos += 1;
                            }
                            if !is_self_closing_nested {
                                depth += 1;
                            }
                        }
                    }
                }
            }

            pos += 1;
        }
        return None;
    }

    // Script/style blocks: use static closing tags
    let closing_tag = if tag_name.eq_ignore_ascii_case(TAG_SCRIPT) {
        CLOSING_SCRIPT
    } else if tag_name.eq_ignore_ascii_case(TAG_STYLE) {
        CLOSING_STYLE
    } else {
        // Custom block: need to find closing tag dynamically
        return find_custom_block_end(
            bytes,
            source,
            tag_name,
            pos,
            content_start,
            start_line,
            attrs,
        );
    };

    // For script blocks, we need to be aware of string literals to avoid
    // matching closing tags inside strings like: const x = `</script>`
    let is_script = tag_name.eq_ignore_ascii_case(TAG_SCRIPT);

    // Track the previous non-whitespace character to determine string context
    let mut prev_significant_char: u8 = b'\n'; // Start as if at beginning of line

    while pos < len {
        let b = bytes[pos];

        if b == b'\n' {
            line += 1;
            last_newline = pos;
            prev_significant_char = b'\n';
            pos += 1;
            continue;
        }

        // Skip whitespace but don't update prev_significant_char
        if b == b' ' || b == b'\t' || b == b'\r' {
            pos += 1;
            continue;
        }

        // For script blocks, skip over comments and string literals
        if is_script {
            // Check for single-line comment
            if b == b'/' && pos + 1 < len && bytes[pos + 1] == b'/' {
                // Skip to end of line
                pos += 2;
                while pos < len && bytes[pos] != b'\n' {
                    pos += 1;
                }
                continue;
            }

            // Check for multi-line comment
            if b == b'/' && pos + 1 < len && bytes[pos + 1] == b'*' {
                pos += 2;
                while pos + 1 < len {
                    if bytes[pos] == b'\n' {
                        line += 1;
                        last_newline = pos;
                    }
                    if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
                        pos += 2;
                        break;
                    }
                    pos += 1;
                }
                continue;
            }

            // Check for string literals (', ", `)
            // Only treat as string if in a context where strings are expected
            // (after =, (, [, ,, :, {, or at start of expression)
            // This avoids treating quotes inside regex literals as strings
            if b == b'\'' || b == b'"' || b == b'`' {
                let is_string_context = matches!(
                    prev_significant_char,
                    b'=' | b'('
                        | b'['
                        | b','
                        | b':'
                        | b'{'
                        | b';'
                        | b'\n'
                        | b'?'
                        | b'&'
                        | b'|'
                        | b'+'
                        | b'-'
                        | b'*'
                        | b'!'
                        | b'>'
                        | b'<'
                        | b'%'
                        | b'^'
                );

                if is_string_context {
                    let quote = b;
                    pos += 1;

                    while pos < len {
                        let c = bytes[pos];

                        if c == b'\n' {
                            line += 1;
                            last_newline = pos;
                        }

                        // Handle escape sequences
                        if c == b'\\' && pos + 1 < len {
                            pos += 2; // Skip escaped character
                            continue;
                        }

                        // Handle template literal expressions ${...}
                        if quote == b'`' && c == b'$' && pos + 1 < len && bytes[pos + 1] == b'{' {
                            pos += 2;
                            let mut brace_depth = 1;
                            while pos < len && brace_depth > 0 {
                                let inner = bytes[pos];
                                if inner == b'\n' {
                                    line += 1;
                                    last_newline = pos;
                                }
                                if inner == b'{' {
                                    brace_depth += 1;
                                } else if inner == b'}' {
                                    brace_depth -= 1;
                                } else if inner == b'\\' && pos + 1 < len {
                                    pos += 1; // Skip escape in template expression
                                }
                                pos += 1;
                            }
                            continue;
                        }

                        // End of string
                        if c == quote {
                            pos += 1;
                            break;
                        }

                        // For non-template strings, newline ends the string (syntax error, but handle gracefully)
                        if quote != b'`' && c == b'\n' {
                            break;
                        }

                        pos += 1;
                    }
                    prev_significant_char = quote; // String ended with quote
                    continue;
                }
            }
        }

        // Check for closing tag
        if b == b'<' && starts_with_bytes(&bytes[pos..], closing_tag) {
            let content_end = pos;
            let end_pos = pos + closing_tag.len();
            let col = pos - last_newline + closing_tag.len();
            let content = Cow::Borrowed(&source[content_start..content_end]);
            return Some((
                tag_name,
                attrs,
                content,
                content_start,
                content_end,
                end_pos,
                line,
                col,
            ));
        }

        prev_significant_char = b;
        pos += 1;
    }

    None
}

/// Find the end of a custom block (non-template/script/style)
fn find_custom_block_end<'a>(
    bytes: &[u8],
    source: &'a str,
    tag_name: &'a [u8],
    mut pos: usize,
    content_start: usize,
    start_line: usize,
    attrs: FxHashMap<Cow<'a, str>, Cow<'a, str>>,
) -> Option<(
    &'a [u8],
    FxHashMap<Cow<'a, str>, Cow<'a, str>>,
    Cow<'a, str>,
    usize,
    usize,
    usize,
    usize,
    usize,
)> {
    let len = bytes.len();
    let mut line = start_line;
    let mut last_newline = content_start;

    while pos < len {
        if let Some(lt_offset) = memchr(b'<', &bytes[pos..]) {
            // Count newlines
            for &b in &bytes[pos..pos + lt_offset] {
                if b == b'\n' {
                    line += 1;
                    last_newline = pos + lt_offset;
                }
            }
            pos += lt_offset;

            // Check for </
            if pos + 2 < len && bytes[pos] == b'<' && bytes[pos + 1] == b'/' {
                let close_tag_start = pos + 2;
                // Check if tag name matches
                if close_tag_start + tag_name.len() <= len
                    && bytes[close_tag_start..close_tag_start + tag_name.len()]
                        .eq_ignore_ascii_case(tag_name)
                {
                    // Check for closing >
                    let after_name = close_tag_start + tag_name.len();
                    if after_name < len && bytes[after_name] == b'>' {
                        let content_end = pos;
                        let end_pos = after_name + 1;
                        let col = pos - last_newline + (end_pos - pos);
                        let content = Cow::Borrowed(&source[content_start..content_end]);
                        return Some((
                            tag_name,
                            attrs,
                            content,
                            content_start,
                            content_end,
                            end_pos,
                            line,
                            col,
                        ));
                    }
                }
            }
            pos += 1;
        } else {
            break;
        }
    }

    None
}

/// Fast byte slice prefix check
#[inline(always)]
fn starts_with_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.len() >= needle.len() && haystack[..needle.len()].eq_ignore_ascii_case(needle)
}

/// Fast tag name character check
#[inline(always)]
fn is_tag_name_char_fast(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_')
}

/// Fast whitespace check
#[inline(always)]
fn is_whitespace_fast(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_sfc() {
        let result = parse_sfc("", Default::default()).unwrap();
        assert!(result.template.is_none());
        assert!(result.script.is_none());
        assert!(result.styles.is_empty());
    }

    #[test]
    fn test_parse_template_only() {
        let source = "<template><div>Hello</div></template>";
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert_eq!(template.content, "<div>Hello</div>");
    }

    #[test]
    fn test_parse_with_lang_attr() {
        let source = r#"<script lang="ts">const x: number = 1</script>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script.is_some());
        let script = result.script.unwrap();
        assert_eq!(script.lang.as_deref(), Some("ts"));
    }

    #[test]
    fn test_parse_multiple_styles() {
        let source = r#"
<style>.a {}</style>
<style scoped>.b {}</style>
<style lang="scss">.c {}</style>
"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert_eq!(result.styles.len(), 3);
        assert!(!result.styles[0].scoped);
        assert!(result.styles[1].scoped);
        assert_eq!(result.styles[2].lang.as_deref(), Some("scss"));
    }

    #[test]
    fn test_parse_custom_block() {
        let source = r#"
<template><div></div></template>
<i18n>{"en": {"hello": "Hello"}}</i18n>
"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert_eq!(result.custom_blocks.len(), 1);
        assert_eq!(result.custom_blocks[0].block_type, "i18n");
    }

    #[test]
    fn test_parse_script_setup() {
        let source = r#"
<script setup lang="ts">
import { ref } from 'vue'
const count = ref(0)
</script>
"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script.setup);
        assert_eq!(script.lang.as_deref(), Some("ts"));
    }

    #[test]
    fn test_zero_copy_content() {
        let source = "<template><div>Hello World</div></template>";
        let result = parse_sfc(source, Default::default()).unwrap();

        // Verify that content is borrowed (Cow::Borrowed)
        let template = result.template.unwrap();
        match &template.content {
            Cow::Borrowed(s) => {
                // The string should be a slice of the original source
                let ptr = s.as_ptr();
                let source_ptr = source.as_ptr();
                assert!(ptr >= source_ptr && ptr < unsafe { source_ptr.add(source.len()) });
            }
            Cow::Owned(_) => panic!("Expected Cow::Borrowed, got Cow::Owned"),
        }
    }

    #[test]
    fn test_closing_template_tag_with_whitespace() {
        // Test that closing </template> tag with whitespace before '>' is handled correctly
        let source = r#"<script setup>
const x = 1
</script>

<template
  ><div>Hello</div></template
>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert_eq!(template.content, "<div>Hello</div>");
    }

    #[test]
    fn test_closing_template_tag_with_newline() {
        // Test that closing </template> tag with newline before '>' is handled correctly
        // This pattern appears in some Vue files with specific formatting
        let source = r#"<template>
  <div>Content</div>
</template
>

<style>
.foo {}
</style>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert!(template.content.contains("<div>Content</div>"));

        assert_eq!(result.styles.len(), 1);
    }

    #[test]
    fn test_nested_template_in_string_literal() {
        // Test that embedded <template> tags inside string literals don't confuse the parser
        let source = r#"<script setup lang="ts">
const code = `<template>
  <div>Nested</div>
</template>`
</script>

<template>
  <div>{{ code }}</div>
</template>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        assert!(result.template.is_some());

        let template = result.template.unwrap();
        // The main template should be the one at depth 0, not the one in the string
        assert!(template.content.contains("{{ code }}"));
    }

    #[test]
    fn test_template_with_v_slot_syntax() {
        // Test that <template v-slot> and <template #name> are handled as nested, not root
        let source = r#"<template>
  <MyComponent>
    <template #header>Header</template>
    <template v-slot:footer>Footer</template>
  </MyComponent>
</template>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.template.is_some());
        let template = result.template.unwrap();
        // Should contain the nested templates as content
        assert!(template.content.contains("<template #header>"));
        assert!(template.content.contains("<template v-slot:footer>"));
    }

    #[test]
    fn test_multiline_closing_tag_complex() {
        // Test complex case with multiple blocks and multiline closing tags
        let source = r#"<script setup>
const x = `</template>`  // embedded in string
</script>

<template
><div class="container">
    <template v-if="show">
      Content
    </template
    ><template v-else>
      Other
    </template>
  </div></template
>

<style scoped>
.container {}
</style>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        assert!(result.template.is_some());
        assert_eq!(result.styles.len(), 1);
        assert!(result.styles[0].scoped);

        let template = result.template.unwrap();
        assert!(template.content.contains("<div class=\"container\">"));
        assert!(template.content.contains("<template v-if=\"show\">"));
        assert!(template.content.contains("<template v-else>"));
    }

    #[test]
    fn test_script_with_embedded_closing_tag_in_template_literal() {
        // Test that </script> inside a template literal doesn't end the script block
        let source = r#"<script setup lang="ts">
const code = `<script setup>
console.log('hello')
</script>`
const x = 1
</script>

<template>
  <div>{{ code }}</div>
</template>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        // The script content should include everything up to the real </script>
        assert!(script.content.contains("const code = `<script setup>"));
        assert!(script.content.contains("</script>`"));
        assert!(script.content.contains("const x = 1"));
    }

    #[test]
    fn test_script_with_embedded_closing_tag_in_single_quote() {
        // Test that </script> inside a single-quoted string doesn't end the script block
        let source = r#"<script setup>
const tag = '</script>'
const y = 2
</script>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script.content.contains("const tag = '</script>'"));
        assert!(script.content.contains("const y = 2"));
    }

    #[test]
    fn test_script_with_embedded_closing_tag_in_double_quote() {
        // Test that </script> inside a double-quoted string doesn't end the script block
        let source = r#"<script setup>
const tag = "</script>"
const z = 3
</script>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script.content.contains(r#"const tag = "</script>""#));
        assert!(script.content.contains("const z = 3"));
    }

    #[test]
    fn test_script_with_embedded_closing_tag_in_comment() {
        // Test that </script> inside comments doesn't end the script block
        let source = r#"<script setup>
// This is a comment: </script>
const a = 1
/* Multi-line comment
   </script>
*/
const b = 2
</script>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script.content.contains("// This is a comment: </script>"));
        assert!(script.content.contains("const a = 1"));
        assert!(script.content.contains("</script>"));
        assert!(script.content.contains("const b = 2"));
    }

    #[test]
    fn test_script_with_template_literal_expression() {
        // Test that template literal with ${} expressions is handled correctly
        let source = r#"<script setup>
const name = 'world'
const code = `Hello ${name}! </script> ${1 + 2}`
const c = 3
</script>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script
            .content
            .contains("const code = `Hello ${name}! </script> ${1 + 2}`"));
        assert!(script.content.contains("const c = 3"));
    }

    #[test]
    fn test_script_with_escaped_quotes() {
        // Test that escaped quotes in strings are handled correctly
        let source = r#"<script setup>
const str1 = "He said \"</script>\""
const str2 = 'It\'s </script> here'
const str3 = `Template \` </script> \``
const d = 4
</script>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script.content.contains("const d = 4"));
    }

    #[test]
    fn test_script_with_regex_containing_quotes() {
        // Test that regex literals containing quotes don't confuse the parser
        // This is important for Monaco editor tokenizer patterns like: [/`[^`]*`/, "string"]
        let source = r#"<script setup>
const tokenizer = {
  root: [
    [/<script[^>]*>/, "tag"],
    [/"[^"]*"/, "string"],
    [/'[^']*'/, "string"],
    [/`[^`]*`/, "string"],
  ]
}
const e = 5
</script>

<template>
  <div>Test</div>
</template>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script.content.contains("const tokenizer"));
        assert!(script.content.contains(r#"[/`[^`]*`/, "string"]"#));
        assert!(script.content.contains("const e = 5"));

        assert!(result.template.is_some());
        let template = result.template.unwrap();
        assert!(template.content.contains("<div>Test</div>"));
    }

    #[test]
    fn test_script_with_division_operator() {
        // Test that division operator doesn't interfere with string detection
        let source = r#"<script setup>
const x = 10 / 2
const y = "test"
const z = x / y
</script>"#;
        let result = parse_sfc(source, Default::default()).unwrap();

        assert!(result.script_setup.is_some());
        let script = result.script_setup.unwrap();
        assert!(script.content.contains("const x = 10 / 2"));
        assert!(script.content.contains(r#"const y = "test""#));
    }
}
