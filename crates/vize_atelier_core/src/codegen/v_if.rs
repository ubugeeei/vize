//! v-if generation functions.

use crate::ast::*;

use super::children::generate_children;
use super::context::CodegenContext;
use super::expression::generate_expression;
use super::helpers::{camelize, capitalize_first, escape_js_string, is_valid_js_identifier};
use super::node::generate_node;
use super::props::generate_directive_prop_with_static;
use vize_carton::FxHashSet;

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
        let mut component_name = String::with_capacity(11 + el.tag.len());
        component_name.push_str("_component_");
        component_name.push_str(el.tag.as_str());
        ctx.push(&component_name);
    }

    // Extract static class/style for merging with dynamic bindings
    let (static_class, static_style) = extract_static_class_style(el);
    let has_dyn_class = has_dynamic_class(el);
    let has_dyn_style = has_dynamic_style(el);

    // Check if component has v-bind spread
    let has_vbind_spread = has_vbind_spread(el);
    if has_vbind_spread {
        ctx.use_helper(RuntimeHelper::MergeProps);
        ctx.push(", ");
        ctx.push(ctx.helper(RuntimeHelper::MergeProps));
        ctx.push("(");

        let mut first_merge_arg = true;
        for prop in el.props.iter() {
            if let PropNode::Directive(dir) = prop {
                if dir.name == "bind" && dir.arg.is_none() {
                    if let Some(exp) = &dir.exp {
                        if !first_merge_arg {
                            ctx.push(", ");
                        }
                        generate_expression(ctx, exp);
                        first_merge_arg = false;
                    }
                }
            }
        }

        if !first_merge_arg {
            ctx.push(", ");
        }
        generate_if_branch_props_object(
            ctx,
            el,
            branch,
            branch_index,
            static_class,
            static_style,
            has_dyn_class,
            has_dyn_style,
        );
        ctx.push(")");
    } else {
        ctx.push(", ");
        generate_if_branch_props_object(
            ctx,
            el,
            branch,
            branch_index,
            static_class,
            static_style,
            has_dyn_class,
            has_dyn_style,
        );
    }

    ctx.push("))")
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

    // Generate props with key and all other props (handle v-bind spreads)
    let has_vbind_spread = has_vbind_spread(el);
    if has_vbind_spread {
        ctx.use_helper(RuntimeHelper::MergeProps);
        ctx.push(", ");
        ctx.push(ctx.helper(RuntimeHelper::MergeProps));
        ctx.push("(");

        // Add all v-bind spreads
        let mut first_merge_arg = true;
        for prop in el.props.iter() {
            if let PropNode::Directive(dir) = prop {
                if dir.name == "bind" && dir.arg.is_none() {
                    if let Some(exp) = &dir.exp {
                        if !first_merge_arg {
                            ctx.push(", ");
                        }
                        generate_expression(ctx, exp);
                        first_merge_arg = false;
                    }
                }
            }
        }

        if !first_merge_arg {
            ctx.push(", ");
        }
        generate_if_branch_props_object(
            ctx,
            el,
            branch,
            branch_index,
            static_class,
            static_style,
            has_dyn_class,
            has_dyn_style,
        );
        ctx.push(")");
    } else {
        ctx.push(", ");
        generate_if_branch_props_object(
            ctx,
            el,
            branch,
            branch_index,
            static_class,
            static_style,
            has_dyn_class,
            has_dyn_style,
        );
    }

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

/// Generate props object for v-if branch (with key and other props)
#[allow(clippy::too_many_arguments)]
fn generate_if_branch_props_object(
    ctx: &mut CodegenContext,
    el: &ElementNode<'_>,
    branch: &IfBranchNode<'_>,
    branch_index: usize,
    static_class: Option<&str>,
    static_style: Option<&str>,
    has_dynamic_class: bool,
    has_dynamic_style: bool,
) {
    ctx.push("{");
    ctx.indent();
    ctx.newline();
    ctx.push("key: ");
    generate_if_branch_key(ctx, branch, branch_index);

    let mut seen_events: FxHashSet<String> = FxHashSet::default();

    for prop in el.props.iter() {
        if should_skip_prop_for_if(prop, has_dynamic_class, has_dynamic_style) {
            continue;
        }
        if is_vbind_spread_prop(prop) {
            continue;
        }
        if let PropNode::Directive(dir) = prop {
            if dir.name == "on" {
                if let Some(key) = get_static_event_key(dir) {
                    if !seen_events.insert(key) {
                        continue;
                    }
                }
            }
        }
        ctx.push(",");
        ctx.newline();
        generate_single_prop_for_if(ctx, prop, static_class, static_style);
    }

    // Add scope_id for scoped CSS
    if let Some(ref scope_id) = ctx.options.scope_id.clone() {
        ctx.push(",");
        ctx.newline();
        ctx.push("\"");
        ctx.push(scope_id);
        ctx.push("\": \"\"");
    }

    ctx.deindent();
    ctx.newline();
    ctx.push("}");
}

/// Check if element has v-bind object spread
fn has_vbind_spread(el: &ElementNode<'_>) -> bool {
    el.props.iter().any(|p| is_vbind_spread_prop(p))
}

/// Check if prop is a v-bind object spread (v-bind="obj")
fn is_vbind_spread_prop(prop: &PropNode<'_>) -> bool {
    if let PropNode::Directive(dir) = prop {
        return dir.name == "bind" && dir.arg.is_none();
    }
    false
}

/// Compute static event prop key for dedupe (e.g., onClick, onUpdate:modelValue)
fn get_static_event_key(dir: &DirectiveNode<'_>) -> Option<String> {
    let arg = dir.arg.as_ref()?;
    let ExpressionNode::Simple(exp) = arg else {
        return None;
    };
    if !exp.is_static {
        return None;
    }

    let mut event_name = exp.content.as_str();
    let is_keyboard_event = matches!(event_name, "keydown" | "keyup" | "keypress");

    let mut event_option_modifiers: Vec<&str> = Vec::new();
    let mut system_modifiers: Vec<&str> = Vec::new();

    for modifier in dir.modifiers.iter() {
        let mod_name = modifier.content.as_str();
        match mod_name {
            "capture" | "once" | "passive" => {
                event_option_modifiers.push(mod_name);
            }
            "left" | "right" => {
                if !is_keyboard_event {
                    system_modifiers.push(mod_name);
                }
            }
            "middle" => {
                system_modifiers.push(mod_name);
            }
            _ => {}
        }
    }

    let has_right_modifier = system_modifiers.contains(&"right");
    let has_middle_modifier = system_modifiers.contains(&"middle");

    if event_name == "click" && has_right_modifier {
        event_name = "contextmenu";
    } else if event_name == "click" && has_middle_modifier {
        event_name = "mouseup";
    }

    let mut key = if event_name.contains(':') {
        let parts: Vec<&str> = event_name.splitn(2, ':').collect();
        if parts.len() == 2 {
            let first_part = camelize(parts[0]);
            let mut name = String::from("on");
            if let Some(first) = first_part.chars().next() {
                name.push_str(&first.to_uppercase().to_string());
                name.push_str(&first_part[first.len_utf8()..]);
            }
            name.push(':');
            name.push_str(parts[1]);
            name
        } else {
            String::from(event_name)
        }
    } else {
        let camelized = camelize(event_name);
        let mut name = String::from("on");
        if let Some(first) = camelized.chars().next() {
            name.push_str(&first.to_uppercase().to_string());
            name.push_str(&camelized[first.len_utf8()..]);
        }
        name
    };

    for opt_mod in &event_option_modifiers {
        key.push_str(&capitalize_first(opt_mod));
    }

    Some(key)
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

// Note: v-if directive behavior is tested via SFC snapshot tests
// in tests/fixtures/sfc/patches.toml. Unit tests for AST-based functions
// require bumpalo allocation which adds complexity without significant benefit.

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
