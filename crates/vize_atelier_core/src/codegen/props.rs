//! Props generation functions.

use crate::ast::*;

use super::context::CodegenContext;
use super::expression::{generate_event_handler, generate_expression, generate_simple_expression};
use super::helpers::{camelize, capitalize_first, escape_js_string, is_valid_js_identifier};

/// Check if there's a v-bind without argument (object spread)
fn has_vbind_object(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            return dir.name == "bind" && dir.arg.is_none();
        }
        false
    })
}

/// Check if there's a v-on without argument (event object spread)
fn has_von_object(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            return dir.name == "on" && dir.arg.is_none();
        }
        false
    })
}

/// Check if there are other props besides v-bind/v-on object spreads
fn has_other_props(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| match p {
        PropNode::Attribute(_) => true,
        PropNode::Directive(dir) => {
            // v-bind without arg is the object spread, not a regular prop
            if dir.name == "bind" && dir.arg.is_none() {
                return false;
            }
            // v-on without arg is the event object spread, not a regular prop
            if dir.name == "on" && dir.arg.is_none() {
                return false;
            }
            is_supported_directive(dir)
        }
    })
}

/// Generate the v-bind object expression
fn generate_vbind_object_exp(ctx: &mut CodegenContext, props: &[PropNode<'_>]) {
    for p in props {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" && dir.arg.is_none() {
                if let Some(exp) = &dir.exp {
                    generate_expression(ctx, exp);
                    return;
                }
            }
        }
    }
}

/// Generate the v-on object expression wrapped with toHandlers
fn generate_von_object_exp(ctx: &mut CodegenContext, props: &[PropNode<'_>]) {
    ctx.use_helper(RuntimeHelper::ToHandlers);
    ctx.push(ctx.helper(RuntimeHelper::ToHandlers));
    ctx.push("(");
    for p in props {
        if let PropNode::Directive(dir) = p {
            if dir.name == "on" && dir.arg.is_none() {
                if let Some(exp) = &dir.exp {
                    generate_expression(ctx, exp);
                    ctx.push(", true"); // true for handlerOnly
                    break;
                }
            }
        }
    }
    ctx.push(")");
}

/// Generate props object
pub fn generate_props(ctx: &mut CodegenContext, props: &[PropNode<'_>]) {
    // Clone scope_id to avoid borrow checker issues
    let scope_id = ctx.options.scope_id.clone();

    // If no props but we have scope_id, generate object with just scope_id
    if props.is_empty() {
        if let Some(ref sid) = scope_id {
            ctx.push("{ \"");
            ctx.push(sid);
            ctx.push("\": \"\" }");
        } else {
            ctx.push("null");
        }
        return;
    }

    // Check for v-bind object (v-bind="attrs") and v-on object (v-on="handlers")
    let has_vbind_obj = has_vbind_object(props);
    let has_von_obj = has_von_object(props);
    let has_other = has_other_props(props);

    // Handle cases with object spreads (v-bind="obj" or v-on="obj")
    if has_vbind_obj || has_von_obj {
        if has_other || (has_vbind_obj && has_von_obj) {
            // Multiple spreads or spread with other props: _mergeProps(...)
            ctx.use_helper(RuntimeHelper::MergeProps);
            ctx.push(ctx.helper(RuntimeHelper::MergeProps));
            ctx.push("(");

            let mut first_merge_arg = true;

            // Add v-bind object spread
            if has_vbind_obj {
                generate_vbind_object_exp(ctx, props);
                first_merge_arg = false;
            }

            // Add v-on object spread (wrapped with toHandlers)
            if has_von_obj {
                if !first_merge_arg {
                    ctx.push(", ");
                }
                generate_von_object_exp(ctx, props);
                first_merge_arg = false;
            }

            // Add other props as object (includes scope_id)
            // Inside mergeProps, skip normalizeClass/normalizeStyle - mergeProps handles it
            if has_other {
                if !first_merge_arg {
                    ctx.push(", ");
                }
                generate_props_object_inner(ctx, props, true, true);
            } else if let Some(ref sid) = scope_id {
                // No other props but we have scope_id, add it as separate object
                if !first_merge_arg {
                    ctx.push(", ");
                }
                ctx.push("{ \"");
                ctx.push(sid);
                ctx.push("\": \"\" }");
            }

            ctx.push(")");
        } else if has_vbind_obj {
            // v-bind="attrs" alone
            // If we have scope_id, we need to merge it with the bound object
            if let Some(ref sid) = scope_id {
                // _mergeProps(_normalizeProps(_guardReactiveProps(obj)), { "data-v-xxx": "" })
                ctx.use_helper(RuntimeHelper::MergeProps);
                ctx.use_helper(RuntimeHelper::NormalizeProps);
                ctx.use_helper(RuntimeHelper::GuardReactiveProps);
                ctx.push(ctx.helper(RuntimeHelper::MergeProps));
                ctx.push("(");
                ctx.push(ctx.helper(RuntimeHelper::NormalizeProps));
                ctx.push("(");
                ctx.push(ctx.helper(RuntimeHelper::GuardReactiveProps));
                ctx.push("(");
                generate_vbind_object_exp(ctx, props);
                ctx.push(")), { \"");
                ctx.push(sid);
                ctx.push("\": \"\" })");
            } else {
                // _normalizeProps(_guardReactiveProps(_ctx.attrs))
                ctx.use_helper(RuntimeHelper::NormalizeProps);
                ctx.use_helper(RuntimeHelper::GuardReactiveProps);
                ctx.push(ctx.helper(RuntimeHelper::NormalizeProps));
                ctx.push("(");
                ctx.push(ctx.helper(RuntimeHelper::GuardReactiveProps));
                ctx.push("(");
                generate_vbind_object_exp(ctx, props);
                ctx.push("))");
            }
        } else {
            // v-on="handlers" alone
            // If we have scope_id, we need to merge it with the handlers
            if let Some(ref sid) = scope_id {
                // _mergeProps(_toHandlers(handlers, true), { "data-v-xxx": "" })
                ctx.use_helper(RuntimeHelper::MergeProps);
                ctx.push(ctx.helper(RuntimeHelper::MergeProps));
                ctx.push("(");
                generate_von_object_exp(ctx, props);
                ctx.push(", { \"");
                ctx.push(sid);
                ctx.push("\": \"\" })");
            } else {
                // _toHandlers(_ctx.handlers)
                generate_von_object_exp(ctx, props);
            }
        }
        return;
    }

    // Check if we need normalizeProps wrapper
    // - dynamic v-model argument
    // - dynamic v-bind key (:[attr])
    // - dynamic v-on key (@[event])
    let has_dyn_vmodel = has_dynamic_vmodel(props);
    let has_dyn_key = has_dynamic_key(props);
    let needs_normalize = has_dyn_vmodel || has_dyn_key;
    if needs_normalize {
        ctx.use_helper(RuntimeHelper::NormalizeProps);
        ctx.push(ctx.helper(RuntimeHelper::NormalizeProps));
        ctx.push("(");
    }

    generate_props_object(ctx, props, false);

    // Close normalizeProps wrapper if needed
    if needs_normalize {
        ctx.push(")");
    }
}

/// Generate props as a regular object { key: value, ... }
fn generate_props_object(
    ctx: &mut CodegenContext,
    props: &[PropNode<'_>],
    skip_object_spreads: bool,
) {
    generate_props_object_inner(ctx, props, skip_object_spreads, false);
}

/// Generate the props object with optional class/style normalization skipping.
/// `inside_merge_props`: when true, skip normalizeClass/normalizeStyle wrappers
/// because mergeProps handles normalization internally.
fn generate_props_object_inner(
    ctx: &mut CodegenContext,
    props: &[PropNode<'_>],
    skip_object_spreads: bool,
    inside_merge_props: bool,
) {
    // When inside mergeProps, skip normalizeClass/normalizeStyle wrappers
    let prev_skip = ctx.skip_normalize;
    if inside_merge_props {
        ctx.skip_normalize = true;
    }

    // Clone scope_id to avoid borrow checker issues
    let scope_id = ctx.options.scope_id.clone();

    // Check for static class/style that need to be merged with dynamic
    let static_class = props.iter().find_map(|p| {
        if let PropNode::Attribute(attr) = p {
            if attr.name == "class" {
                return attr.value.as_ref().map(|v| v.content.as_str());
            }
        }
        None
    });

    let static_style = props.iter().find_map(|p| {
        if let PropNode::Attribute(attr) = p {
            if attr.name == "style" {
                return attr.value.as_ref().map(|v| v.content.as_str());
            }
        }
        None
    });

    let has_dynamic_class = props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                    return exp.content == "class";
                }
            }
        }
        false
    });

    let has_dynamic_style = props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                    return exp.content == "style";
                }
            }
        }
        false
    });

    // Skip static class/style if we have dynamic version (will merge them)
    let skip_static_class = static_class.is_some() && has_dynamic_class;
    let skip_static_style = static_style.is_some() && has_dynamic_style;

    // Count visible props (attributes + supported directives + scope_id if present)
    let has_scope_id = scope_id.is_some();
    let skip_is = ctx.skip_is_prop;
    let visible_count = props
        .iter()
        .filter(|p| {
            // Skip `is` prop for dynamic components
            if skip_is {
                match p {
                    PropNode::Attribute(attr) if attr.name == "is" => return false,
                    PropNode::Directive(dir)
                        if dir.name == "bind"
                            && matches!(&dir.arg, Some(ExpressionNode::Simple(exp)) if exp.content == "is") =>
                    {
                        return false
                    }
                    _ => {}
                }
            }
            match p {
                PropNode::Attribute(attr) => {
                    if skip_static_class && attr.name == "class" {
                        return false;
                    }
                    if skip_static_style && attr.name == "style" {
                        return false;
                    }
                    true
                }
                PropNode::Directive(dir) => is_supported_directive(dir),
            }
        })
        .count()
        + if has_scope_id { 1 } else { 0 };

    // Check if any prop requires a normalizer (class/style bindings) or uses helper functions (v-text)
    let has_normalizer = props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            // v-text uses _toDisplayString, which makes the output multiline
            if dir.name == "text" {
                return true;
            }
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                    return exp.content == "class" || exp.content == "style";
                }
            }
        }
        false
    });

    // Check if any v-on has inline handler (not just identifier) or has runtime modifiers
    // Also check for cached handlers which produce long expressions
    let has_inline_handler = props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "on" {
                // When cache_handlers is enabled, handlers produce long expressions
                // that need multiline formatting (except setup-const which aren't cached)
                if ctx.options.cache_handlers && dir.exp.is_some() {
                    let is_const = dir.exp.as_ref().is_some_and(|exp| {
                        if let ExpressionNode::Simple(simple) = exp {
                            if !simple.is_static {
                                let content = simple.content.trim();
                                if crate::transforms::is_simple_identifier(content) {
                                    if let Some(ref metadata) = ctx.options.binding_metadata {
                                        return matches!(
                                            metadata.bindings.get(content),
                                            Some(crate::options::BindingType::SetupConst)
                                        );
                                    }
                                }
                            }
                        }
                        false
                    });
                    if !is_const {
                        return true;
                    }
                }
                // Check for modifiers that will use withModifiers or withKeys (not event option modifiers)
                let has_runtime_modifier = dir.modifiers.iter().any(|m| {
                    let n = m.content.as_str();
                    // Event option modifiers (capture, once, passive) don't require multiline
                    // because they just modify the event name, not wrap the handler
                    !matches!(n, "capture" | "once" | "passive")
                });
                if has_runtime_modifier {
                    return true;
                }
                if let Some(ExpressionNode::Simple(simple)) = &dir.exp {
                    // Inline if contains operators, parens, or is not simple identifier
                    let content = simple.content.as_str();
                    return content.contains('(')
                        || content.contains('+')
                        || content.contains('-')
                        || content.contains('=')
                        || content.contains(' ');
                }
            }
        }
        false
    });

    let multiline = visible_count > 1 || has_normalizer || has_inline_handler;

    if multiline {
        ctx.push("{");
        ctx.indent();
    } else {
        ctx.push("{ ");
    }

    // Pre-scan: find duplicate v-on event names that need array merging
    let event_counts = count_event_names(props);

    let mut first = true;
    // Track which event names have already been output (for array merging)
    let mut emitted_events: std::collections::HashSet<String> = std::collections::HashSet::new();

    for prop in props {
        // Skip `is` prop when generating for dynamic components
        if ctx.skip_is_prop {
            match prop {
                PropNode::Attribute(attr) if attr.name == "is" => continue,
                PropNode::Directive(dir)
                    if dir.name == "bind"
                        && matches!(&dir.arg, Some(ExpressionNode::Simple(exp)) if exp.content == "is") =>
                {
                    continue
                }
                _ => {}
            }
        }

        match prop {
            PropNode::Attribute(attr) => {
                // Skip static class/style if merging with dynamic
                if skip_static_class && attr.name == "class" {
                    continue;
                }
                if skip_static_style && attr.name == "style" {
                    continue;
                }
                if !first {
                    ctx.push(",");
                }
                if multiline {
                    ctx.newline();
                } else if !first {
                    ctx.push(" ");
                }
                first = false;
                // Keys need quotes if they contain special characters (like hyphens)
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
                    // In inline mode, ref="refName" should reference the setup variable directly
                    // instead of being a string literal, if refName is a known binding
                    let is_ref_binding = attr.name == "ref"
                        && ctx.options.inline
                        && ctx
                            .options
                            .binding_metadata
                            .as_ref()
                            .is_some_and(|m| m.bindings.contains_key(value.content.as_str()));
                    if is_ref_binding {
                        ctx.push(&value.content);
                    } else {
                        ctx.push("\"");
                        ctx.push(&escape_js_string(&value.content));
                        ctx.push("\"");
                    }
                } else {
                    ctx.push("\"\"");
                }
            }
            PropNode::Directive(dir) => {
                // Skip v-bind/v-on object spreads (handled separately by generate_props)
                if skip_object_spreads
                    && dir.arg.is_none()
                    && (dir.name == "bind" || dir.name == "on")
                {
                    continue;
                }
                // Only add comma if directive produces valid output
                if is_supported_directive(dir) {
                    // Check for duplicate v-on events that should be merged into arrays
                    if dir.name == "on" {
                        if let Some(event_key) = get_von_event_key(dir) {
                            let count = event_counts.get(&event_key).copied().unwrap_or(0);
                            if count > 1 {
                                if emitted_events.contains(&event_key) {
                                    // Skip: already emitted as part of array
                                    continue;
                                }
                                // First occurrence: emit as array with all handlers for this event
                                emitted_events.insert(event_key.clone());
                                if !first {
                                    ctx.push(",");
                                }
                                if multiline {
                                    ctx.newline();
                                } else if !first {
                                    ctx.push(" ");
                                }
                                first = false;
                                generate_merged_event_handlers(
                                    ctx,
                                    props,
                                    &event_key,
                                    static_class,
                                    static_style,
                                );
                                continue;
                            }
                        }
                    }

                    if !first {
                        ctx.push(",");
                    }
                    if multiline {
                        ctx.newline();
                    } else if !first {
                        ctx.push(" ");
                    }
                    first = false;
                    generate_directive_prop_with_static(ctx, dir, static_class, static_style);
                }
            }
        }
    }

    // Add scope_id attribute for scoped CSS
    if let Some(ref sid) = scope_id {
        if !first {
            ctx.push(",");
        }
        if multiline {
            ctx.newline();
        } else if !first {
            ctx.push(" ");
        }
        ctx.push("\"");
        ctx.push(sid);
        ctx.push("\": \"\"");
    }

    if multiline {
        ctx.deindent();
        ctx.newline();
        ctx.push("}");
    } else {
        ctx.push(" }");
    }

    // Restore skip_normalize flag
    ctx.skip_normalize = prev_skip;
}

/// Get the event key for a v-on directive (e.g., "onClick", "onKeyupEnter")
fn get_von_event_key(dir: &DirectiveNode<'_>) -> Option<String> {
    if dir.name != "on" {
        return None;
    }
    if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
        if exp.is_static {
            let camelized = camelize(exp.content.as_str());
            let mut key = String::from("on");
            if let Some(first) = camelized.chars().next() {
                key.push(first.to_uppercase().next().unwrap_or(first));
                key.push_str(&camelized[first.len_utf8()..]);
            }
            Some(key)
        } else {
            None // Dynamic events can't be merged
        }
    } else {
        None
    }
}

/// Count occurrences of each event name across all v-on directives
fn count_event_names(props: &[PropNode<'_>]) -> std::collections::HashMap<String, usize> {
    let mut counts = std::collections::HashMap::new();
    for p in props {
        if let PropNode::Directive(dir) = p {
            if let Some(key) = get_von_event_key(dir) {
                *counts.entry(key).or_insert(0) += 1;
            }
        }
    }
    counts
}

/// Generate merged event handlers for the same event name as array syntax
/// e.g., onClick: [_ctx.a, _withModifiers(_ctx.b, ["ctrl"])]
fn generate_merged_event_handlers(
    ctx: &mut CodegenContext,
    props: &[PropNode<'_>],
    target_event_key: &str,
    _static_class: Option<&str>,
    _static_style: Option<&str>,
) {
    // Output the event key name (e.g., "onClick" or "\"onUpdate:modelValue\"")
    // Event names containing ':' need quotes for valid JavaScript
    if target_event_key.contains(':') {
        ctx.push("\"");
        ctx.push(target_event_key);
        ctx.push("\"");
    } else {
        ctx.push(target_event_key);
    }
    ctx.push(": [");

    // Output each handler as an element in the array
    let mut handler_idx = 0;
    for p in props {
        if let PropNode::Directive(dir) = p {
            if let Some(key) = get_von_event_key(dir) {
                if key == target_event_key {
                    if handler_idx > 0 {
                        ctx.push(", ");
                    }
                    generate_von_handler_value(ctx, dir);
                    handler_idx += 1;
                }
            }
        }
    }

    ctx.push("]");
}

/// Generate just the handler value part of a v-on directive (without the key name)
fn generate_von_handler_value(ctx: &mut CodegenContext, dir: &DirectiveNode<'_>) {
    // Classify modifiers (same logic as in generate_directive_prop_with_static)
    let event_name = if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
        exp.content.as_str()
    } else {
        ""
    };
    let is_keyboard_event = matches!(event_name, "keydown" | "keyup" | "keypress");

    let mut system_modifiers: Vec<&str> = Vec::new();
    let mut key_modifiers: Vec<&str> = Vec::new();

    for modifier in dir.modifiers.iter() {
        let mod_name = modifier.content.as_str();
        match mod_name {
            "capture" | "once" | "passive" | "native" => {}
            "left" | "right" => {
                if is_keyboard_event {
                    key_modifiers.push(mod_name);
                } else {
                    system_modifiers.push(mod_name);
                }
            }
            "stop" | "prevent" | "self" | "ctrl" | "shift" | "alt" | "meta" | "middle"
            | "exact" => {
                system_modifiers.push(mod_name);
            }
            "enter" | "tab" | "delete" | "esc" | "space" | "up" | "down" => {
                key_modifiers.push(mod_name);
            }
            _ => {
                key_modifiers.push(mod_name);
            }
        }
    }

    let has_system_mods = !system_modifiers.is_empty();
    let has_key_mods = !key_modifiers.is_empty();

    if has_key_mods {
        ctx.use_helper(RuntimeHelper::WithKeys);
        ctx.push("_withKeys(");
    }

    if has_system_mods {
        ctx.use_helper(RuntimeHelper::WithModifiers);
        ctx.push("_withModifiers(");
    }

    if let Some(exp) = &dir.exp {
        generate_event_handler(ctx, exp, false);
    } else {
        ctx.push("() => {}");
    }

    if has_system_mods {
        ctx.push(", [");
        for (i, mod_name) in system_modifiers.iter().enumerate() {
            if i > 0 {
                ctx.push(",");
            }
            ctx.push("\"");
            ctx.push(mod_name);
            ctx.push("\"");
        }
        ctx.push("])");
    }

    if has_key_mods {
        ctx.push(", [");
        for (i, mod_name) in key_modifiers.iter().enumerate() {
            if i > 0 {
                ctx.push(",");
            }
            ctx.push("\"");
            ctx.push(mod_name);
            ctx.push("\"");
        }
        ctx.push("])");
    }
}

/// Check if a directive will produce valid output
pub fn is_supported_directive(dir: &DirectiveNode<'_>) -> bool {
    // v-model with dynamic arg on components needs special props handling
    // Static v-model is handled via withDirectives for native elements or transformed for components
    if dir.name == "model" {
        return dir.arg.as_ref().is_some_and(|arg| match arg {
            ExpressionNode::Simple(exp) => !exp.is_static,
            ExpressionNode::Compound(_) => true,
        });
    }
    matches!(dir.name.as_str(), "bind" | "on" | "html" | "text")
}

/// Check if any v-bind prop has a dynamic key (v-bind with dynamic arg)
/// Note: v-on with dynamic arg uses _toHandlerKey() instead and doesn't need _normalizeProps
fn has_dynamic_key(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "bind" {
                if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                    return !exp.is_static;
                }
            }
        }
        false
    })
}

/// Check if element has dynamic v-model (with dynamic argument)
pub fn has_dynamic_vmodel(props: &[PropNode<'_>]) -> bool {
    props.iter().any(|p| {
        if let PropNode::Directive(dir) = p {
            if dir.name == "model" {
                return dir.arg.as_ref().is_some_and(|arg| match arg {
                    ExpressionNode::Simple(exp) => !exp.is_static,
                    ExpressionNode::Compound(_) => true,
                });
            }
        }
        false
    })
}

/// Generate directive as prop with optional static class/style merging
pub fn generate_directive_prop_with_static(
    ctx: &mut CodegenContext,
    dir: &DirectiveNode<'_>,
    static_class: Option<&str>,
    static_style: Option<&str>,
) {
    match dir.name.as_str() {
        "bind" => {
            let mut is_class = false;
            let mut is_style = false;

            // Check for modifiers
            let has_camel = dir.modifiers.iter().any(|m| m.content == "camel");
            let has_prop = dir.modifiers.iter().any(|m| m.content == "prop");
            let has_attr = dir.modifiers.iter().any(|m| m.content == "attr");

            if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                if !exp.is_static {
                    // Dynamic attribute name: [_ctx.expr || ""]: value
                    ctx.push("[");
                    // If the expression doesn't already have a prefix, add _ctx.
                    let content = exp.content.as_str();
                    if content.contains('.')
                        || content.starts_with('_')
                        || content.starts_with('$')
                        || content.contains('`')
                        || content.contains('(')
                    {
                        // Template literal or already prefixed expression
                        // For template literals, wrap with parens and prefix inner identifiers
                        if content.starts_with('`') {
                            ctx.push("(");
                            // Prefix identifiers inside template literals with _ctx.
                            let prefixed =
                                super::expression::generate_simple_expression_with_prefix(
                                    ctx, content,
                                );
                            ctx.push(&prefixed);
                            ctx.push(")");
                        } else {
                            generate_simple_expression(ctx, exp);
                        }
                    } else {
                        ctx.push("_ctx.");
                        ctx.push(content);
                    }
                    ctx.push(" || \"\"]: ");
                } else {
                    let key = &exp.content;
                    is_class = key == "class";
                    is_style = key == "style";

                    // Transform key based on modifiers
                    let transformed_key: vize_carton::String = if has_camel {
                        // Convert kebab-case to camelCase
                        camelize(key)
                    } else if has_prop {
                        // Add . prefix for DOM property binding
                        let mut name = String::with_capacity(1 + key.len());
                        name.push('.');
                        name.push_str(key);
                        name.into()
                    } else if has_attr {
                        // Add ^ prefix for attribute binding
                        let mut name = String::with_capacity(1 + key.len());
                        name.push('^');
                        name.push_str(key);
                        name.into()
                    } else {
                        key.to_string().into()
                    };

                    let needs_quotes = !is_valid_js_identifier(&transformed_key);
                    if needs_quotes {
                        ctx.push("\"");
                    }
                    ctx.push(&transformed_key);
                    if needs_quotes {
                        ctx.push("\"");
                    }
                    ctx.push(": ");
                }
            }
            if let Some(exp) = &dir.exp {
                if is_class && !ctx.skip_normalize {
                    ctx.use_helper(RuntimeHelper::NormalizeClass);
                    ctx.push("_normalizeClass(");
                    // Merge static class if present
                    if let Some(static_val) = static_class {
                        ctx.push("[\"");
                        ctx.push(static_val);
                        ctx.push("\", ");
                        generate_expression(ctx, exp);
                        ctx.push("]");
                    } else {
                        generate_expression(ctx, exp);
                    }
                    ctx.push(")");
                } else if is_style && !ctx.skip_normalize {
                    ctx.use_helper(RuntimeHelper::NormalizeStyle);
                    ctx.push("_normalizeStyle(");
                    // Merge static style if present
                    if let Some(static_val) = static_style {
                        ctx.push("[{");
                        // Parse static style and convert to object
                        for (i, part) in static_val
                            .split(';')
                            .filter(|s| !s.trim().is_empty())
                            .enumerate()
                        {
                            if i > 0 {
                                ctx.push(",");
                            }
                            let parts: Vec<&str> = part.splitn(2, ':').collect();
                            if parts.len() == 2 {
                                let key = parts[0].trim();
                                let value = parts[1].trim();
                                ctx.push("\"");
                                ctx.push(key);
                                ctx.push("\":\"");
                                ctx.push(value);
                                ctx.push("\"");
                            }
                        }
                        ctx.push("}, ");
                        generate_expression(ctx, exp);
                        ctx.push("]");
                    } else {
                        generate_expression(ctx, exp);
                    }
                    ctx.push(")");
                } else {
                    generate_expression(ctx, exp);
                }
            } else {
                ctx.push("undefined");
            }
        }
        "on" => {
            // Get event name first to determine context for modifiers
            let (event_name, is_dynamic_event) = if let Some(ExpressionNode::Simple(exp)) = &dir.arg
            {
                (exp.content.as_str(), !exp.is_static)
            } else {
                ("", false)
            };

            // Check if this is a keyboard event (for context-dependent modifiers)
            let is_keyboard_event = matches!(event_name, "keydown" | "keyup" | "keypress");

            // Collect modifiers into categories
            let mut event_option_modifiers: Vec<&str> = Vec::new();
            let mut system_modifiers: Vec<&str> = Vec::new();
            let mut key_modifiers: Vec<&str> = Vec::new();

            for modifier in dir.modifiers.iter() {
                let mod_name = modifier.content.as_str();
                match mod_name {
                    // Event option modifiers - appended to event name
                    "capture" | "once" | "passive" => {
                        event_option_modifiers.push(mod_name);
                    }
                    // "native" modifier is a no-op in Vue 3 (removed)
                    "native" => {}
                    // Context-dependent: left/right are arrow keys on keyboard events,
                    // mouse buttons on click events
                    "left" | "right" => {
                        if is_keyboard_event {
                            key_modifiers.push(mod_name);
                        } else {
                            system_modifiers.push(mod_name);
                        }
                    }
                    // System modifiers - wrapped with withModifiers
                    "stop" | "prevent" | "self" | "ctrl" | "shift" | "alt" | "meta" | "middle"
                    | "exact" => {
                        system_modifiers.push(mod_name);
                    }
                    // Key modifiers - wrapped with withKeys
                    "enter" | "tab" | "delete" | "esc" | "space" | "up" | "down" => {
                        key_modifiers.push(mod_name);
                    }
                    _ => {
                        // Unknown modifiers (including numeric keycodes) are treated as key modifiers
                        key_modifiers.push(mod_name);
                    }
                }
            }

            if let Some(ExpressionNode::Simple(exp)) = &dir.arg {
                if is_dynamic_event {
                    // Dynamic event name: [_toHandlerKey(_ctx.event)]:
                    ctx.use_helper(RuntimeHelper::ToHandlerKey);
                    ctx.push("[");
                    ctx.push(ctx.helper(RuntimeHelper::ToHandlerKey));
                    ctx.push("(");
                    let content = exp.content.as_str();
                    if content.contains('.') || content.starts_with('_') || content.starts_with('$')
                    {
                        generate_simple_expression(ctx, exp);
                    } else {
                        ctx.push("_ctx.");
                        ctx.push(content);
                    }
                    ctx.push(")]: ");
                } else {
                    let mut event_name = exp.content.as_str();

                    // Special mouse button modifiers that change the event name
                    // @click.right -> onContextmenu, @click.middle -> onMouseup
                    let has_right_modifier = system_modifiers.contains(&"right");
                    let has_middle_modifier = system_modifiers.contains(&"middle");

                    if event_name == "click" && has_right_modifier {
                        event_name = "contextmenu";
                    } else if event_name == "click" && has_middle_modifier {
                        event_name = "mouseup";
                    }

                    // Handle special event names like "update:modelValue"
                    if event_name.contains(':') {
                        // Event name with colon needs quotes (e.g., "onUpdate:modelValue")
                        let parts: Vec<&str> = event_name.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            ctx.push("\"on");
                            // Capitalize the first part (e.g., "update" -> "Update")
                            // Also convert kebab-case to camelCase
                            let first_part_camelized = camelize(parts[0]);
                            if let Some(first) = first_part_camelized.chars().next() {
                                ctx.push(&first.to_uppercase().to_string());
                                ctx.push(&first_part_camelized[first.len_utf8()..]);
                            }
                            ctx.push(":");
                            ctx.push(parts[1]);
                            // Append event option modifiers
                            for opt_mod in &event_option_modifiers {
                                ctx.push(&capitalize_first(opt_mod));
                            }
                            ctx.push("\": ");
                        }
                    } else {
                        // Simple event names don't need quotes (onUpdate, onClick)
                        // Convert kebab-case to camelCase first (e.g., "select-koma" -> "selectKoma")
                        let camelized = camelize(event_name);
                        ctx.push("on");
                        // Capitalize first letter of camelized name
                        if let Some(first) = camelized.chars().next() {
                            ctx.push(&first.to_uppercase().to_string());
                            ctx.push(&camelized[first.len_utf8()..]);
                        }
                        // Append event option modifiers (Capture, Once, Passive)
                        for opt_mod in &event_option_modifiers {
                            ctx.push(&capitalize_first(opt_mod));
                        }
                        ctx.push(": ");
                    }
                }
            }

            // Generate handler with optional withModifiers/withKeys wrappers
            // Order: _withKeys(_withModifiers(handler, [system_mods]), [key_mods])
            let has_system_mods = !system_modifiers.is_empty();
            let has_key_mods = !key_modifiers.is_empty();

            // Check if this handler needs caching
            // When cache_handlers is true, handlers are cached UNLESS the handler is a
            // setup-const binding (stable reference, no need for caching)
            // Pattern: _cache[n] || (_cache[n] = handler)
            // Simple identifiers get safety wrapper: (...args) => (_ctx.handler && _ctx.handler(...args))
            // Inline expressions get: $event => (expression)
            let is_const_handler = dir.exp.as_ref().is_some_and(|exp| {
                if let ExpressionNode::Simple(simple) = exp {
                    if !simple.is_static {
                        let content = simple.content.trim();
                        // Check if content is a simple identifier that's a setup-const binding
                        if crate::transforms::is_simple_identifier(content) {
                            if let Some(ref metadata) = ctx.options.binding_metadata {
                                return matches!(
                                    metadata.bindings.get(content),
                                    Some(crate::options::BindingType::SetupConst)
                                );
                            }
                        }
                    }
                }
                false
            });
            let needs_cache = ctx.options.cache_handlers && dir.exp.is_some() && !is_const_handler;

            if needs_cache {
                let cache_index = ctx.next_cache_index();
                ctx.push("_cache[");
                ctx.push(&cache_index.to_string());
                ctx.push("] || (_cache[");
                ctx.push(&cache_index.to_string());
                ctx.push("] = ");
            }

            if has_key_mods {
                ctx.use_helper(RuntimeHelper::WithKeys);
                ctx.push("_withKeys(");
            }

            if has_system_mods {
                ctx.use_helper(RuntimeHelper::WithModifiers);
                ctx.push("_withModifiers(");
            }

            // Generate the actual handler
            if let Some(exp) = &dir.exp {
                generate_event_handler(ctx, exp, needs_cache);
            } else {
                ctx.push("() => {}");
            }

            // Close withModifiers wrapper
            if has_system_mods {
                ctx.push(", [");
                for (i, mod_name) in system_modifiers.iter().enumerate() {
                    if i > 0 {
                        ctx.push(",");
                    }
                    ctx.push("\"");
                    ctx.push(mod_name);
                    ctx.push("\"");
                }
                ctx.push("])");
            }

            // Close withKeys wrapper
            if has_key_mods {
                ctx.push(", [");
                for (i, mod_name) in key_modifiers.iter().enumerate() {
                    if i > 0 {
                        ctx.push(",");
                    }
                    ctx.push("\"");
                    ctx.push(mod_name);
                    ctx.push("\"");
                }
                ctx.push("])");
            }

            // Close cache wrapper
            if needs_cache {
                ctx.push(")");
            }
        }
        "model" => {
            // Handle dynamic v-model on component
            // Generate: [_ctx.prop]: _ctx.value, ["onUpdate:" + _ctx.prop]: handler
            if let Some(ExpressionNode::Simple(arg_exp)) = &dir.arg {
                if !arg_exp.is_static {
                    let prop_name = &arg_exp.content;
                    let value_exp = dir
                        .exp
                        .as_ref()
                        .map(|e| match e {
                            ExpressionNode::Simple(s) => s.content.as_str(),
                            ExpressionNode::Compound(c) => c.loc.source.as_str(),
                        })
                        .unwrap_or("undefined");

                    // [_ctx.prop]: _ctx.value
                    ctx.push("[_ctx.");
                    ctx.push(prop_name);
                    ctx.push("]: ");
                    ctx.push(value_exp);
                    ctx.push(",");
                    ctx.newline();

                    // ["onUpdate:" + _ctx.prop]: $event => ((_ctx.value) = $event)
                    ctx.push("[\"onUpdate:\" + _ctx.");
                    ctx.push(prop_name);
                    ctx.push("]: $event => ((");
                    ctx.push(value_exp);
                    ctx.push(") = $event)");

                    // Add modifiers if present
                    if !dir.modifiers.is_empty() {
                        ctx.push(",");
                        ctx.newline();
                        // [_ctx.prop + "Modifiers"]: { modifier: true }
                        ctx.push("[_ctx.");
                        ctx.push(prop_name);
                        ctx.push(" + \"Modifiers\"]: { ");
                        for (i, modifier) in dir.modifiers.iter().enumerate() {
                            if i > 0 {
                                ctx.push(", ");
                            }
                            ctx.push(&modifier.content);
                            ctx.push(": true");
                        }
                        ctx.push(" }");
                    }
                }
            }
        }
        "html" => {
            // v-html="rawHtml" -> innerHTML: _ctx.rawHtml
            ctx.push("innerHTML: ");
            if let Some(exp) = &dir.exp {
                generate_expression(ctx, exp);
            } else {
                ctx.push("undefined");
            }
        }
        "text" => {
            // v-text="message" -> textContent: _toDisplayString(_ctx.message)
            ctx.use_helper(RuntimeHelper::ToDisplayString);
            ctx.push("textContent: ");
            ctx.push(ctx.helper(RuntimeHelper::ToDisplayString));
            ctx.push("(");
            if let Some(exp) = &dir.exp {
                generate_expression(ctx, exp);
            } else {
                ctx.push("undefined");
            }
            ctx.push(")");
        }
        _ => {
            // Other directives are skipped by is_supported_directive()
            // This case should not be reached in normal operation
        }
    }
}
