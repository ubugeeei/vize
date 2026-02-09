//! v-html transform for DOM elements.
//!
//! v-html sets the element's innerHTML.

use vize_atelier_core::DirectiveNode;

/// Check if directive is v-html
pub fn is_v_html(dir: &DirectiveNode<'_>) -> bool {
    dir.name.as_str() == "html"
}

/// Generate innerHTML prop for v-html
pub fn generate_html_prop(dir: &DirectiveNode<'_>) -> Option<(String, String)> {
    if let Some(vize_atelier_core::ExpressionNode::Simple(simple)) = &dir.exp {
        return Some((String::from("innerHTML"), simple.content.to_string()));
    }
    None
}

/// Generate v-html warning (innerHTML can be a security risk)
pub fn generate_html_warning() -> &'static str {
    "v-html will override any existing children with innerHTML. Use with caution as it can lead to XSS vulnerabilities."
}

#[cfg(test)]
mod tests {
    use vize_atelier_core::{DirectiveNode, ExpressionNode, SimpleExpressionNode, SourceLocation};
    use vize_carton::{Box, Bump};

    use super::*;

    fn create_test_directive<'a>(allocator: &'a Bump, name: &str, exp: &str) -> DirectiveNode<'a> {
        let mut dir = DirectiveNode::new(allocator, name, SourceLocation::STUB);
        let exp_node = SimpleExpressionNode::new(exp, false, SourceLocation::STUB);
        let boxed = Box::new_in(exp_node, allocator);
        dir.exp = Some(ExpressionNode::Simple(boxed));
        dir
    }

    #[test]
    fn test_is_v_html() {
        let allocator = Bump::new();
        let dir = create_test_directive(&allocator, "html", "content");
        assert!(is_v_html(&dir));
    }

    #[test]
    fn test_generate_html_prop() {
        let allocator = Bump::new();
        let dir = create_test_directive(&allocator, "html", "content");
        let result = generate_html_prop(&dir);
        assert!(result.is_some());
        let (key, value) = result.unwrap();
        assert_eq!(key, "innerHTML");
        assert_eq!(value, "content");
    }

    #[test]
    fn test_is_v_html_false() {
        let allocator = Bump::new();
        let dir = create_test_directive(&allocator, "text", "content");
        assert!(!is_v_html(&dir));
    }

    #[test]
    fn test_generate_html_prop_no_exp() {
        let allocator = Bump::new();
        let dir = DirectiveNode::new(&allocator, "html", SourceLocation::STUB);
        assert!(generate_html_prop(&dir).is_none());
    }

    #[test]
    fn test_generate_html_warning() {
        let warning = generate_html_warning();
        assert!(warning.contains("XSS"));
        assert!(warning.contains("innerHTML"));
    }
}
