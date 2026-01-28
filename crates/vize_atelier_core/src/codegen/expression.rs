//! Expression generation functions.

use crate::ast::*;

use super::context::CodegenContext;

/// Generate expression
pub fn generate_expression(ctx: &mut CodegenContext, expr: &ExpressionNode<'_>) {
    match expr {
        ExpressionNode::Simple(exp) => {
            generate_simple_expression(ctx, exp);
        }
        ExpressionNode::Compound(comp) => {
            for child in comp.children.iter() {
                match child {
                    CompoundExpressionChild::Simple(exp) => {
                        generate_simple_expression(ctx, exp);
                    }
                    CompoundExpressionChild::String(s) => {
                        ctx.push(s);
                    }
                    CompoundExpressionChild::Symbol(helper) => {
                        ctx.push(ctx.helper(*helper));
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Generate simple expression
pub fn generate_simple_expression(ctx: &mut CodegenContext, exp: &SimpleExpressionNode<'_>) {
    if exp.is_static {
        ctx.push("\"");
        ctx.push(&exp.content);
        ctx.push("\"");
    } else {
        // Strip TypeScript if needed
        if ctx.options.is_ts && exp.content.contains(" as ") {
            let stripped = crate::transforms::strip_typescript_from_expression(&exp.content);
            ctx.push(&stripped);
        } else {
            // Expression content should already be processed by transform phase
            // (e.g., "msg" -> "_ctx.msg" if prefix_identifiers is enabled)
            ctx.push(&exp.content);
        }
    }
}

/// Check if a string is a simple member expression like _ctx.foo or $setup.bar
/// This is used to determine if an event handler needs wrapping
pub fn is_simple_member_expression(s: &str) -> bool {
    // Check for pattern like _ctx.identifier or $setup.identifier
    if let Some(dot_pos) = s.find('.') {
        let prefix = &s[..dot_pos];
        let suffix = &s[dot_pos + 1..];
        // Prefix should be _ctx, $setup, or similar
        let valid_prefix = prefix == "_ctx" || prefix == "$setup" || prefix == "$props";
        // Suffix should be a simple identifier (no dots, no parens, etc.)
        let valid_suffix = !suffix.is_empty()
            && !suffix.contains('.')
            && !suffix.contains('(')
            && !suffix.contains('[');
        return valid_prefix && valid_suffix;
    }
    false
}

/// Check if an event handler expression is an inline handler
/// Inline handlers are expressions that are NOT simple identifiers or member expressions
/// Note: This is kept for potential future use (e.g., optimizations)
#[allow(dead_code)]
pub fn is_inline_handler(exp: &ExpressionNode<'_>) -> bool {
    match exp {
        ExpressionNode::Simple(simple) => {
            if simple.is_static {
                return false;
            }

            // Use the ORIGINAL source expression, not the transformed content
            // During transform phase, inline handlers like "count++" get wrapped as
            // "$event => (count.value++)" which would incorrectly be detected as "already arrow function"
            let content = simple.loc.source.as_str();

            // Already an arrow function or function expression - not inline
            if content.contains("=>") || content.trim().starts_with("function") {
                return false;
            }

            // Simple identifier or member expression - not inline (method reference)
            if crate::transforms::is_simple_identifier(content)
                || is_simple_member_expression(content)
            {
                return false;
            }

            // Everything else is an inline handler (needs caching)
            true
        }
        ExpressionNode::Compound(_) => {
            // Compound expressions are typically inline
            true
        }
    }
}

/// Generate event handler expression
/// Wraps inline expressions in arrow functions, strips TypeScript, and prefixes identifiers
/// When `for_caching` is true, simple identifiers are wrapped with safety check
pub fn generate_event_handler(
    ctx: &mut CodegenContext,
    exp: &ExpressionNode<'_>,
    for_caching: bool,
) {
    match exp {
        ExpressionNode::Simple(simple) => {
            if simple.is_static {
                ctx.push("\"");
                ctx.push(&simple.content);
                ctx.push("\"");
                return;
            }

            let content = &simple.content;

            // Step 1: Strip TypeScript if needed
            let ts_stripped = if ctx.options.is_ts && content.contains(" as ") {
                crate::transforms::strip_typescript_from_expression(content)
            } else {
                content.to_string()
            };

            // Step 2: Prefix identifiers if needed
            let processed = if ctx.options.prefix_identifiers {
                crate::transforms::prefix_identifiers_in_expression(&ts_stripped)
            } else {
                ts_stripped
            };

            // Check if it's already an arrow function or function expression
            if processed.contains("=>") || processed.trim().starts_with("function") {
                ctx.push(&processed);
                return;
            }

            // Check if it's a simple identifier or member expression (method name/reference)
            // _ctx.handler, handler, $setup.handler
            if crate::transforms::is_simple_identifier(&processed)
                || is_simple_member_expression(&processed)
            {
                if for_caching {
                    // When caching, wrap simple identifiers with safety check:
                    // (...args) => (_ctx.handler && _ctx.handler(...args))
                    ctx.push("(...args) => (");
                    ctx.push(&processed);
                    ctx.push(" && ");
                    ctx.push(&processed);
                    ctx.push("(...args))");
                } else {
                    // Not caching: use directly
                    ctx.push(&processed);
                }
                return;
            }

            // Compound expression (function call, etc.): wrap as $event => (expression)
            ctx.push("$event => (");
            ctx.push(&processed);
            ctx.push(")");
        }
        ExpressionNode::Compound(comp) => {
            // For compound expressions, generate normally
            for child in comp.children.iter() {
                match child {
                    CompoundExpressionChild::Simple(exp) => {
                        generate_simple_expression(ctx, exp);
                    }
                    CompoundExpressionChild::String(s) => {
                        ctx.push(s);
                    }
                    CompoundExpressionChild::Symbol(helper) => {
                        ctx.push(ctx.helper(*helper));
                    }
                    _ => {}
                }
            }
        }
    }
}
