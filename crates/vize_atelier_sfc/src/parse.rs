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

    // Script/style blocks: use static closing tags with memchr
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

    // Fast path for script/style using memchr
    while pos < len {
        if let Some(lt_offset) = memchr(b'<', &bytes[pos..]) {
            // Count newlines in skipped content
            for &b in &bytes[pos..pos + lt_offset] {
                if b == b'\n' {
                    line += 1;
                    last_newline = pos + lt_offset;
                }
            }
            pos += lt_offset;

            // Check for closing tag
            if starts_with_bytes(&bytes[pos..], closing_tag) {
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
            pos += 1;
        } else {
            break;
        }
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
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_croquis_playground_parse() {
        let source =
            std::fs::read_to_string("../../playground/src/components/CroquisPlayground.vue")
                .unwrap();
        println!("Source length: {}", source.len());

        let result = parse_sfc(&source, SfcParseOptions::default());
        match result {
            Ok(desc) => {
                println!("Parse successful!");
                if let Some(ref script_setup) = desc.script_setup {
                    println!(
                        "Script setup: tag offset {}..{}",
                        script_setup.loc.tag_start, script_setup.loc.tag_end
                    );
                    println!(
                        "Script setup content offset {}..{}",
                        script_setup.loc.start, script_setup.loc.end
                    );
                    println!(
                        "Script setup first 50 chars: {:?}",
                        script_setup.content.chars().take(50).collect::<String>()
                    );
                }
                if let Some(ref template) = desc.template {
                    println!(
                        "Template: tag offset {}..{}",
                        template.loc.tag_start, template.loc.tag_end
                    );
                    println!(
                        "Template content offset {}..{}",
                        template.loc.start, template.loc.end
                    );
                    println!(
                        "Template content first 100 chars: {:?}",
                        template.content.chars().take(100).collect::<String>()
                    );
                }
            }
            Err(e) => {
                panic!("Parse error: {:?}", e);
            }
        }
    }
}

#[test]
fn test_croquis_debug() {
    let source =
        std::fs::read_to_string("../../playground/src/components/CroquisPlayground.vue").unwrap();
    println!("Source length: {}", source.len());

    // Find all <template tags manually
    let mut pos = 0;
    let mut template_positions = vec![];
    while let Some(idx) = source[pos..].find("<template") {
        let actual_pos = pos + idx;
        let context: String = source.chars().skip(actual_pos).take(50).collect();
        template_positions.push((actual_pos, context));
        pos = actual_pos + 1;
    }

    println!("\nAll <template positions:");
    for (p, ctx) in &template_positions {
        println!("  {}: {:?}", p, ctx);
    }

    let result = parse_sfc(&source, SfcParseOptions::default());
    match result {
        Ok(desc) => {
            if let Some(ref template) = desc.template {
                println!("\nParsed template tag_start: {}", template.loc.tag_start);
                let ctx: String = source
                    .chars()
                    .skip(template.loc.tag_start)
                    .take(50)
                    .collect();
                println!("Content at tag_start: {:?}", ctx);
            }
            for (i, style) in desc.styles.iter().enumerate() {
                println!(
                    "Style {}: tag offset {}..{}",
                    i, style.loc.tag_start, style.loc.tag_end
                );
            }
        }
        Err(e) => panic!("{:?}", e),
    }
}
