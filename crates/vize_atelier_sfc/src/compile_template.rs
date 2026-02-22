//! Template compilation for Vue SFCs.
//!
//! This module handles compilation of `<template>` blocks,
//! supporting both DOM mode and Vapor mode.

use vize_atelier_vapor::{compile_vapor, VaporCompilerOptions};
use vize_carton::Bump;

use crate::types::*;

/// Compile template block
pub(crate) fn compile_template_block(
    template: &SfcTemplateBlock,
    options: &TemplateCompileOptions,
    scope_id: &str,
    has_scoped: bool,
    is_ts: bool,
    bindings: Option<&BindingMetadata>,
    croquis: Option<vize_croquis::analysis::Croquis>,
) -> Result<String, SfcError> {
    let allocator = Bump::new();

    // Build DOM compiler options
    let mut dom_opts = options.compiler_options.clone().unwrap_or_default();
    dom_opts.mode = vize_atelier_core::options::CodegenMode::Module;
    dom_opts.prefix_identifiers = true;
    dom_opts.scope_id = if has_scoped {
        let mut attr = String::with_capacity(scope_id.len() + 7);
        attr.push_str("data-v-");
        attr.push_str(scope_id);
        Some(attr.into())
    } else {
        None
    };
    dom_opts.ssr = options.ssr;
    dom_opts.is_ts = is_ts;

    // For script setup, use inline mode to match Vue's actual compiler behavior
    // Inline mode generates direct closure references (e.g., msg instead of $setup.msg)
    // which are captured in the setup() function scope
    if bindings.is_some() {
        dom_opts.inline = true;
        dom_opts.hoist_static = true;
        dom_opts.cache_handlers = true;
    }

    // Pass binding metadata directly (no string conversion needed)
    dom_opts.binding_metadata = bindings.cloned();

    // Pass Croquis to DOM compiler for enhanced transforms
    if let Some(c) = croquis {
        dom_opts.croquis = Some(Box::new(c));
    }

    // Compile template
    let (_, errors, result) =
        vize_atelier_dom::compile_template_with_options(&allocator, &template.content, dom_opts);

    if !errors.is_empty() {
        let mut message = String::from("Template compilation errors: ");
        use std::fmt::Write as _;
        let _ = write!(&mut message, "{:?}", errors);
        return Err(SfcError {
            message,
            code: Some("TEMPLATE_ERROR".to_string()),
            loc: Some(template.loc.clone()),
        });
    }

    // Generate render function with proper imports
    let mut output = String::new();

    // Add Vue imports
    output.push_str(&result.preamble);
    output.push('\n');

    // The codegen already generates a complete function with closing brace,
    // so we just need to use it directly
    output.push_str(&result.code);
    output.push('\n');

    Ok(output)
}

/// Compile template block using Vapor mode
pub(crate) fn compile_template_block_vapor(
    template: &SfcTemplateBlock,
    scope_id: &str,
    has_scoped: bool,
) -> Result<String, SfcError> {
    let allocator = Bump::new();

    // Build Vapor compiler options
    let vapor_opts = VaporCompilerOptions {
        prefix_identifiers: false,
        ssr: false,
        ..Default::default()
    };

    // Compile template with Vapor
    let result = compile_vapor(&allocator, &template.content, vapor_opts);

    if !result.error_messages.is_empty() {
        let mut message = String::from("Vapor template compilation errors: ");
        use std::fmt::Write as _;
        let _ = write!(&mut message, "{:?}", result.error_messages);
        return Err(SfcError {
            message,
            code: Some("VAPOR_TEMPLATE_ERROR".to_string()),
            loc: Some(template.loc.clone()),
        });
    }

    // Process the Vapor output to extract imports and render function
    let mut output = String::new();
    let scope_attr = if has_scoped {
        let mut attr = String::with_capacity(scope_id.len() + 7);
        attr.push_str("data-v-");
        attr.push_str(scope_id);
        attr
    } else {
        String::new()
    };

    // Parse the Vapor output to separate imports and function body
    let code = &result.code;

    // Extract import line
    if let Some(import_end) = code.find('\n') {
        let import_line = &code[..import_end];
        // Rewrite import to use 'vue' instead of 'vue/vapor' for compatibility
        output.push_str(import_line);
        output.push('\n');

        // Extract template declarations and function body
        let rest = &code[import_end + 1..];

        // Find template declarations (const tN = ...)
        let mut template_decls = Vec::new();
        let mut func_start = 0;
        for (i, line) in rest.lines().enumerate() {
            if line.starts_with("const t") && line.contains("_template(") {
                // Add scope ID to template if scoped
                if has_scoped && !scope_attr.is_empty() {
                    let modified = add_scope_id_to_template(line, &scope_attr);
                    template_decls.push(modified);
                } else {
                    template_decls.push(line.to_string());
                }
            } else if line.starts_with("export default") {
                func_start = i;
                break;
            }
        }

        // Output template declarations
        for decl in template_decls {
            output.push_str(&decl);
            output.push('\n');
        }

        // Extract and convert the function body
        let lines: Vec<&str> = rest.lines().collect();
        if func_start < lines.len() {
            // Convert "export default () => {" to "function render(_ctx, $props, $emit, $attrs, $slots) {"
            output.push_str("function render(_ctx, $props, $emit, $attrs, $slots) {\n");

            // Copy function body (skip "export default () => {" and final "}")
            for line in lines.iter().skip(func_start + 1) {
                if *line == "}" {
                    break;
                }
                output.push_str(line);
                output.push('\n');
            }

            output.push_str("}\n");
        }
    }

    Ok(output)
}

/// Add scope ID to template string
fn add_scope_id_to_template(template_line: &str, scope_id: &str) -> String {
    // Find the template string content and add scope_id to the first element
    if let Some(start) = template_line.find("\"<") {
        if let Some(end) = template_line.rfind(">\"") {
            let prefix = &template_line[..start + 2]; // up to and including "<"
            let content = &template_line[start + 2..end + 1]; // element content
            let suffix = &template_line[end + 1..]; // closing quote and paren

            // Find end of first tag name
            if let Some(tag_end) = content.find(|c: char| c.is_whitespace() || c == '>') {
                let tag_name = &content[..tag_end];
                let rest = &content[tag_end..];

                // Insert scope_id attribute after tag name
                let mut result = String::with_capacity(
                    prefix.len() + tag_name.len() + scope_id.len() + rest.len() + suffix.len() + 1,
                );
                result.push_str(prefix);
                result.push_str(tag_name);
                result.push(' ');
                result.push_str(scope_id);
                result.push_str(rest);
                result.push_str(suffix);
                return result;
            }
        }
    }
    template_line.to_string()
}

/// State for tracking string/template literal context across multiple lines.
/// Required because template literals (backtick strings) can span multiple lines,
/// and `${...}` expressions within them contain code-mode braces that must be
/// distinguished from surrounding code braces.
#[derive(Clone)]
struct StringTrackState {
    /// Whether we're inside a string literal (regular or template literal text portion)
    in_string: bool,
    /// The quote character of the current string ('\'' | '"' | '`')
    string_char: char,
    /// Whether the last character was a backslash escape
    escape: bool,
    /// Stack for nested template literal `${...}` expressions.
    /// Each entry tracks the brace depth within that expression.
    /// When a `}` is encountered and the top depth is 0, the expression ends
    /// and we return to the enclosing template literal text.
    template_expr_brace_stack: Vec<i32>,
}

impl Default for StringTrackState {
    fn default() -> Self {
        Self {
            in_string: false,
            string_char: '\0',
            escape: false,
            template_expr_brace_stack: Vec::new(),
        }
    }
}

/// Count net brace depth change ({ minus }) in a line, properly tracking
/// string literals and template literal `${...}` expressions.
/// State is carried across lines to handle multiline template literals.
fn count_braces_with_state(line: &str, state: &mut StringTrackState) -> i32 {
    let mut count: i32 = 0;
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        if state.escape {
            state.escape = false;
            i += 1;
            continue;
        }

        if state.in_string {
            if ch == '\\' {
                state.escape = true;
                i += 1;
                continue;
            }

            if state.string_char == '`' {
                if ch == '`' {
                    // Close template literal
                    state.in_string = false;
                } else if ch == '$' && i + 1 < len && chars[i + 1] == '{' {
                    // Enter template expression ${...}
                    // The ${ is template syntax, not a code brace
                    state.in_string = false;
                    state.template_expr_brace_stack.push(0);
                    i += 2; // Skip both $ and {
                    continue;
                }
            } else if ch == state.string_char {
                // Close regular string (' or ")
                state.in_string = false;
            }
        } else {
            // Not in string - we're in code mode
            match ch {
                '\'' | '"' => {
                    state.in_string = true;
                    state.string_char = ch;
                }
                '`' => {
                    state.in_string = true;
                    state.string_char = '`';
                }
                '{' => {
                    if let Some(depth) = state.template_expr_brace_stack.last_mut() {
                        *depth += 1;
                    }
                    count += 1;
                }
                '}' => {
                    if let Some(&depth) = state.template_expr_brace_stack.last() {
                        if depth == 0 {
                            // This } closes the ${...} expression, not a code brace
                            state.template_expr_brace_stack.pop();
                            state.in_string = true;
                            state.string_char = '`';
                            i += 1;
                            continue;
                        } else {
                            // Nested brace inside ${...} expression
                            *state.template_expr_brace_stack.last_mut().unwrap() -= 1;
                            count -= 1;
                        }
                    } else {
                        count -= 1;
                    }
                }
                _ => {}
            }
        }

        i += 1;
    }

    count
}

/// Count net brace depth change ({  minus }) in a line, ignoring braces inside string literals.
/// NOTE: This function does NOT track state across lines. For multiline template literals,
/// use `count_braces_with_state` instead.
#[cfg(test)]
fn count_braces_outside_strings(line: &str) -> i32 {
    let mut state = StringTrackState::default();
    count_braces_with_state(line, &mut state)
}

/// Compact render body by removing unnecessary line breaks inside function calls and arrays
#[allow(dead_code)]
fn compact_render_body(render_body: &str) -> String {
    let mut result = String::new();
    let mut chars = render_body.chars().peekable();
    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut brace_depth: i32 = 0;
    let mut in_string = false;
    let mut string_char = '\0';
    let mut in_template = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' | '\'' if !in_template => {
                if !in_string {
                    in_string = true;
                    string_char = ch;
                } else if string_char == ch {
                    in_string = false;
                }
                result.push(ch);
            }
            '`' => {
                in_template = !in_template;
                result.push(ch);
            }
            '(' if !in_string && !in_template => {
                paren_depth += 1;
                result.push(ch);
            }
            ')' if !in_string && !in_template => {
                paren_depth = paren_depth.saturating_sub(1);
                result.push(ch);
            }
            '[' if !in_string && !in_template => {
                bracket_depth += 1;
                result.push(ch);
            }
            ']' if !in_string && !in_template => {
                bracket_depth = bracket_depth.saturating_sub(1);
                result.push(ch);
            }
            '{' if !in_string && !in_template => {
                brace_depth += 1;
                result.push(ch);
            }
            '}' if !in_string && !in_template => {
                brace_depth = brace_depth.saturating_sub(1);
                result.push(ch);
            }
            '\n' => {
                // If inside braces (block bodies), keep newlines to preserve statement separation
                if brace_depth > 0 && !in_string && !in_template {
                    result.push('\n');
                } else if (paren_depth > 0 || bracket_depth > 0) && !in_string && !in_template {
                    result.push(' ');
                    // Skip following whitespace
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_whitespace() && next_ch != '\n' {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                } else {
                    // Keep newline outside of function calls/arrays or inside strings
                    result.push(ch);
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

/// Extract imports, hoisted consts, and render function from compiled template code
/// Returns (imports, hoisted, render_function) where render_function is the full function definition
pub(crate) fn extract_template_parts_full(template_code: &str) -> (String, String, String) {
    let mut imports = String::new();
    let mut hoisted = String::new();
    let mut render_fn = String::new();
    let mut in_render = false;
    let mut brace_depth = 0;
    let mut brace_state = StringTrackState::default();

    for line in template_code.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("import ") {
            imports.push_str(line);
            imports.push('\n');
        } else if trimmed.starts_with("const _hoisted_") {
            hoisted.push_str(line);
            hoisted.push('\n');
        } else if trimmed.starts_with("export function render(")
            || trimmed.starts_with("function render(")
        {
            in_render = true;
            brace_depth = 0;
            brace_state = StringTrackState::default();
            brace_depth += count_braces_with_state(line, &mut brace_state);
            render_fn.push_str(line);
            render_fn.push('\n');
        } else if in_render {
            brace_depth += count_braces_with_state(line, &mut brace_state);
            render_fn.push_str(line);
            render_fn.push('\n');

            if brace_depth == 0 {
                in_render = false;
            }
        }
    }

    (imports, hoisted, render_fn)
}

/// Extract imports, hoisted consts, preamble (component/directive resolution), and render body
/// from compiled template code.
/// Returns (imports, hoisted, preamble, render_body)
#[allow(dead_code)]
pub(crate) fn extract_template_parts(template_code: &str) -> (String, String, String, String) {
    let mut imports = String::new();
    let mut hoisted = String::new();
    let mut preamble = String::new(); // Component/directive resolution statements
    let mut render_body = String::new();
    let mut in_render = false;
    let mut in_return = false;
    let mut brace_depth = 0;
    let mut brace_state = StringTrackState::default();
    let mut return_paren_depth = 0;

    // Collect all lines for look-ahead
    let lines: Vec<&str> = template_code.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with("import ") {
            imports.push_str(line);
            imports.push('\n');
        } else if trimmed.starts_with("const _hoisted_") {
            // Hoisted template variables
            hoisted.push_str(line);
            hoisted.push('\n');
        } else if trimmed.starts_with("export function render(")
            || trimmed.starts_with("function render(")
        {
            in_render = true;
            brace_depth = 0;
            brace_state = StringTrackState::default();
            brace_depth += count_braces_with_state(line, &mut brace_state);
        } else if in_render {
            brace_depth += count_braces_with_state(line, &mut brace_state);

            // Extract the return statement inside the render function (may span multiple lines)
            if in_return {
                // Continue collecting return body
                render_body.push('\n');
                render_body.push_str(line);
                return_paren_depth += line.matches('(').count() as i32;
                return_paren_depth -= line.matches(')').count() as i32;

                // Check if return statement is complete:
                // - Parentheses must be balanced (return_paren_depth <= 0)
                // - Next non-empty line must NOT be a ternary continuation (? or :)
                if return_paren_depth <= 0 {
                    // Look ahead to check for ternary continuation
                    let next_continues_ternary = lines
                        .iter()
                        .skip(i + 1)
                        .map(|l| l.trim())
                        .find(|l| !l.is_empty())
                        .map(|l| l.starts_with('?') || l.starts_with(':'))
                        .unwrap_or(false);

                    if !next_continues_ternary {
                        in_return = false;
                        // Remove trailing semicolon if present
                        let trimmed_body = render_body.trim_end();
                        if let Some(stripped) = trimmed_body.strip_suffix(';') {
                            render_body = stripped.to_string();
                        }
                    }
                }
            } else if let Some(stripped) = trimmed.strip_prefix("return ") {
                render_body = stripped.to_string();
                // Count parentheses to handle multi-line return
                return_paren_depth =
                    stripped.matches('(').count() as i32 - stripped.matches(')').count() as i32;
                if return_paren_depth > 0 {
                    in_return = true;
                } else {
                    // Check if next non-empty line is a ternary continuation
                    let next_continues_ternary = lines
                        .iter()
                        .skip(i + 1)
                        .map(|l| l.trim())
                        .find(|l| !l.is_empty())
                        .map(|l| l.starts_with('?') || l.starts_with(':'))
                        .unwrap_or(false);

                    if next_continues_ternary {
                        in_return = true;
                    } else {
                        // Single line return - remove trailing semicolon if present
                        if render_body.ends_with(';') {
                            render_body.pop();
                        }
                    }
                }
            } else if trimmed.starts_with("const _component_")
                || trimmed.starts_with("const _directive_")
            {
                // Component/directive resolution statements go in preamble
                preamble.push_str(trimmed);
                preamble.push('\n');
            }

            if brace_depth == 0 {
                in_render = false;
            }
        }
    }

    // Compact the render body to remove unnecessary line breaks inside function calls
    let compacted = compact_render_body(&render_body);

    (imports, hoisted, preamble, compacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_scope_id_to_template() {
        let input = r#"const t0 = _template("<div class='container'>Hello</div>")"#;
        let result = add_scope_id_to_template(input, "data-v-abc123");
        assert!(result.contains("data-v-abc123"));
    }

    // --- count_braces_outside_strings tests ---

    #[test]
    fn test_count_braces_normal() {
        assert_eq!(count_braces_outside_strings("{ a: 1 }"), 0);
        assert_eq!(count_braces_outside_strings("{"), 1);
        assert_eq!(count_braces_outside_strings("}"), -1);
        assert_eq!(count_braces_outside_strings("{ { }"), 1);
    }

    #[test]
    fn test_count_braces_ignores_string_braces() {
        assert_eq!(
            count_braces_outside_strings("_toDisplayString(isArray.value ? ']' : '}')"),
            0
        );
        assert_eq!(count_braces_outside_strings(r#"var x = "{";"#), 0);
        assert_eq!(count_braces_outside_strings("var x = `{`;"), 0);
    }

    #[test]
    fn test_count_braces_mixed_string_and_code() {
        assert_eq!(count_braces_outside_strings("if (x) { var s = '}'"), 1);
    }

    #[test]
    fn test_count_braces_escaped_quotes() {
        assert_eq!(count_braces_outside_strings(r"var x = '\'' + '}'"), 0);
    }

    #[test]
    fn test_extract_template_parts_full_brace_in_string() {
        let template_code = r#"import { toDisplayString as _toDisplayString } from 'vue'

export function render(_ctx, _cache) {
  return _toDisplayString(isArray.value ? ']' : '}')
}"#;

        let (imports, _hoisted, render_fn) = extract_template_parts_full(template_code);

        assert!(imports.contains("import"));
        assert!(
            render_fn.contains("_toDisplayString"),
            "Render function was truncated. Got:\n{}",
            render_fn
        );
        let trimmed = render_fn.trim();
        assert!(
            trimmed.ends_with('}'),
            "Render function should end with closing brace. Got:\n{}",
            render_fn
        );
    }

    #[test]
    fn test_extract_template_parts_basic() {
        let template_code = r#"import { createVNode as _createVNode } from 'vue'

const _hoisted_1 = { class: "test" }

export function render(_ctx, _cache) {
  return _createVNode("div", _hoisted_1, "Hello")
}"#;

        let (imports, hoisted, _preamble, render_body) = extract_template_parts(template_code);

        assert!(imports.contains("import"));
        assert!(hoisted.contains("_hoisted_1"));
        assert!(render_body.contains("_createVNode"));
    }

    // --- Multiline template literal tests ---

    #[test]
    fn test_count_braces_multiline_template_literal() {
        // Simulate processing lines of a multiline template literal
        // from actual codegen output
        let mut state = StringTrackState::default();

        // Line with } closing object, then opening a template literal with ${
        // Content: }, _toDisplayString(`${t("key")}: v${ver.major}.${
        // Using concat! to avoid raw string delimiter issues
        let line1 = concat!(r#"}, _toDisplayString(`${t("key")}: v${ver.major}.${"#);
        let count1 = count_braces_with_state(line1, &mut state);
        // The } before _toDisplayString is a code brace (-1)
        // Template literal opens with `, then ${...} are template expression syntax
        assert_eq!(count1, -1, "Line 1 brace count");
        // After ${t("key")} closes and ${ver.major} closes, we enter ${ again
        // The line ends inside ${ expression inside template literal
        assert!(
            !state.template_expr_brace_stack.is_empty(),
            "Should be inside template expression after line 1"
        );

        // Line inside template expression (just expression content)
        let line2 = "            ver.minor";
        let count2 = count_braces_with_state(line2, &mut state);
        assert_eq!(count2, 0, "Line 2 brace count");

        // Line with } closing ${...} expression, ` closing template literal
        let line3 = r##"          }`) + "\n      ", 1 /* TEXT */)))"##;
        let count3 = count_braces_with_state(line3, &mut state);
        // The } closes ${...} (not a code brace), ` closes template literal
        assert_eq!(count3, 0, "Line 3 brace count");
        assert!(!state.in_string, "Should be outside string after line 3");
        assert!(
            state.template_expr_brace_stack.is_empty(),
            "Template expression stack should be empty"
        );

        // Total brace change should be -1 (only the } before _toDisplayString)
        assert_eq!(count1 + count2 + count3, -1);
    }

    #[test]
    fn test_extract_template_parts_multiline_template_literal() {
        // This is the actual pattern that was causing the bug:
        // a render function with a multiline template literal inside ${}
        let template_code = r#"import { openBlock as _openBlock, createElementBlock as _createElementBlock, toDisplayString as _toDisplayString, createCommentVNode as _createCommentVNode } from "vue"

export function render(_ctx, _cache, $props, $setup, $data, $options) {
  return (show.value)
    ? (_openBlock(), _createElementBlock("div", {
      key: 0,
      class: "outer"
    }, [
      _createElementVNode("div", { class: "inner" }, [
        (ver.value)
          ? (_openBlock(), _createElementBlock("span", { key: 0 }, "\n        " + _toDisplayString(`${t("key")}: v${ver.value.major}.${
            ver.value.minor
          }`) + "\n      ", 1 /* TEXT */))
          : (_openBlock(), _createElementBlock("span", { key: 1 }, "no"))
      ])
    ]))
    : _createCommentVNode("v-if", true)
}"#;

        let (_imports, _hoisted, _preamble, render_body) = extract_template_parts(template_code);

        // The render body must contain both v-if branches and the comment node
        assert!(
            render_body.contains("_toDisplayString"),
            "Should contain the template literal expression. Got:\n{}",
            render_body
        );
        assert!(
            render_body.contains("_createCommentVNode"),
            "Should contain the v-if comment node (else branch). Got:\n{}",
            render_body
        );
        assert!(
            render_body.contains("\"no\""),
            "Should contain the v-else branch content. Got:\n{}",
            render_body
        );
    }

    #[test]
    fn test_extract_template_parts_full_multiline_template_literal() {
        let template_code = r#"import { toDisplayString as _toDisplayString } from 'vue'

export function render(_ctx, _cache) {
  return _toDisplayString(`${t("key")}: v${ver.major}.${
    ver.minor
  }`)
}"#;

        let (_imports, _hoisted, render_fn) = extract_template_parts_full(template_code);

        assert!(
            render_fn.contains("_toDisplayString"),
            "Render function should contain the expression. Got:\n{}",
            render_fn
        );
        let trimmed = render_fn.trim();
        assert!(
            trimmed.ends_with('}'),
            "Render function should end with closing brace. Got:\n{}",
            render_fn
        );
    }

    // --- Generalized template literal / string tracking tests ---

    #[test]
    fn test_count_braces_template_literal_with_nested_object() {
        // Template literal containing ${...} with nested object literal: `${fn({a: 1})}`
        let mut state = StringTrackState::default();
        let line = r#"x = `result: ${fn({a: 1, b: {c: 2}})}`"#;
        let count = count_braces_with_state(line, &mut state);
        // No code-level braces; all { and } are inside template literal expressions
        assert_eq!(
            count, 0,
            "Braces inside template expression should be balanced"
        );
        assert!(!state.in_string, "Template literal should be closed");
    }

    #[test]
    fn test_count_braces_nested_template_literals() {
        // Nested template literals: `outer ${`inner ${x}`} end`
        let mut state = StringTrackState::default();
        let line = r#"x = `outer ${`inner ${x}`} end`"#;
        let count = count_braces_with_state(line, &mut state);
        assert_eq!(
            count, 0,
            "Nested template literals should not affect brace count"
        );
        assert!(!state.in_string, "All template literals should be closed");
    }

    #[test]
    fn test_count_braces_multiline_template_expr_with_object() {
        // Multiline: template expression contains an object literal spanning lines
        let mut state = StringTrackState::default();

        // `value: ${fn({`  — the { after fn( is a real code brace inside the expr
        let line1 = r#"x = `value: ${fn({"#;
        let c1 = count_braces_with_state(line1, &mut state);
        assert_eq!(
            c1, 1,
            "Line 1: object literal brace inside template expression"
        );

        let line2 = r#"  key: val"#;
        let c2 = count_braces_with_state(line2, &mut state);
        assert_eq!(c2, 0, "Line 2: no braces");

        // })}` — the first } closes the object (-1), the second } closes ${...} (not counted)
        let line3 = r#"})}`"#;
        let c3 = count_braces_with_state(line3, &mut state);
        assert_eq!(c3, -1, "Line 3: closing object brace");
        assert!(!state.in_string, "Template literal should be closed");
        assert_eq!(c1 + c2 + c3, 0, "Total should be balanced");
    }

    #[test]
    fn test_count_braces_template_literal_with_arrow_function() {
        // Template expression with arrow function: `${items.map(x => ({ name: x })).join()}`
        let mut state = StringTrackState::default();
        let line = r#"x = `${items.map(x => ({ name: x })).join()}`"#;
        let count = count_braces_with_state(line, &mut state);
        assert_eq!(count, 0);
        assert!(!state.in_string);
    }

    #[test]
    fn test_count_braces_state_across_many_lines() {
        // Simulate a real render function with multiline template literal
        let mut state = StringTrackState::default();

        // function render() {
        let c1 = count_braces_with_state("function render() {", &mut state);
        assert_eq!(c1, 1);

        //   return _toDisplayString(`${fn({
        // The { after fn( is a code brace inside the template expression (+1)
        let c2 = count_braces_with_state(r#"  return _toDisplayString(`${fn({"#, &mut state);
        assert_eq!(c2, 1, "Object literal brace inside template expression");

        //     key: val,
        let c3 = count_braces_with_state("    key: val,", &mut state);
        assert_eq!(c3, 0);

        //     nested: {
        // This { is a nested code brace inside the template expression
        let c4 = count_braces_with_state("    nested: {", &mut state);
        assert_eq!(c4, 1, "Nested brace inside template expression");

        //       deep: true
        let c5 = count_braces_with_state("      deep: true", &mut state);
        assert_eq!(c5, 0);

        //     }
        // This } closes the nested object brace
        let c6 = count_braces_with_state("    }", &mut state);
        assert_eq!(c6, -1, "Closing nested brace inside template expression");

        //   })}`)
        // The first } closes the outer object (-1), the second } closes ${...} (not counted)
        let c7 = count_braces_with_state(r#"  })}`)"#, &mut state);
        assert_eq!(c7, -1, "Closing outer object brace");

        // }
        let c8 = count_braces_with_state("}", &mut state);
        assert_eq!(c8, -1);

        assert_eq!(
            c1 + c2 + c3 + c4 + c5 + c6 + c7 + c8,
            0,
            "Total: function opens and closes"
        );
        assert!(!state.in_string);
        assert!(state.template_expr_brace_stack.is_empty());
    }

    #[test]
    fn test_count_braces_regular_strings_with_braces() {
        // Single and double quoted strings with braces should still work
        let mut state = StringTrackState::default();

        let line = r#"if (x) { var s = "}" + '{' }"#;
        let count = count_braces_with_state(line, &mut state);
        assert_eq!(count, 0, "Braces inside regular strings should be ignored");
    }

    #[test]
    fn test_extract_template_parts_deeply_nested_multiline() {
        // Multiple levels of nesting with multiline template literals
        let template_code = r#"import { toDisplayString as _toDisplayString, createElementBlock as _createElementBlock, openBlock as _openBlock, createCommentVNode as _createCommentVNode, createElementVNode as _createElementVNode } from "vue"

export function render(_ctx, _cache, $props, $setup, $data, $options) {
  return (cond.value)
    ? (_openBlock(), _createElementBlock("div", { key: 0 }, [
        _createElementVNode("p", null, _toDisplayString(`${items.value.map(x => ({
          name: x.name,
          label: `${x.prefix}-${
            x.suffix
          }`
        })).length} items`)),
        _createElementVNode("span", null, "after")
      ]))
    : _createCommentVNode("v-if", true)
}"#;

        let (_imports, _hoisted, _preamble, render_body) = extract_template_parts(template_code);

        assert!(
            render_body.contains("_createCommentVNode"),
            "Should contain comment node (else branch). Got:\n{}",
            render_body
        );
        assert!(
            render_body.contains("\"after\""),
            "Should contain content after template literal. Got:\n{}",
            render_body
        );
    }

    #[test]
    fn test_extract_template_parts_full_deeply_nested_multiline() {
        // Same deep nesting test for extract_template_parts_full
        let template_code = r#"import { toDisplayString as _toDisplayString } from "vue"

export function render(_ctx, _cache) {
  return _toDisplayString(`${items.map(x => ({
    name: x.name,
    value: `nested-${
      x.value
    }`
  })).length} items`)
}"#;

        let (_imports, _hoisted, render_fn) = extract_template_parts_full(template_code);

        let trimmed = render_fn.trim();
        assert!(
            trimmed.ends_with('}'),
            "Render function should end with closing brace. Got:\n{}",
            render_fn
        );
        assert!(
            render_fn.contains("items"),
            "Render function should contain the full expression. Got:\n{}",
            render_fn
        );
    }
}
