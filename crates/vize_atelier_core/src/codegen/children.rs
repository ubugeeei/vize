//! Children, text, comment, and interpolation generation functions.

use crate::ast::*;

use super::context::CodegenContext;
use super::expression::generate_expression;
use super::helpers::escape_js_string;
use super::node::generate_node;

/// Generate children array
pub fn generate_children(ctx: &mut CodegenContext, children: &[TemplateChildNode<'_>]) {
    generate_children_inner(ctx, children, false);
}

/// Generate children, forcing array form with createTextVNode (for withDirectives elements)
pub fn generate_children_force_array(ctx: &mut CodegenContext, children: &[TemplateChildNode<'_>]) {
    generate_children_inner(ctx, children, true);
}

fn generate_children_inner(
    ctx: &mut CodegenContext,
    children: &[TemplateChildNode<'_>],
    force_array: bool,
) {
    if children.is_empty() {
        ctx.push("null");
        return;
    }

    // Check if single text/interpolation child can be inlined (unless forced to array)
    if !force_array && children.len() == 1 {
        match &children[0] {
            TemplateChildNode::Text(text) => {
                ctx.push("\"");
                ctx.push(&escape_js_string(&text.content));
                ctx.push("\"");
                return;
            }
            TemplateChildNode::Interpolation(interp) => {
                let helper = ctx.helper(RuntimeHelper::ToDisplayString);
                ctx.use_helper(RuntimeHelper::ToDisplayString);
                ctx.push(helper);
                ctx.push("(");
                generate_expression(ctx, &interp.content);
                ctx.push(")");
                return;
            }
            _ => {}
        }
    }

    // Check if all children are text/interpolation - if so, use string concatenation (unless forced to array)
    let all_text_or_interp = children.iter().all(|child| {
        matches!(
            child,
            TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
        )
    });

    if !force_array && all_text_or_interp {
        // Generate concatenated expression: "text" + _toDisplayString(expr) + "more"
        for (i, child) in children.iter().enumerate() {
            if i > 0 {
                ctx.push(" + ");
            }
            match child {
                TemplateChildNode::Text(text) => {
                    ctx.push("\"");
                    ctx.push(&escape_js_string(&text.content));
                    ctx.push("\"");
                }
                TemplateChildNode::Interpolation(interp) => {
                    let helper = ctx.helper(RuntimeHelper::ToDisplayString);
                    ctx.use_helper(RuntimeHelper::ToDisplayString);
                    ctx.push(helper);
                    ctx.push("(");
                    generate_expression(ctx, &interp.content);
                    ctx.push(")");
                }
                _ => {}
            }
        }
        return;
    }

    ctx.push("[");
    ctx.indent();

    for (i, child) in children.iter().enumerate() {
        if i > 0 {
            ctx.push(",");
        }
        ctx.newline();
        // In array context, interpolations need to be wrapped in createTextVNode
        match child {
            TemplateChildNode::Interpolation(interp) => {
                let create_text = ctx.helper(RuntimeHelper::CreateText);
                ctx.use_helper(RuntimeHelper::CreateText);
                let to_display = ctx.helper(RuntimeHelper::ToDisplayString);
                ctx.use_helper(RuntimeHelper::ToDisplayString);
                ctx.push(create_text);
                ctx.push("(");
                ctx.push(to_display);
                ctx.push("(");
                generate_expression(ctx, &interp.content);
                ctx.push("), 1 /* TEXT */)");
            }
            _ => generate_node(ctx, child),
        }
    }

    ctx.deindent();
    ctx.newline();
    ctx.push("]");
}

/// Generate text node
pub fn generate_text(ctx: &mut CodegenContext, text: &TextNode) {
    let helper = ctx.helper(RuntimeHelper::CreateText);
    ctx.use_helper(RuntimeHelper::CreateText);
    ctx.push(helper);
    ctx.push("(\"");
    ctx.push(&escape_js_string(&text.content));
    ctx.push("\")");
}

/// Generate comment node
pub fn generate_comment(ctx: &mut CodegenContext, comment: &CommentNode) {
    let helper = ctx.helper(RuntimeHelper::CreateComment);
    ctx.use_helper(RuntimeHelper::CreateComment);
    ctx.push(helper);
    ctx.push("(\"");
    ctx.push(&escape_js_string(&comment.content));
    ctx.push("\")");
}

/// Generate interpolation
pub fn generate_interpolation(ctx: &mut CodegenContext, interp: &InterpolationNode<'_>) {
    let helper = ctx.helper(RuntimeHelper::ToDisplayString);
    ctx.use_helper(RuntimeHelper::ToDisplayString);
    ctx.push(helper);
    ctx.push("(");
    generate_expression(ctx, &interp.content);
    ctx.push(")");
}
