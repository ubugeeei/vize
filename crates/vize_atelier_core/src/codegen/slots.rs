//! Slot generation functions.
//!
//! Generates slot objects for component children.

use crate::ast::*;
use crate::transforms::v_slot::{collect_slots, get_slot_name, has_v_slot};

use super::context::CodegenContext;
use super::helpers::{escape_js_string, is_valid_js_identifier};
use super::node::generate_node;

/// Get slot props expression as raw source (not transformed)
fn get_slot_props(dir: &DirectiveNode<'_>) -> Option<vize_carton::String> {
    dir.exp.as_ref().map(|exp| match exp {
        ExpressionNode::Simple(s) => s.loc.source.clone(),
        ExpressionNode::Compound(c) => c.loc.source.clone(),
    })
}

/// Add _ctx. prefix to default value identifiers in destructuring patterns.
/// e.g., "{ item = defaultItem }" -> "{ item = _ctx.defaultItem }"
/// Only processes identifiers after `=` (default values), not the param names.
fn prefix_slot_defaults(source: &str) -> std::string::String {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut result = std::string::String::with_capacity(len + 20);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'=' {
            // Skip == and =>
            if i + 1 < len && (bytes[i + 1] == b'=' || bytes[i + 1] == b'>') {
                result.push(bytes[i] as char);
                result.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            result.push('=');
            i += 1;
            // Skip whitespace after =
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t') {
                result.push(bytes[i] as char);
                i += 1;
            }
            // Check if next is a simple identifier (not a literal/number/string/object)
            if i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b'$') {
                // Collect the identifier
                let start = i;
                while i < len
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
                {
                    i += 1;
                }
                let ident = &source[start..i];
                // Don't prefix keywords/literals
                if !matches!(
                    ident,
                    "true" | "false" | "null" | "undefined" | "NaN" | "Infinity"
                ) {
                    result.push_str("_ctx.");
                }
                result.push_str(ident);
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Extract parameter names from slot props expression
/// e.g., "{ item }" -> ["item"], "{ item, index }" -> ["item", "index"]
/// e.g., "slotProps" -> ["slotProps"]
fn extract_slot_params(props_str: &str) -> Vec<std::string::String> {
    let mut params = Vec::new();
    super::v_for::extract_destructure_params(props_str.trim(), &mut params);
    params
}

/// Check if component has slot children that need to be generated as slots object
pub fn has_slot_children(el: &ElementNode<'_>) -> bool {
    if el.children.is_empty() {
        return false;
    }

    // Teleport and KeepAlive pass children as arrays, not slot objects
    if matches!(el.tag.as_str(), "Teleport" | "KeepAlive") {
        return false;
    }

    // Check for v-slot on component root
    for prop in &el.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name.as_str() == "slot" {
                return true;
            }
        }
    }

    // Check for any children (default slot) or template slots
    true
}

/// Check if component has dynamic slots (requires DYNAMIC_SLOTS patch flag)
pub fn has_dynamic_slots_flag(el: &ElementNode<'_>) -> bool {
    let collected_slots = collect_slots(el);
    collected_slots.iter().any(|s| s.is_dynamic)
}

/// Generate slots object for component
pub fn generate_slots(ctx: &mut CodegenContext, el: &ElementNode<'_>) {
    // Note: WithCtx helper is registered at each _withCtx() output site,
    // not here, to avoid importing it when slots don't actually use it.

    // Check for v-slot on component root (shorthand for default slot)
    let root_slot = el.props.iter().find_map(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name.as_str() == "slot" {
                return Some(dir.as_ref());
            }
        }
        None
    });

    let collected_slots = collect_slots(el);
    let has_dynamic_slots = collected_slots.iter().any(|s| s.is_dynamic);

    ctx.push("{");
    ctx.indent();

    if let Some(slot_dir) = root_slot {
        // v-slot on component root - all children go to default slot
        ctx.newline();
        ctx.push("default: ");
        ctx.use_helper(RuntimeHelper::WithCtx);
        ctx.push(ctx.helper(RuntimeHelper::WithCtx));
        ctx.push("(");
        // Slot props (scoped slot params) - use raw source with default value prefix
        let params = if let Some(props_str) = get_slot_props(slot_dir) {
            let processed = prefix_slot_defaults(&props_str);
            ctx.push("(");
            ctx.push(&processed);
            ctx.push(")");
            extract_slot_params(&props_str)
        } else {
            ctx.push("()");
            vec![]
        };

        // Track slot params for stripping _ctx. prefix
        ctx.add_slot_params(&params);

        ctx.push(" => [");
        ctx.indent();
        generate_slot_children(ctx, &el.children);
        ctx.deindent();
        ctx.newline();
        ctx.push("])");

        // Remove slot params
        ctx.remove_slot_params(&params);
    } else {
        // Check for named slots via template#slotName
        let mut has_generated_default = false;
        let mut first_slot = true;

        for child in &el.children {
            if let TemplateChildNode::Element(template_el) = child {
                if template_el.tag.as_str() == "template" && has_v_slot(template_el) {
                    // This is a named slot template
                    if let Some(slot_dir) = template_el.props.iter().find_map(|p| {
                        if let PropNode::Directive(dir) = p {
                            if dir.name.as_str() == "slot" {
                                return Some(dir.as_ref());
                            }
                        }
                        None
                    }) {
                        if !first_slot {
                            ctx.push(",");
                        }
                        first_slot = false;
                        ctx.newline();

                        let slot_name = get_slot_name(slot_dir);
                        let is_dynamic = slot_dir
                            .arg
                            .as_ref()
                            .map(|arg| match arg {
                                ExpressionNode::Simple(exp) => !exp.is_static,
                                ExpressionNode::Compound(_) => true,
                            })
                            .unwrap_or(false);

                        if is_dynamic {
                            // Dynamic slot name: [_ctx.slotName]
                            ctx.push("[");
                            ctx.push("_ctx.");
                            ctx.push(&slot_name);
                            ctx.push("]");
                        } else if is_valid_js_identifier(&slot_name) {
                            ctx.push(&slot_name);
                        } else {
                            ctx.push("\"");
                            ctx.push(&escape_js_string(&slot_name));
                            ctx.push("\"");
                        }

                        if slot_name.as_str() == "default" {
                            has_generated_default = true;
                        }

                        ctx.push(": ");
                        ctx.use_helper(RuntimeHelper::WithCtx);
                        ctx.push(ctx.helper(RuntimeHelper::WithCtx));
                        ctx.push("(");

                        // Slot props - use raw source with default value prefix
                        let params = if let Some(props_str) = get_slot_props(slot_dir) {
                            let processed = prefix_slot_defaults(&props_str);
                            ctx.push("(");
                            ctx.push(&processed);
                            ctx.push(")");
                            extract_slot_params(&props_str)
                        } else {
                            ctx.push("()");
                            vec![]
                        };

                        // Track slot params for stripping _ctx. prefix
                        ctx.add_slot_params(&params);

                        ctx.push(" => [");
                        ctx.indent();
                        generate_slot_children(ctx, &template_el.children);
                        ctx.deindent();
                        ctx.newline();
                        ctx.push("])");

                        // Remove slot params
                        ctx.remove_slot_params(&params);
                    }
                }
            }
        }

        // Generate default slot for non-template children
        let default_children: Vec<_> = el
            .children
            .iter()
            .filter(|child| {
                if let TemplateChildNode::Element(template_el) = child {
                    !(template_el.tag.as_str() == "template" && has_v_slot(template_el))
                } else {
                    true
                }
            })
            .collect();

        if !default_children.is_empty() && !has_generated_default {
            if !first_slot {
                ctx.push(",");
            }
            ctx.newline();
            ctx.push("default: ");
            ctx.use_helper(RuntimeHelper::WithCtx);
            ctx.push(ctx.helper(RuntimeHelper::WithCtx));
            ctx.push("(() => [");
            ctx.indent();
            for (i, child) in default_children.iter().enumerate() {
                if i > 0 {
                    ctx.push(",");
                }
                ctx.newline();
                generate_slot_child_node(ctx, child);
            }
            ctx.deindent();
            ctx.newline();
            ctx.push("])");
        }
    }

    // Add slot stability flag
    ctx.push(",");
    ctx.newline();
    if has_dynamic_slots {
        ctx.push("_: 2 /* DYNAMIC */");
    } else {
        ctx.push("_: 1 /* STABLE */");
    }

    ctx.deindent();
    ctx.newline();
    ctx.push("}");
}

/// Generate children for a slot
fn generate_slot_children(ctx: &mut CodegenContext, children: &[TemplateChildNode<'_>]) {
    // Check if all children are text/interpolation - if so, concatenate into single _createTextVNode
    let all_text_or_interp = children.iter().all(|child| {
        matches!(
            child,
            TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
        )
    });

    if all_text_or_interp && !children.is_empty() {
        ctx.newline();
        ctx.use_helper(RuntimeHelper::CreateText);
        ctx.push(ctx.helper(RuntimeHelper::CreateText));
        ctx.push("(");

        let has_interpolation = children
            .iter()
            .any(|c| matches!(c, TemplateChildNode::Interpolation(_)));

        for (i, child) in children.iter().enumerate() {
            if i > 0 {
                ctx.push(" + ");
            }
            match child {
                TemplateChildNode::Text(text) => {
                    ctx.push("\"");
                    ctx.push(&super::helpers::escape_js_string(&text.content));
                    ctx.push("\"");
                }
                TemplateChildNode::Interpolation(interp) => {
                    ctx.use_helper(RuntimeHelper::ToDisplayString);
                    ctx.push(ctx.helper(RuntimeHelper::ToDisplayString));
                    ctx.push("(");
                    generate_slot_expression(ctx, &interp.content);
                    ctx.push(")");
                }
                _ => {}
            }
        }

        if has_interpolation {
            ctx.push(", 1 /* TEXT */)");
        } else {
            ctx.push(")");
        }
    } else {
        for (i, child) in children.iter().enumerate() {
            if i > 0 {
                ctx.push(",");
            }
            ctx.newline();
            generate_slot_child_node(ctx, child);
        }
    }
}

/// Generate a single child node for slot content
fn generate_slot_child_node(ctx: &mut CodegenContext, child: &TemplateChildNode<'_>) {
    match child {
        TemplateChildNode::Text(text) => {
            ctx.use_helper(RuntimeHelper::CreateText);
            ctx.push(ctx.helper(RuntimeHelper::CreateText));
            ctx.push("(\"");
            ctx.push(&super::helpers::escape_js_string(&text.content));
            ctx.push("\")");
        }
        TemplateChildNode::Interpolation(interp) => {
            ctx.use_helper(RuntimeHelper::CreateText);
            ctx.use_helper(RuntimeHelper::ToDisplayString);
            ctx.push(ctx.helper(RuntimeHelper::CreateText));
            ctx.push("(");
            ctx.push(ctx.helper(RuntimeHelper::ToDisplayString));
            ctx.push("(");
            // Generate expression, stripping _ctx. prefix for slot params
            generate_slot_expression(ctx, &interp.content);
            ctx.push("), 1 /* TEXT */)");
        }
        _ => {
            generate_node(ctx, child);
        }
    }
}

/// Generate expression for slot content, stripping _ctx. prefix for slot parameters
fn generate_slot_expression(ctx: &mut CodegenContext, expr: &ExpressionNode<'_>) {
    match expr {
        ExpressionNode::Simple(exp) => {
            if exp.is_static {
                ctx.push("\"");
                ctx.push(&exp.content);
                ctx.push("\"");
            } else {
                // Strip _ctx. prefix for slot parameters
                let content = strip_ctx_prefix_for_slot_params(ctx, &exp.content);
                ctx.push(&content);
            }
        }
        ExpressionNode::Compound(comp) => {
            for child in comp.children.iter() {
                match child {
                    crate::ast::CompoundExpressionChild::Simple(exp) => {
                        if exp.is_static {
                            ctx.push("\"");
                            ctx.push(&exp.content);
                            ctx.push("\"");
                        } else {
                            let content = strip_ctx_prefix_for_slot_params(ctx, &exp.content);
                            ctx.push(&content);
                        }
                    }
                    crate::ast::CompoundExpressionChild::String(s) => {
                        ctx.push(s);
                    }
                    crate::ast::CompoundExpressionChild::Symbol(helper) => {
                        ctx.push(ctx.helper(*helper));
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Strip _ctx. prefix from identifiers that are slot parameters
fn strip_ctx_prefix_for_slot_params(ctx: &CodegenContext, content: &str) -> std::string::String {
    let mut result = content.to_string();
    for param in &ctx.slot_params {
        // Replace _ctx.paramName with paramName
        let mut prefixed = String::with_capacity(5 + param.len());
        prefixed.push_str("_ctx.");
        prefixed.push_str(param);
        result = result.replace(&prefixed, param);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_js_identifier_valid() {
        assert!(is_valid_js_identifier("foo"));
        assert!(is_valid_js_identifier("_bar"));
        assert!(is_valid_js_identifier("$baz"));
        assert!(is_valid_js_identifier("foo123"));
        assert!(is_valid_js_identifier("camelCase"));
        assert!(is_valid_js_identifier("PascalCase"));
    }

    #[test]
    fn test_is_valid_js_identifier_invalid() {
        assert!(!is_valid_js_identifier("123foo")); // starts with number
        assert!(!is_valid_js_identifier("")); // empty
        assert!(!is_valid_js_identifier("foo-bar")); // contains hyphen
        assert!(!is_valid_js_identifier("foo.bar")); // contains dot
        assert!(!is_valid_js_identifier("foo bar")); // contains space
        assert!(!is_valid_js_identifier("item-header")); // hyphenated slot name
    }

    #[test]
    fn test_hyphenated_slot_names_need_quotes() {
        assert!(!is_valid_js_identifier("item-header"));
        assert!(!is_valid_js_identifier("card-body"));
        assert!(!is_valid_js_identifier("main-content"));
        assert!(!is_valid_js_identifier("list-item"));
    }

    #[test]
    fn test_regular_slot_names_are_valid_identifiers() {
        assert!(is_valid_js_identifier("default"));
        assert!(is_valid_js_identifier("header"));
        assert!(is_valid_js_identifier("footer"));
        assert!(is_valid_js_identifier("content"));
    }

    #[test]
    fn test_prefix_slot_defaults() {
        // Default values should get _ctx. prefix
        assert_eq!(
            prefix_slot_defaults("{ item = defaultItem }"),
            "{ item = _ctx.defaultItem }"
        );
        assert_eq!(prefix_slot_defaults("{ count = 0 }"), "{ count = 0 }");
        assert_eq!(
            prefix_slot_defaults("{ name = 'test' }"),
            "{ name = 'test' }"
        );
        // Literals should not be prefixed
        assert_eq!(prefix_slot_defaults("{ x = true }"), "{ x = true }");
        assert_eq!(prefix_slot_defaults("{ x = false }"), "{ x = false }");
        assert_eq!(prefix_slot_defaults("{ x = null }"), "{ x = null }");
        assert_eq!(
            prefix_slot_defaults("{ x = undefined }"),
            "{ x = undefined }"
        );
    }
}
