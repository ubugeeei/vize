//! Template compilation for Vue SFCs.
//!
//! This module handles compilation of `<template>` blocks,
//! supporting both DOM mode and Vapor mode.

use vue_allocator::Bump;
use vue_compiler_vapor::{compile_vapor, VaporCompilerOptions};

use crate::types::*;

/// Compile template block
pub(crate) fn compile_template_block(
    template: &SfcTemplateBlock,
    options: &TemplateCompileOptions,
    scope_id: &str,
    has_scoped: bool,
    is_ts: bool,
    bindings: Option<&BindingMetadata>,
) -> Result<String, SfcError> {
    let allocator = Bump::new();

    // Build DOM compiler options
    let mut dom_opts = options.compiler_options.clone().unwrap_or_default();
    dom_opts.mode = vue_compiler_core::options::CodegenMode::Module;
    dom_opts.prefix_identifiers = true;
    dom_opts.scope_id = if has_scoped {
        Some(format!("data-v-{}", scope_id).into())
    } else {
        None
    };
    dom_opts.ssr = options.ssr;
    dom_opts.is_ts = is_ts;

    // For script setup, use inline mode (render function inside setup return)
    if bindings.is_some() {
        dom_opts.inline = true;
        dom_opts.hoist_static = true;
    }

    // Pass binding metadata from script setup to template compiler
    if let Some(script_bindings) = bindings {
        let mut binding_map = rustc_hash::FxHashMap::default();
        for (name, binding_type) in &script_bindings.bindings {
            let type_str = match binding_type {
                BindingType::Data => "data",
                BindingType::Props => "props",
                BindingType::PropsAliased => "props-aliased",
                BindingType::SetupLet => "setup-let",
                BindingType::SetupConst => "setup-const",
                BindingType::SetupReactiveConst => "setup-reactive-const",
                BindingType::SetupMaybeRef => "setup-maybe-ref",
                BindingType::SetupRef => "setup-ref",
                BindingType::Options => "options",
                BindingType::LiteralConst => "literal-const",
            };
            binding_map.insert(
                vue_allocator::String::from(name.as_str()),
                vue_allocator::String::from(type_str),
            );
        }
        dom_opts.binding_metadata = Some(vue_compiler_dom::BindingMetadataMap {
            bindings: binding_map,
        });
    }

    // Compile template
    let (_, errors, result) =
        vue_compiler_dom::compile_template_with_options(&allocator, &template.content, dom_opts);

    if !errors.is_empty() {
        return Err(SfcError {
            message: format!("Template compilation errors: {:?}", errors),
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
        return Err(SfcError {
            message: format!(
                "Vapor template compilation errors: {:?}",
                result.error_messages
            ),
            code: Some("VAPOR_TEMPLATE_ERROR".to_string()),
            loc: Some(template.loc.clone()),
        });
    }

    // Process the Vapor output to extract imports and render function
    let mut output = String::new();
    let scope_attr = if has_scoped {
        format!("data-v-{}", scope_id)
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
                return format!("{}{} {}{}{}", prefix, tag_name, scope_id, rest, suffix);
            }
        }
    }
    template_line.to_string()
}

/// Extract imports, hoisted consts, and render body from compiled template code
pub(crate) fn extract_template_parts(template_code: &str) -> (String, String, String) {
    let mut imports = String::new();
    let mut hoisted = String::new();
    let mut render_body = String::new();
    let mut in_render = false;
    let mut in_return = false;
    let mut brace_depth = 0;
    let mut return_brace_depth = 0;

    for line in template_code.lines() {
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
            // Count opening braces
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
        } else if in_render {
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;

            // Extract the return statement inside the render function (may span multiple lines)
            if in_return {
                // Continue collecting return body
                render_body.push('\n');
                render_body.push_str(line);
                return_brace_depth += line.matches('(').count() as i32;
                return_brace_depth -= line.matches(')').count() as i32;

                // Check if return statement is complete
                if return_brace_depth <= 0 {
                    in_return = false;
                    // Remove trailing semicolon if present
                    let trimmed_body = render_body.trim_end();
                    if let Some(stripped) = trimmed_body.strip_suffix(';') {
                        render_body = stripped.to_string();
                    }
                }
            } else if let Some(stripped) = trimmed.strip_prefix("return ") {
                render_body = stripped.to_string();
                // Count parentheses to handle multi-line return
                return_brace_depth =
                    stripped.matches('(').count() as i32 - stripped.matches(')').count() as i32;
                if return_brace_depth > 0 {
                    in_return = true;
                } else {
                    // Single line return - remove trailing semicolon if present
                    if render_body.ends_with(';') {
                        render_body.pop();
                    }
                }
            }

            if brace_depth == 0 {
                in_render = false;
            }
        }
    }

    (imports, hoisted, render_body)
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

    #[test]
    fn test_extract_template_parts_basic() {
        let template_code = r#"import { createVNode as _createVNode } from 'vue'

const _hoisted_1 = { class: "test" }

export function render(_ctx, _cache) {
  return _createVNode("div", _hoisted_1, "Hello")
}"#;

        let (imports, hoisted, render_body) = extract_template_parts(template_code);

        assert!(imports.contains("import"));
        assert!(hoisted.contains("_hoisted_1"));
        assert!(render_body.contains("_createVNode"));
    }
}
