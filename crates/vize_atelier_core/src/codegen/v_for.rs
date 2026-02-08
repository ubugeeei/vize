//! v-for generation functions.

use crate::ast::*;

use super::children::generate_children;
use super::context::CodegenContext;
use super::element::{generate_vshow_closing, has_vshow_directive};
use super::expression::generate_expression;
use super::helpers::escape_js_string;
use super::node::generate_node;
use super::patch_flag::{calculate_element_patch_info, patch_flag_name};

/// Extract parameter names from a v-for callback expression.
/// Handles simple identifiers ("item"), destructuring patterns ("{ id, name }"),
/// nested destructure ("{ user: { name } }"), rest elements ("{ id, ...rest }"),
/// and array destructuring ("[first, second]").
fn extract_for_params(expr: &ExpressionNode<'_>, params: &mut Vec<String>) {
    let content = match expr {
        ExpressionNode::Simple(exp) => exp.content.as_str(),
        _ => return,
    };
    extract_destructure_params(content.trim(), params);
}

/// Recursively extract parameter names from a destructuring pattern string.
pub(super) fn extract_destructure_params(trimmed: &str, params: &mut Vec<String>) {
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let inner = &trimmed[1..trimmed.len() - 1];
        // Split by commas at the top level (respecting nested braces/brackets)
        for part in split_top_level(inner) {
            let part = part.trim();
            // Handle rest element: ...rest
            if let Some(rest) = part.strip_prefix("...") {
                let rest = rest.trim();
                if !rest.is_empty() && is_valid_ident(rest) {
                    params.push(rest.to_string());
                }
                continue;
            }
            // Handle default values: "item = default" — take name before =
            if let Some(eq_pos) = part.find('=') {
                let name = part[..eq_pos].trim();
                if !name.is_empty() && is_valid_ident(name) {
                    params.push(name.to_string());
                }
                continue;
            }
            // Handle renaming/nested: "original: value"
            if let Some(colon_pos) = part.find(':') {
                let value = part[colon_pos + 1..].trim();
                // Value might be another destructure pattern or a simple identifier
                if value.starts_with('{') || value.starts_with('[') {
                    extract_destructure_params(value, params);
                } else if is_valid_ident(value) {
                    params.push(value.to_string());
                }
                continue;
            }
            // Simple identifier
            if !part.is_empty() && is_valid_ident(part) {
                params.push(part.to_string());
            }
        }
    } else if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len() - 1];
        for part in split_top_level(inner) {
            let part = part.trim();
            if let Some(rest) = part.strip_prefix("...") {
                let rest = rest.trim();
                if !rest.is_empty() && is_valid_ident(rest) {
                    params.push(rest.to_string());
                }
            } else if part.starts_with('{') || part.starts_with('[') {
                extract_destructure_params(part, params);
            } else if !part.is_empty() && is_valid_ident(part) {
                params.push(part.to_string());
            }
        }
    } else if is_valid_ident(trimmed) {
        params.push(trimmed.to_string());
    }
}

/// Split a string by commas at the top level, respecting nested braces and brackets.
pub(super) fn split_top_level(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, b) in s.bytes().enumerate() {
        match b {
            b'{' | b'[' | b'(' => depth += 1,
            b'}' | b']' | b')' => depth -= 1,
            b',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// Check if a string is a valid JS identifier
pub(super) fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// Check if content is a numeric literal (for v-for range)
fn is_numeric_content(content: &str) -> bool {
    !content.is_empty() && content.chars().all(|c| c.is_ascii_digit())
}

/// Check if source is a numeric literal (for v-for range)
pub fn is_numeric_source(source: &ExpressionNode<'_>) -> bool {
    if let ExpressionNode::Simple(exp) = source {
        is_numeric_content(exp.content.as_str())
    } else {
        false
    }
}

/// Check if element has a :key binding
pub fn get_element_key<'a, 'b>(el: &'b ElementNode<'a>) -> Option<&'b ExpressionNode<'a>>
where
    'a: 'b,
{
    for prop in &el.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if arg.content == "key" {
                        return dir.exp.as_ref();
                    }
                }
            }
        }
    }
    None
}

/// Generate for node
pub fn generate_for(ctx: &mut CodegenContext, for_node: &ForNode<'_>) {
    ctx.use_helper(RuntimeHelper::OpenBlock);
    ctx.use_helper(RuntimeHelper::CreateElementBlock);
    ctx.use_helper(RuntimeHelper::Fragment);
    ctx.use_helper(RuntimeHelper::RenderList);

    // Determine if this is a numeric range (stable) or dynamic list
    let is_stable = is_numeric_source(&for_node.source);

    // Check if children have keys
    let has_key = for_node.children.iter().any(|child| {
        if let TemplateChildNode::Element(el) = child {
            get_element_key(el).is_some()
        } else {
            false
        }
    });

    // Fragment type: 64 = STABLE, 128 = KEYED, 256 = UNKEYED
    let fragment_flag = if is_stable {
        64 // STABLE_FRAGMENT
    } else if has_key {
        128 // KEYED_FRAGMENT
    } else {
        256 // UNKEYED_FRAGMENT
    };

    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::OpenBlock));
    if is_stable {
        ctx.push("(), ");
    } else {
        ctx.push("(true), ");
    }
    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::Fragment));
    ctx.push(", null, ");
    ctx.push(ctx.helper(RuntimeHelper::RenderList));
    ctx.push("(");
    generate_expression(ctx, &for_node.source);
    ctx.push(", (");

    // Collect callback parameter names for scope registration
    let mut callback_params: Vec<String> = Vec::new();

    // Value alias
    if let Some(value) = &for_node.value_alias {
        generate_expression(ctx, value);
        extract_for_params(value, &mut callback_params);
    } else {
        ctx.push("_item");
    }

    // Key alias
    if let Some(key) = &for_node.key_alias {
        ctx.push(", ");
        generate_expression(ctx, key);
        extract_for_params(key, &mut callback_params);
    }

    // Index alias
    if let Some(index) = &for_node.object_index_alias {
        ctx.push(", ");
        generate_expression(ctx, index);
        extract_for_params(index, &mut callback_params);
    }

    // Register callback params so they don't get _ctx. prefix
    ctx.add_slot_params(&callback_params);

    ctx.push(") => {");
    ctx.indent();
    ctx.newline();
    ctx.push("return ");

    // Generate child as block (not regular node)
    if for_node.children.len() == 1 {
        generate_for_item(ctx, &for_node.children[0], is_stable);
    } else {
        generate_children(ctx, &for_node.children);
    }

    // Unregister callback params
    ctx.remove_slot_params(&callback_params);

    ctx.deindent();
    ctx.newline();
    // Close with fragment flag
    let flag_name = match fragment_flag {
        64 => "STABLE_FRAGMENT",
        128 => "KEYED_FRAGMENT",
        256 => "UNKEYED_FRAGMENT",
        _ => "FRAGMENT",
    };
    ctx.push("}), ");
    ctx.push(&fragment_flag.to_string());
    ctx.push(" /* ");
    ctx.push(flag_name);
    ctx.push(" */))");
}

/// Check if element has props besides the key
fn has_other_props(el: &ElementNode<'_>) -> bool {
    el.props.iter().any(|p| match p {
        PropNode::Attribute(_) => true,
        PropNode::Directive(dir) => {
            // Skip key binding (already handled separately)
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if arg.content == "key" {
                        return false;
                    }
                }
            }
            // Skip v-for directive (handled by parent)
            if dir.name == "for" {
                return false;
            }
            true
        }
    })
}

/// Check if prop should be skipped for v-for item (key binding and v-for directive)
fn should_skip_prop(p: &PropNode<'_>) -> bool {
    if let PropNode::Directive(dir) = p {
        if dir.name == "bind" {
            if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                if arg.content == "key" {
                    return true;
                }
            }
        }
        // Skip v-for directive
        if dir.name == "for" {
            return true;
        }
        // Skip custom/unsupported directives (handled via withDirectives)
        if !super::props::is_supported_directive(dir) {
            return true;
        }
    }
    false
}

/// Generate props for v-for item, including key and all other props
fn generate_for_item_props(
    ctx: &mut CodegenContext,
    el: &ElementNode<'_>,
    key_exp: Option<&ExpressionNode<'_>>,
) {
    let has_other = has_other_props(el);
    let scope_id = ctx.options.scope_id.clone();

    if key_exp.is_none() && !has_other && scope_id.is_none() {
        ctx.push(", null");
        return;
    }

    ctx.push(", ");

    if !has_other {
        // Only key (and optionally scope_id), no other props
        if let Some(key) = key_exp {
            ctx.push("{ key: ");
            generate_expression(ctx, key);
            if let Some(ref sid) = scope_id {
                ctx.push(", \"");
                ctx.push(sid);
                ctx.push("\": \"\"");
            }
            ctx.push(" }");
        } else if let Some(ref sid) = scope_id {
            // No key, no other props, but has scope_id
            ctx.push("{ \"");
            ctx.push(sid);
            ctx.push("\": \"\" }");
        }
        return;
    }

    if let Some(key) = key_exp {
        // Merge key with other props - generate as object with key first
        ctx.push("{");
        ctx.indent();
        ctx.newline();
        ctx.push("key: ");
        generate_expression(ctx, key);

        // Add other props inline (skipping key binding and v-for)
        for prop in el.props.iter() {
            if should_skip_prop(prop) {
                continue;
            }
            ctx.push(",");
            ctx.newline();
            generate_single_prop(ctx, prop);
        }

        // Add scope_id for scoped CSS
        if let Some(ref sid) = scope_id {
            ctx.push(",");
            ctx.newline();
            ctx.push("\"");
            ctx.push(sid);
            ctx.push("\": \"\"");
        }

        ctx.deindent();
        ctx.newline();
        ctx.push("}");
    } else {
        // No key, generate props directly (skipping v-for directive)
        ctx.push("{");
        let mut first = true;
        for prop in el.props.iter() {
            if should_skip_prop(prop) {
                continue;
            }
            if !first {
                ctx.push(",");
            }
            ctx.push(" ");
            generate_single_prop(ctx, prop);
            first = false;
        }

        // Add scope_id for scoped CSS
        if let Some(ref sid) = scope_id {
            if !first {
                ctx.push(",");
            }
            ctx.push(" \"");
            ctx.push(sid);
            ctx.push("\": \"\"");
        }

        ctx.push(" }");
    }
}

/// Generate a single prop (attribute or directive)
fn generate_single_prop(ctx: &mut CodegenContext, prop: &PropNode<'_>) {
    match prop {
        PropNode::Attribute(attr) => {
            // Keys need quotes if they contain special characters (like hyphens)
            let needs_quotes = !super::helpers::is_valid_js_identifier(&attr.name);
            if needs_quotes {
                ctx.push("\"");
            }
            ctx.push(&attr.name);
            if needs_quotes {
                ctx.push("\"");
            }
            ctx.push(": ");
            if let Some(value) = &attr.value {
                ctx.push("\"");
                ctx.push(&escape_js_string(&value.content));
                ctx.push("\"");
            } else {
                ctx.push("\"\"");
            }
        }
        PropNode::Directive(dir) => {
            super::props::generate_directive_prop_with_static(ctx, dir, None, None);
        }
    }
}

/// Generate item for v-for (as block, not regular vnode)
pub fn generate_for_item(ctx: &mut CodegenContext, node: &TemplateChildNode<'_>, is_stable: bool) {
    match node {
        TemplateChildNode::Element(el) => {
            let key_exp = get_element_key(el);
            let is_template = el.tag_type == ElementType::Template;
            let is_component = el.tag_type == ElementType::Component;

            // Check for v-show directive
            let has_vshow = has_vshow_directive(el);
            if has_vshow {
                ctx.use_helper(RuntimeHelper::WithDirectives);
                ctx.use_helper(RuntimeHelper::VShow);
                ctx.push(ctx.helper(RuntimeHelper::WithDirectives));
                ctx.push("(");
            }

            if is_stable {
                // Stable fragment: use createElementVNode without block wrapper
                ctx.use_helper(RuntimeHelper::CreateElementVNode);
                ctx.push(ctx.helper(RuntimeHelper::CreateElementVNode));
                ctx.push("(\"");
                ctx.push(&el.tag);
                ctx.push("\"");

                // Props with key and all other props
                generate_for_item_props(ctx, el, key_exp);

                // Children
                if !el.children.is_empty() {
                    ctx.push(", ");
                    generate_children(ctx, &el.children);
                }

                // Add TEXT patch flag if has interpolation
                let has_interpolation = el
                    .children
                    .iter()
                    .any(|c| matches!(c, TemplateChildNode::Interpolation(_)));
                if has_interpolation {
                    ctx.push(", 1 /* TEXT */");
                }

                ctx.push(")");

                // Close withDirectives for v-show
                if has_vshow {
                    generate_vshow_closing(ctx, el);
                }
            } else {
                // Dynamic list: wrap in block
                ctx.use_helper(RuntimeHelper::OpenBlock);
                ctx.push("(");
                ctx.push(ctx.helper(RuntimeHelper::OpenBlock));
                ctx.push("(), ");

                // Template with single child element optimization:
                // unwrap the template and generate the child directly as a block
                let unwrapped_child: Option<&ElementNode<'_>> =
                    if is_template && el.children.len() == 1 {
                        if let TemplateChildNode::Element(ref child_el) = el.children[0] {
                            if child_el.tag_type == ElementType::Element {
                                Some(child_el)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                let gen_is_template = is_template && unwrapped_child.is_none();

                if is_component {
                    // Component: use createBlock
                    ctx.use_helper(RuntimeHelper::CreateBlock);
                    ctx.push(ctx.helper(RuntimeHelper::CreateBlock));
                    ctx.push("(");
                    // In inline mode, components are directly in scope (imported at module level)
                    // In function mode, use $setup.ComponentName to access setup bindings
                    if ctx.is_component_in_bindings(&el.tag) {
                        if !ctx.options.inline {
                            ctx.push("$setup.");
                        }
                        ctx.push(&el.tag);
                    } else {
                        ctx.push("_component_");
                        ctx.push(&el.tag.replace('-', "_"));
                    }
                } else if gen_is_template {
                    // Template with multiple children: use Fragment
                    ctx.use_helper(RuntimeHelper::CreateElementBlock);
                    ctx.use_helper(RuntimeHelper::Fragment);
                    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
                    ctx.push("(");
                    ctx.push(ctx.helper(RuntimeHelper::Fragment));
                } else if let Some(child_el) = unwrapped_child {
                    // Template with single child: unwrap to child element
                    ctx.use_helper(RuntimeHelper::CreateElementBlock);
                    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
                    ctx.push("(\"");
                    ctx.push(&child_el.tag);
                    ctx.push("\"");
                } else {
                    // Regular element
                    ctx.use_helper(RuntimeHelper::CreateElementBlock);
                    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
                    ctx.push("(\"");
                    ctx.push(&el.tag);
                    ctx.push("\"");
                }

                // Props with key and all other props
                // For unwrapped template child, use child's props with template's key
                let props_el = unwrapped_child.unwrap_or(el);
                generate_for_item_props(ctx, props_el, key_exp);

                // Children
                let children_el = unwrapped_child.unwrap_or(el);
                if !children_el.children.is_empty() {
                    ctx.push(", ");
                    if gen_is_template {
                        // Template children are array
                        ctx.push("[");
                        ctx.indent();
                        for (i, child) in children_el.children.iter().enumerate() {
                            if i > 0 {
                                ctx.push(",");
                            }
                            ctx.newline();
                            generate_node(ctx, child);
                        }
                        ctx.deindent();
                        ctx.newline();
                        ctx.push("]");
                    } else {
                        generate_children(ctx, &children_el.children);
                    }
                }

                // Add patch flag
                if is_component {
                    // For components inside v-for, use full patch flag calculation
                    let (patch_flag, dynamic_props) = calculate_element_patch_info(
                        el,
                        ctx.options.binding_metadata.as_ref(),
                        ctx.options.cache_handlers,
                    );
                    // If no children were emitted but we have patch info, emit null for children
                    if el.children.is_empty() && (patch_flag.is_some() || dynamic_props.is_some()) {
                        ctx.push(", null");
                    }
                    if let Some(flag) = patch_flag {
                        ctx.push(", ");
                        ctx.push(&flag.to_string());
                        ctx.push(" /* ");
                        ctx.push(&patch_flag_name(flag));
                        ctx.push(" */");
                    }
                    if let Some(props) = dynamic_props {
                        ctx.push(", [");
                        for (i, prop) in props.iter().enumerate() {
                            if i > 0 {
                                ctx.push(", ");
                            }
                            ctx.push("\"");
                            ctx.push(prop);
                            ctx.push("\"");
                        }
                        ctx.push("]");
                    }
                } else if gen_is_template {
                    ctx.push(", 64 /* STABLE_FRAGMENT */");
                } else {
                    // For regular elements (and unwrapped template children), use full patch flag calculation
                    let flag_el = unwrapped_child.unwrap_or(el);
                    let (patch_flag, dynamic_props) = calculate_element_patch_info(
                        flag_el,
                        ctx.options.binding_metadata.as_ref(),
                        ctx.options.cache_handlers,
                    );
                    if let Some(flag) = patch_flag {
                        ctx.push(", ");
                        ctx.push(&flag.to_string());
                        ctx.push(" /* ");
                        ctx.push(&patch_flag_name(flag));
                        ctx.push(" */");
                    }
                    if let Some(props) = dynamic_props {
                        ctx.push(", [");
                        for (i, prop) in props.iter().enumerate() {
                            if i > 0 {
                                ctx.push(", ");
                            }
                            ctx.push("\"");
                            ctx.push(prop);
                            ctx.push("\"");
                        }
                        ctx.push("]");
                    }
                }

                ctx.push("))");

                // Close withDirectives for v-show
                if has_vshow {
                    generate_vshow_closing(ctx, el);
                }
            }
        }
        _ => generate_node(ctx, node),
    }
}

#[cfg(test)]
mod tests {
    use super::is_numeric_content;

    /// Test numeric source detection for v-for range expressions.
    /// This tests the actual `is_numeric_content` helper function.
    #[test]
    fn test_is_numeric_content() {
        // Valid numeric literals (v-for="n in 10")
        assert!(is_numeric_content("10"));
        assert!(is_numeric_content("100"));
        assert!(is_numeric_content("0"));
        assert!(is_numeric_content("12345"));

        // Invalid: variable names
        assert!(!is_numeric_content("items"));
        assert!(!is_numeric_content("arr"));

        // Invalid: expressions
        assert!(!is_numeric_content("arr.length"));
        assert!(!is_numeric_content("10 + 5"));

        // Invalid: floating point
        assert!(!is_numeric_content("10.5"));

        // Invalid: empty string
        assert!(!is_numeric_content(""));
    }
}

// Note: Directive skipping behavior (v-for with custom directives, :key handling)
// is tested via SFC snapshot tests in tests/fixtures/sfc/patches.toml.
