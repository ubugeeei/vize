//! Node generation functions.

use crate::ast::*;

use super::children::{generate_comment, generate_interpolation, generate_text};
use super::context::CodegenContext;
use super::element::generate_element;
use super::v_for::generate_for;
use super::v_if::generate_if;

/// Generate node code
pub fn generate_node(ctx: &mut CodegenContext, node: &TemplateChildNode<'_>) {
    match node {
        TemplateChildNode::Element(el) => generate_element(ctx, el),
        TemplateChildNode::Text(text) => generate_text(ctx, text),
        TemplateChildNode::Comment(comment) => generate_comment(ctx, comment),
        TemplateChildNode::Interpolation(interp) => generate_interpolation(ctx, interp),
        TemplateChildNode::If(if_node) => generate_if(ctx, if_node),
        TemplateChildNode::For(for_node) => generate_for(ctx, for_node),
        TemplateChildNode::Hoisted(index) => {
            // Output reference to hoisted variable
            ctx.push("_hoisted_");
            ctx.push(&(index + 1).to_string());
        }
        _ => {
            ctx.push("null /* unsupported node */");
        }
    }
}
