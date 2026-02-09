//! SSR-specific transforms.
//!
//! This module contains SSR-specific transform passes that modify the AST
//! for optimal SSR code generation.

use vize_atelier_core::ast::{ElementNode, ExpressionNode, PropNode};

// For now, most SSR-specific transforms are integrated directly into the codegen.
// This module will be expanded as we add more sophisticated transforms.

/// Check if an element has v-model directive
pub fn has_v_model(el: &ElementNode) -> bool {
    el.props
        .iter()
        .any(|p| matches!(p, PropNode::Directive(dir) if dir.name == "model"))
}

/// Check if an element has v-show directive
pub fn has_v_show(el: &ElementNode) -> bool {
    el.props
        .iter()
        .any(|p| matches!(p, PropNode::Directive(dir) if dir.name == "show"))
}

/// Check if an element has v-html directive
pub fn has_v_html(el: &ElementNode) -> bool {
    el.props
        .iter()
        .any(|p| matches!(p, PropNode::Directive(dir) if dir.name == "html"))
}

/// Check if an element has v-text directive
pub fn has_v_text(el: &ElementNode) -> bool {
    el.props
        .iter()
        .any(|p| matches!(p, PropNode::Directive(dir) if dir.name == "text"))
}

/// Get v-model expression if present
pub fn get_v_model_exp<'a>(el: &'a ElementNode<'a>) -> Option<&'a ExpressionNode<'a>> {
    for prop in &el.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "model" {
                return dir.exp.as_ref();
            }
        }
    }
    None
}

/// Get v-show expression if present
pub fn get_v_show_exp<'a>(el: &'a ElementNode<'a>) -> Option<&'a ExpressionNode<'a>> {
    for prop in &el.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "show" {
                return dir.exp.as_ref();
            }
        }
    }
    None
}

/// Get v-html expression if present
pub fn get_v_html_exp<'a>(el: &'a ElementNode<'a>) -> Option<&'a ExpressionNode<'a>> {
    for prop in &el.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "html" {
                return dir.exp.as_ref();
            }
        }
    }
    None
}

/// Get v-text expression if present
pub fn get_v_text_exp<'a>(el: &'a ElementNode<'a>) -> Option<&'a ExpressionNode<'a>> {
    for prop in &el.props {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "text" {
                return dir.exp.as_ref();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use vize_atelier_core::ast::{
        DirectiveNode, ElementNode, ExpressionNode, SimpleExpressionNode, SourceLocation,
    };
    use vize_carton::{Box, Bump};

    fn make_element_with_directive<'a>(
        allocator: &'a Bump,
        tag: &str,
        dir_name: &str,
        exp: Option<&str>,
    ) -> &'a ElementNode<'a> {
        let mut el = ElementNode::new(allocator, tag, SourceLocation::STUB);
        let mut dir = DirectiveNode::new(allocator, dir_name, SourceLocation::STUB);
        if let Some(e) = exp {
            let exp_node = SimpleExpressionNode::new(e, false, SourceLocation::STUB);
            let boxed = Box::new_in(exp_node, allocator);
            dir.exp = Some(ExpressionNode::Simple(boxed));
        }
        el.props
            .push(PropNode::Directive(Box::new_in(dir, allocator)));
        allocator.alloc(el)
    }

    fn make_plain_element<'a>(allocator: &'a Bump, tag: &str) -> &'a ElementNode<'a> {
        let el = ElementNode::new(allocator, tag, SourceLocation::STUB);
        allocator.alloc(el)
    }

    #[test]
    fn test_has_v_model_true() {
        let allocator = Bump::new();
        let el = make_element_with_directive(&allocator, "input", "model", Some("msg"));
        assert!(has_v_model(el));
    }

    #[test]
    fn test_has_v_model_false() {
        let allocator = Bump::new();
        let el = make_plain_element(&allocator, "input");
        assert!(!has_v_model(el));
    }

    #[test]
    fn test_has_v_show_true() {
        let allocator = Bump::new();
        let el = make_element_with_directive(&allocator, "div", "show", Some("visible"));
        assert!(has_v_show(el));
    }

    #[test]
    fn test_has_v_show_false() {
        let allocator = Bump::new();
        let el = make_plain_element(&allocator, "div");
        assert!(!has_v_show(el));
    }

    #[test]
    fn test_has_v_html_true() {
        let allocator = Bump::new();
        let el = make_element_with_directive(&allocator, "div", "html", Some("content"));
        assert!(has_v_html(el));
    }

    #[test]
    fn test_has_v_html_false() {
        let allocator = Bump::new();
        let el = make_plain_element(&allocator, "div");
        assert!(!has_v_html(el));
    }

    #[test]
    fn test_has_v_text_true() {
        let allocator = Bump::new();
        let el = make_element_with_directive(&allocator, "div", "text", Some("msg"));
        assert!(has_v_text(el));
    }

    #[test]
    fn test_has_v_text_false() {
        let allocator = Bump::new();
        let el = make_plain_element(&allocator, "div");
        assert!(!has_v_text(el));
    }

    #[test]
    fn test_get_v_model_exp_some() {
        let allocator = Bump::new();
        let el = make_element_with_directive(&allocator, "input", "model", Some("msg"));
        assert!(get_v_model_exp(el).is_some());
    }

    #[test]
    fn test_get_v_model_exp_none() {
        let allocator = Bump::new();
        let el = make_plain_element(&allocator, "input");
        assert!(get_v_model_exp(el).is_none());
    }

    #[test]
    fn test_get_v_show_exp_some() {
        let allocator = Bump::new();
        let el = make_element_with_directive(&allocator, "div", "show", Some("visible"));
        assert!(get_v_show_exp(el).is_some());
    }
}
