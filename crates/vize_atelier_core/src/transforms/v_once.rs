//! v-once directive transform.
//!
//! Transforms v-once directives for one-time rendering.

use crate::ast::*;
use crate::transform::TransformContext;

/// Check if element has v-once directive
pub fn has_v_once(el: &ElementNode<'_>) -> bool {
    el.props
        .iter()
        .any(|prop| matches!(prop, PropNode::Directive(dir) if dir.name == "once"))
}

/// Remove v-once directive from element
pub fn remove_v_once(el: &mut ElementNode<'_>) {
    let mut i = 0;
    while i < el.props.len() {
        if let PropNode::Directive(dir) = &el.props[i] {
            if dir.name == "once" {
                el.props.remove(i);
                return;
            }
        }
        i += 1;
    }
}

/// Transform v-once directive
pub fn transform_v_once<'a>(ctx: &mut TransformContext<'a>, _el: &ElementNode<'a>) -> bool {
    // Set v-once flag in context
    ctx.in_v_once = true;

    // Add helper
    ctx.helper(RuntimeHelper::SetBlockTracking);

    true
}

/// Generate v-once cache wrapper
pub fn generate_v_once_wrapper(index: usize) -> String {
    let index_str = index.to_string();
    let mut out = String::with_capacity(32 + index_str.len());
    out.push_str("_cache[");
    out.push_str(&index_str);
    out.push_str("] || (_setBlockTracking(-1), _cache[");
    out.push_str(&index_str);
    out.push_str("] = ");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use bumpalo::Bump;

    #[test]
    fn test_has_v_once() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, r#"<div v-once>static</div>"#);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert!(has_v_once(el));
        }
    }

    #[test]
    fn test_no_v_once() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, r#"<div>dynamic</div>"#);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert!(!has_v_once(el));
        }
    }
}
