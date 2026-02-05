//! Text transform.
//!
//! Transforms text nodes and interpolation nodes.

use vize_carton::{String, Vec};

use crate::ast::*;
use crate::transform::TransformContext;

/// Transform text and interpolation children
pub fn transform_text_children(
    ctx: &mut TransformContext<'_>,
    children: &mut Vec<'_, TemplateChildNode<'_>>,
) {
    // Combine consecutive text and interpolation nodes
    let mut i = 0;
    while i < children.len() {
        let has_text = matches!(
            &children[i],
            TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
        );

        if !has_text {
            i += 1;
            continue;
        }

        // Find consecutive text/interpolation nodes
        let mut j = i + 1;
        while j < children.len() {
            if matches!(
                &children[j],
                TemplateChildNode::Text(_) | TemplateChildNode::Interpolation(_)
            ) {
                j += 1;
            } else {
                break;
            }
        }

        // If only one node and it's simple text, skip
        if j == i + 1 {
            if let TemplateChildNode::Text(_) = &children[i] {
                i += 1;
                continue;
            }
        }

        // For interpolations, add helper
        for k in i..j {
            if let TemplateChildNode::Interpolation(_) = &children[k] {
                ctx.helper(RuntimeHelper::ToDisplayString);
            }
        }

        i = j;
    }
}

/// Check if text is all whitespace
pub fn is_whitespace_only(text: &str) -> bool {
    text.chars().all(|c| c.is_whitespace())
}

/// Check if text is condensible whitespace
pub fn is_condensible_whitespace(text: &str) -> bool {
    text.chars()
        .all(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r')
}

/// Condense whitespace in text
pub fn condense_whitespace(text: &str) -> String {
    let mut result = std::string::String::new();
    let mut prev_was_space = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }

    result.into()
}

/// Build text call expression
pub fn build_text_call<'a>(
    ctx: &mut TransformContext<'a>,
    nodes: &[TemplateChildNode<'_>],
) -> Option<TextCallExpression<'a>> {
    if nodes.is_empty() {
        return None;
    }

    let mut parts: Vec<'a, TextPart> = Vec::new_in(ctx.allocator);

    for node in nodes {
        match node {
            TemplateChildNode::Text(text) => {
                parts.push(TextPart::Static(text.content.clone()));
            }
            TemplateChildNode::Interpolation(interp) => {
                ctx.helper(RuntimeHelper::ToDisplayString);
                match &interp.content {
                    ExpressionNode::Simple(exp) => {
                        parts.push(TextPart::Dynamic(exp.content.clone()));
                    }
                    ExpressionNode::Compound(exp) => {
                        parts.push(TextPart::Dynamic(exp.loc.source.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(TextCallExpression { parts })
}

/// Text call expression for codegen
#[derive(Debug)]
pub struct TextCallExpression<'a> {
    pub parts: Vec<'a, TextPart>,
}

/// Part of a text expression
#[derive(Debug)]
pub enum TextPart {
    Static(String),
    Dynamic(String),
}

impl TextPart {
    pub fn to_code(&self) -> String {
        match self {
            TextPart::Static(s) => {
                let escaped = escape_text(s);
                let mut out = String::with_capacity(escaped.len() + 2);
                out.push('"');
                out.push_str(&escaped);
                out.push('"');
                out
            }
            TextPart::Dynamic(s) => {
                let mut out = String::with_capacity(s.len() + 18);
                out.push_str("_toDisplayString(");
                out.push_str(s);
                out.push(')');
                out
            }
        }
    }
}

/// Escape text for code generation
fn escape_text(s: &str) -> std::string::String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_whitespace_only() {
        assert!(is_whitespace_only("   "));
        assert!(is_whitespace_only("\n\t"));
        assert!(!is_whitespace_only("hello"));
        assert!(!is_whitespace_only(" hello "));
    }

    #[test]
    fn test_condense_whitespace() {
        assert_eq!(condense_whitespace("hello  world"), "hello world");
        assert_eq!(condense_whitespace("  hello\n\nworld  "), " hello world ");
    }

    #[test]
    fn test_text_part_code() {
        let static_part = TextPart::Static("hello".into());
        assert_eq!(static_part.to_code(), "\"hello\"");

        let dynamic_part = TextPart::Dynamic("msg".into());
        assert_eq!(dynamic_part.to_code(), "_toDisplayString(msg)");
    }
}
