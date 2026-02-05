//! v-for generation functions.

use crate::ast::*;

use super::children::generate_children;
use super::context::CodegenContext;
use super::element::{generate_vshow_closing, has_vshow_directive};
use super::expression::generate_expression;
use super::helpers::escape_js_string;
use super::node::generate_node;

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

    // Value alias
    if let Some(value) = &for_node.value_alias {
        generate_expression(ctx, value);
    } else {
        ctx.push("_item");
    }

    // Key alias
    if let Some(key) = &for_node.key_alias {
        ctx.push(", ");
        generate_expression(ctx, key);
    }

    // Index alias
    if let Some(index) = &for_node.object_index_alias {
        ctx.push(", ");
        generate_expression(ctx, index);
    }

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
                } else if is_template {
                    // Template: use Fragment
                    ctx.use_helper(RuntimeHelper::CreateElementBlock);
                    ctx.use_helper(RuntimeHelper::Fragment);
                    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
                    ctx.push("(");
                    ctx.push(ctx.helper(RuntimeHelper::Fragment));
                } else {
                    // Regular element
                    ctx.use_helper(RuntimeHelper::CreateElementBlock);
                    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
                    ctx.push("(\"");
                    ctx.push(&el.tag);
                    ctx.push("\"");
                }

                // Props with key and all other props
                generate_for_item_props(ctx, el, key_exp);

                // Children
                if !el.children.is_empty() {
                    ctx.push(", ");
                    if is_template {
                        // Template children are array
                        ctx.push("[");
                        ctx.indent();
                        for (i, child) in el.children.iter().enumerate() {
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
                        generate_children(ctx, &el.children);
                    }
                }

                // Add patch flag
                let has_interpolation = el
                    .children
                    .iter()
                    .any(|c| matches!(c, TemplateChildNode::Interpolation(_)));

                if is_template {
                    ctx.push(", 64 /* STABLE_FRAGMENT */");
                } else if has_interpolation {
                    ctx.push(", 1 /* TEXT */");
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
