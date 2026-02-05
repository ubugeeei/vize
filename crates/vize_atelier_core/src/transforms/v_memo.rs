//! v-memo directive transform.
//!
//! Transforms v-memo directives for memoized rendering.

use vize_carton::String;

use crate::ast::*;
use crate::transform::TransformContext;

/// Check if element has v-memo directive
pub fn has_v_memo(el: &ElementNode<'_>) -> bool {
    el.props
        .iter()
        .any(|prop| matches!(prop, PropNode::Directive(dir) if dir.name == "memo"))
}

/// Get v-memo expression content as string
pub fn get_memo_deps(el: &ElementNode<'_>) -> Option<String> {
    for prop in el.props.iter() {
        if let PropNode::Directive(dir) = prop {
            if dir.name == "memo" {
                if let Some(exp) = &dir.exp {
                    return Some(match exp {
                        ExpressionNode::Simple(s) => s.content.clone(),
                        ExpressionNode::Compound(c) => c.loc.source.clone(),
                    });
                }
            }
        }
    }
    None
}

/// Remove v-memo directive from element
pub fn remove_v_memo(el: &mut ElementNode<'_>) {
    let mut i = 0;
    while i < el.props.len() {
        if let PropNode::Directive(dir) = &el.props[i] {
            if dir.name == "memo" {
                el.props.remove(i);
                return;
            }
        }
        i += 1;
    }
}

/// Transform v-memo directive - adds required helpers
pub fn process_v_memo(ctx: &mut TransformContext<'_>) {
    ctx.helper(RuntimeHelper::WithMemo);
    ctx.helper(RuntimeHelper::IsMemoSame);
}

/// Memo expression info for codegen
#[derive(Debug)]
pub struct MemoInfo {
    pub deps: String,
    pub cached_index: usize,
}

/// Generate v-memo wrapper code
pub fn generate_v_memo_wrapper(deps: &str) -> String {
    let mut out = String::with_capacity(deps.len() + 18);
    out.push_str("_withMemo([");
    out.push_str(deps);
    out.push_str("], () => ");
    out
}

/// Generate memo check code
pub fn generate_memo_check(deps: &str, cache_index: usize) -> String {
    let mut out = String::with_capacity(deps.len() + 24);
    out.push_str("_isMemoSame(_cache, ");
    out.push_str(&cache_index.to_string());
    out.push_str(", [");
    out.push_str(deps);
    out.push_str("])");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use bumpalo::Bump;

    #[test]
    fn test_has_v_memo() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, r#"<div v-memo="[a, b]">memoized</div>"#);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            assert!(has_v_memo(el));
        }
    }

    #[test]
    fn test_get_memo_deps() {
        let allocator = Bump::new();
        let (root, _) = parse(&allocator, r#"<div v-memo="[count]">{{ count }}</div>"#);

        if let TemplateChildNode::Element(el) = &root.children[0] {
            let deps = get_memo_deps(el);
            assert!(deps.is_some());
            assert_eq!(deps.unwrap().as_str(), "[count]");
        }
    }
}
