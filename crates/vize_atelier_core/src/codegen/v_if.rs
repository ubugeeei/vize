//! v-if generation functions.

use crate::ast::*;

use super::children::generate_children;
use super::context::CodegenContext;
use super::expression::generate_expression;
use super::helpers::{escape_js_string, is_valid_js_identifier};
use super::node::generate_node;
use super::props::generate_directive_prop_with_static;

/// Generate if node
pub fn generate_if(ctx: &mut CodegenContext, if_node: &IfNode<'_>) {
    ctx.use_helper(RuntimeHelper::OpenBlock);

    // Vue always imports createCommentVNode for v-if nodes
    ctx.use_helper(RuntimeHelper::CreateComment);

    for (i, branch) in if_node.branches.iter().enumerate() {
        if let Some(condition) = &branch.condition {
            if i == 0 {
                // First branch: output condition with parentheses
                ctx.push("(");
                generate_expression(ctx, condition);
                ctx.push(")");
                ctx.indent();
                ctx.newline();
                ctx.push("? ");
            } else {
                // Subsequent branches (else-if)
                ctx.newline();
                ctx.push(": (");
                generate_expression(ctx, condition);
                ctx.push(")");
                ctx.indent();
                ctx.newline();
                ctx.push("? ");
            }
        } else {
            // Else branch (no condition)
            ctx.newline();
            ctx.push(": ");
        }

        // Generate branch content based on children
        generate_if_branch(ctx, branch, i);

        if branch.condition.is_some() && i > 0 {
            ctx.deindent();
        }
    }

    // Else branch (comment node) - only if all branches have conditions
    if if_node.branches.iter().all(|b| b.condition.is_some()) {
        ctx.newline();
        ctx.push(": ");
        ctx.push(ctx.helper(RuntimeHelper::CreateComment));
        ctx.push("(\"v-if\", true)");
    }

    ctx.deindent();
}

/// Generate a single if branch
pub fn generate_if_branch(
    ctx: &mut CodegenContext,
    branch: &IfBranchNode<'_>,
    branch_index: usize,
) {
    // Single child optimization
    if branch.children.len() == 1 {
        match &branch.children[0] {
            TemplateChildNode::Element(el) => {
                // Check if it's a template element - treat as fragment
                if el.tag_type == ElementType::Template {
                    // Template with single child -> unwrap to single element
                    if el.children.len() == 1 {
                        if let TemplateChildNode::Element(inner) = &el.children[0] {
                            // Check if inner element is a component
                            if inner.tag_type == ElementType::Component {
                                generate_if_branch_component(ctx, inner, branch, branch_index);
                            } else {
                                generate_if_branch_element(ctx, inner, branch, branch_index);
                            }
                            return;
                        }
                    }
                    // Template with multiple children -> fragment
                    generate_if_branch_template_fragment(ctx, &el.children, branch, branch_index);
                } else if el.tag_type == ElementType::Component {
                    // Component
                    generate_if_branch_component(ctx, el, branch, branch_index);
                } else {
                    // Regular element
                    generate_if_branch_element(ctx, el, branch, branch_index);
                }
            }
            _ => {
                // Other node types - wrap in fragment
                generate_if_branch_fragment(ctx, branch, branch_index);
            }
        }
    } else {
        // Multiple children - wrap in fragment
        generate_if_branch_fragment(ctx, branch, branch_index);
    }
}

/// Generate component for if branch
pub fn generate_if_branch_component(
    ctx: &mut CodegenContext,
    el: &ElementNode<'_>,
    branch: &IfBranchNode<'_>,
    branch_index: usize,
) {
    ctx.use_helper(RuntimeHelper::CreateBlock);
    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::OpenBlock));
    ctx.push("(), ");
    ctx.push(ctx.helper(RuntimeHelper::CreateBlock));
    ctx.push("(");
    // Generate component name
    // In inline mode, components are directly in scope (imported at module level)
    // In function mode, use $setup.ComponentName to access setup bindings
    if ctx.is_component_in_bindings(&el.tag) {
        if !ctx.options.inline {
            ctx.push("$setup.");
        }
        ctx.push(el.tag.as_str());
    } else {
        let component_name = format!("_component_{}", el.tag.as_str());
        ctx.push(&component_name);
    }

    // Check if component has props
    let has_other = has_other_props_for_if(el);
    ctx.push(", {");
    ctx.indent();
    ctx.newline();
    ctx.push("key: ");
    generate_if_branch_key(ctx, branch, branch_index);

    // Extract static class/style for merging with dynamic bindings
    let (static_class, static_style) = extract_static_class_style(el);
    let has_dyn_class = has_dynamic_class(el);
    let has_dyn_style = has_dynamic_style(el);

    // Add other props if any
    if has_other {
        for prop in el.props.iter() {
            if should_skip_prop_for_if(prop, has_dyn_class, has_dyn_style) {
                continue;
            }
            ctx.push(",");
            ctx.newline();
            generate_single_prop_for_if(ctx, prop, static_class, static_style);
        }
    }

    // Add scope_id for scoped CSS
    if let Some(ref scope_id) = ctx.options.scope_id.clone() {
        ctx.push(",");
        ctx.newline();
        ctx.push("\"");
        ctx.push(&scope_id);
        ctx.push("\": \"\"");
    }

    ctx.deindent();
    ctx.newline();
    ctx.push("}))")
}

/// Check if element has props besides the key (for v-if branch elements)
fn has_other_props_for_if(el: &ElementNode<'_>) -> bool {
    el.props.iter().any(|p| match p {
        PropNode::Attribute(_) => true,
        PropNode::Directive(dir) => {
            // Skip key binding (handled separately)
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if arg.content == "key" {
                        return false;
                    }
                }
            }
            // Skip v-if/v-else-if/v-else directives (handled by parent)
            if matches!(dir.name.as_str(), "if" | "else-if" | "else") {
                return false;
            }
            true
        }
    })
}

/// Check if prop should be skipped for v-if branch element
fn should_skip_prop_for_if(
    p: &PropNode<'_>,
    has_dynamic_class: bool,
    has_dynamic_style: bool,
) -> bool {
    match p {
        PropNode::Attribute(attr) => {
            // Skip static class if there's a dynamic :class (will be merged)
            if attr.name == "class" && has_dynamic_class {
                return true;
            }
            // Skip static style if there's a dynamic :style (will be merged)
            if attr.name == "style" && has_dynamic_style {
                return true;
            }
            false
        }
        PropNode::Directive(dir) => {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if arg.content == "key" {
                        return true;
                    }
                }
            }
            // Skip v-if/v-else-if/v-else directives
            if matches!(dir.name.as_str(), "if" | "else-if" | "else") {
                return true;
            }
            false
        }
    }
}

/// Extract static class and style values from element props
fn extract_static_class_style<'a>(el: &'a ElementNode<'_>) -> (Option<&'a str>, Option<&'a str>) {
    let mut static_class = None;
    let mut static_style = None;
    for prop in el.props.iter() {
        if let PropNode::Attribute(attr) = prop {
            if attr.name == "class" {
                if let Some(val) = &attr.value {
                    static_class = Some(val.content.as_str());
                }
            } else if attr.name == "style" {
                if let Some(val) = &attr.value {
                    static_style = Some(val.content.as_str());
                }
            }
        }
    }
    (static_class, static_style)
}

/// Check if element has dynamic :class binding
fn has_dynamic_class(el: &ElementNode<'_>) -> bool {
    el.props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    return arg.content == "class";
                }
            }
        }
        false
    })
}

/// Check if element has dynamic :style binding
fn has_dynamic_style(el: &ElementNode<'_>) -> bool {
    el.props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    return arg.content == "style";
                }
            }
        }
        false
    })
}

/// Generate a single prop for v-if branch element
fn generate_single_prop_for_if(
    ctx: &mut CodegenContext,
    prop: &PropNode<'_>,
    static_class: Option<&str>,
    static_style: Option<&str>,
) {
    match prop {
        PropNode::Attribute(attr) => {
            let needs_quotes = !is_valid_js_identifier(&attr.name);
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
                ctx.push(&escape_js_string(value.content.as_str()));
                ctx.push("\"");
            } else {
                ctx.push("\"\"");
            }
        }
        PropNode::Directive(dir) => {
            generate_directive_prop_with_static(ctx, dir, static_class, static_style);
        }
    }
}

/// Generate element for if branch
pub fn generate_if_branch_element(
    ctx: &mut CodegenContext,
    el: &ElementNode<'_>,
    branch: &IfBranchNode<'_>,
    branch_index: usize,
) {
    ctx.use_helper(RuntimeHelper::CreateElementBlock);
    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::OpenBlock));
    ctx.push("(), ");
    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
    ctx.push("(\"");
    ctx.push(el.tag.as_str());
    ctx.push("\"");

    // Extract static class/style for merging with dynamic bindings
    let (static_class, static_style) = extract_static_class_style(el);
    let has_dyn_class = has_dynamic_class(el);
    let has_dyn_style = has_dynamic_style(el);

    // Generate props with key and all other props
    let has_other = has_other_props_for_if(el);
    ctx.push(", {");
    ctx.indent();
    ctx.newline();
    ctx.push("key: ");
    generate_if_branch_key(ctx, branch, branch_index);

    // Add other props
    if has_other {
        for prop in el.props.iter() {
            if should_skip_prop_for_if(prop, has_dyn_class, has_dyn_style) {
                continue;
            }
            ctx.push(",");
            ctx.newline();
            generate_single_prop_for_if(ctx, prop, static_class, static_style);
        }
    }

    // Add scope_id for scoped CSS
    if let Some(ref scope_id) = ctx.options.scope_id.clone() {
        ctx.push(",");
        ctx.newline();
        ctx.push("\"");
        ctx.push(&scope_id);
        ctx.push("\": \"\"");
    }

    ctx.deindent();
    ctx.newline();
    ctx.push("}");

    // Generate children if any
    if !el.children.is_empty() {
        ctx.push(", ");
        if el.children.len() == 1 {
            if let TemplateChildNode::Text(text) = &el.children[0] {
                ctx.push("\"");
                ctx.push(&escape_js_string(text.content.as_str()));
                ctx.push("\"");
            } else {
                generate_if_branch_children(ctx, &el.children);
            }
        } else {
            generate_if_branch_children(ctx, &el.children);
        }
    }

    ctx.push("))");
}

/// Generate template fragment for if branch (multiple children from template)
pub fn generate_if_branch_template_fragment(
    ctx: &mut CodegenContext,
    children: &[TemplateChildNode<'_>],
    branch: &IfBranchNode<'_>,
    branch_index: usize,
) {
    ctx.use_helper(RuntimeHelper::CreateElementBlock);
    ctx.use_helper(RuntimeHelper::Fragment);
    ctx.use_helper(RuntimeHelper::CreateElementVNode);
    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::OpenBlock));
    ctx.push("(), ");
    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::Fragment));
    ctx.push(", { key: ");
    generate_if_branch_key(ctx, branch, branch_index);
    ctx.push(" }, [");
    ctx.indent();
    for (i, child) in children.iter().enumerate() {
        if i > 0 {
            ctx.push(",");
        }
        ctx.newline();
        generate_node(ctx, child);
    }
    ctx.deindent();
    ctx.newline();
    ctx.push("], 64 /* STABLE_FRAGMENT */))");
}

/// Generate key for if branch
pub fn generate_if_branch_key(
    ctx: &mut CodegenContext,
    branch: &IfBranchNode<'_>,
    branch_index: usize,
) {
    // Check if branch has a user-provided key
    if let Some(ref user_key) = branch.user_key {
        match user_key {
            PropNode::Attribute(attr) => {
                // Static key attribute
                if let Some(ref value) = attr.value {
                    ctx.push("\"");
                    ctx.push(&escape_js_string(value.content.as_str()));
                    ctx.push("\"");
                } else {
                    ctx.push(&branch_index.to_string());
                }
            }
            PropNode::Directive(dir) => {
                // Dynamic :key binding
                if let Some(ref exp) = dir.exp {
                    generate_expression(ctx, exp);
                } else {
                    ctx.push(&branch_index.to_string());
                }
            }
        }
    } else {
        ctx.push(&branch_index.to_string());
    }
}

/// Generate fragment wrapper for if branch with multiple children
pub fn generate_if_branch_fragment(
    ctx: &mut CodegenContext,
    branch: &IfBranchNode<'_>,
    branch_index: usize,
) {
    ctx.use_helper(RuntimeHelper::CreateElementBlock);
    ctx.use_helper(RuntimeHelper::Fragment);
    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::OpenBlock));
    ctx.push("(), ");
    ctx.push(ctx.helper(RuntimeHelper::CreateElementBlock));
    ctx.push("(");
    ctx.push(ctx.helper(RuntimeHelper::Fragment));
    ctx.push(", { key: ");
    generate_if_branch_key(ctx, branch, branch_index);
    ctx.push(" }, ");
    generate_children(ctx, &branch.children);
    ctx.push(", 64 /* STABLE_FRAGMENT */))");
}

/// Generate children for if branch element
pub fn generate_if_branch_children(ctx: &mut CodegenContext, children: &[TemplateChildNode<'_>]) {
    if children.is_empty() {
        return;
    }

    // Check if all children are simple (text or interpolation)
    let has_only_text_or_interpolation = children.iter().all(|c| {
        matches!(
            c,
            TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
        )
    });

    if has_only_text_or_interpolation && children.len() == 1 {
        match &children[0] {
            TemplateChildNode::Interpolation(interp) => {
                ctx.use_helper(RuntimeHelper::ToDisplayString);
                ctx.push(ctx.helper(RuntimeHelper::ToDisplayString));
                ctx.push("(");
                generate_expression(ctx, &interp.content);
                ctx.push("), 1 /* TEXT */");
            }
            TemplateChildNode::Text(text) => {
                ctx.push("\"");
                ctx.push(&escape_js_string(text.content.as_str()));
                ctx.push("\"");
            }
            _ => {}
        }
    } else {
        // Complex children - use array
        ctx.push("[");
        ctx.indent();
        for (i, child) in children.iter().enumerate() {
            if i > 0 {
                ctx.push(",");
            }
            ctx.newline();
            generate_node(ctx, child);
        }
        ctx.deindent();
        ctx.newline();
        ctx.push("]");
    }
}
