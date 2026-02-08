//! Transform infrastructure for Vue template AST.
//!
//! This module provides the transform context, traversal, and base transform traits.

mod context;
pub mod element;
pub mod structural;
pub mod traverse;

use vize_carton::{Box, Bump, FxHashSet, String, Vec};
use vize_croquis::{Croquis, ScopeChain};

use crate::ast::*;
use crate::errors::CompilerError;
use crate::options::TransformOptions;

use traverse::traverse_children;

/// Transform function for nodes - returns optional exit function(s)
pub type NodeTransform<'a> =
    fn(&mut TransformContext<'a>, &mut TemplateChildNode<'a>) -> Option<std::vec::Vec<ExitFn<'a>>>;

/// Exit function called after children are processed
pub type ExitFn<'a> = std::boxed::Box<dyn FnOnce(&mut TransformContext<'a>) + 'a>;

/// Transform function for directives
pub type DirectiveTransform<'a> = fn(
    &mut TransformContext<'a>,
    &mut ElementNode<'a>,
    &DirectiveNode<'a>,
) -> Option<DirectiveTransformResult<'a>>;

/// Result of a directive transform
pub struct DirectiveTransformResult<'a> {
    /// Props to add to the element
    pub props: Vec<'a, PropNode<'a>>,
    /// Whether to remove the directive
    pub remove_directive: bool,
    /// SSR tag type hint
    pub ssr_tag_type: Option<u8>,
}

/// Structural directive transform (v-if, v-for)
pub type StructuralDirectiveTransform<'a> =
    fn(&mut TransformContext<'a>, &mut ElementNode<'a>, &DirectiveNode<'a>) -> Option<ExitFn<'a>>;

/// Transform context for AST traversal
pub struct TransformContext<'a> {
    /// Arena allocator
    pub allocator: &'a Bump,
    /// Transform options
    pub options: TransformOptions,
    /// Source code
    pub source: String,
    /// Root node reference
    pub root: Option<*mut RootNode<'a>>,
    /// Parent node stack
    pub parent: Option<ParentNode<'a>>,
    /// Grandparent node
    pub grandparent: Option<ParentNode<'a>>,
    /// Current node being transformed
    pub current_node: Option<*mut TemplateChildNode<'a>>,
    /// Child index in parent
    pub child_index: usize,
    /// Helpers used
    pub helpers: FxHashSet<RuntimeHelper>,
    /// Components used (Vec to maintain template order for code generation)
    pub components: std::vec::Vec<String>,
    /// Directives used (Vec to maintain template order for code generation)
    pub directives: std::vec::Vec<String>,
    /// Hoisted expressions
    pub hoists: Vec<'a, Option<JsChildNode<'a>>>,
    /// Cached expressions
    pub cached: Vec<'a, Option<Box<'a, CacheExpression<'a>>>>,
    /// Temp variable count
    pub temps: u32,
    /// Scope chain for tracking variable visibility
    pub scope_chain: ScopeChain,
    /// Scoped slots
    pub scoped_slots: u32,
    /// Whether in v-once
    pub in_v_once: bool,
    /// Whether in SSR
    pub in_ssr: bool,
    /// Errors collected
    pub errors: std::vec::Vec<CompilerError>,
    /// Node was removed flag
    pub(crate) node_removed: bool,
    /// Semantic analysis summary (optional, for enhanced transforms)
    pub(crate) analysis: Option<&'a Croquis>,
}

/// Enum for parent node types
#[derive(Clone, Copy)]
pub enum ParentNode<'a> {
    Root(*mut RootNode<'a>),
    Element(*mut ElementNode<'a>),
    If(*mut IfNode<'a>),
    IfBranch(*mut IfBranchNode<'a>),
    For(*mut ForNode<'a>),
}

impl<'a> ParentNode<'a> {
    /// Get mutable access to children through raw pointer.
    ///
    /// # Safety
    /// This uses interior mutability via raw pointers stored in the enum variants.
    /// The raw pointers are valid for the duration of the transform and mutation
    /// through them is safe as long as we don't create overlapping mutable references.
    #[allow(clippy::mut_from_ref)]
    pub fn children_mut(&self) -> &mut Vec<'a, TemplateChildNode<'a>> {
        unsafe {
            match self {
                ParentNode::Root(r) => &mut (*(*r)).children,
                ParentNode::Element(e) => &mut (*(*e)).children,
                ParentNode::If(_) => panic!("IfNode doesn't have direct children"),
                ParentNode::IfBranch(b) => &mut (*(*b)).children,
                ParentNode::For(f) => &mut (*(*f)).children,
            }
        }
    }
}

/// Transform the root AST node
pub fn transform<'a>(
    allocator: &'a Bump,
    root: &mut RootNode<'a>,
    options: TransformOptions,
    analysis: Option<&'a Croquis>,
) {
    let source = root.source.clone();
    let mut ctx = if let Some(analysis) = analysis {
        TransformContext::with_analysis(allocator, source, options, analysis)
    } else {
        TransformContext::new(allocator, source, options)
    };
    ctx.root = Some(root as *mut _);

    // Transform the root children
    traverse_children(&mut ctx, ParentNode::Root(root as *mut _));

    // Apply static hoisting after traversal (before codegen)
    use crate::transforms::hoist_static::hoist_static;
    hoist_static(&mut ctx, &mut root.children);

    // Create root codegen node
    create_root_codegen(&mut ctx, root);

    // Update root with context results
    for helper in ctx.helpers.into_iter() {
        root.helpers.push(helper);
    }
    for component in ctx.components.into_iter() {
        root.components.push(component);
    }
    for directive in ctx.directives.into_iter() {
        root.directives.push(directive);
    }
    // Transfer hoisted nodes to root
    for hoist in ctx.hoists.into_iter() {
        root.hoists.push(hoist);
    }
    root.temps = ctx.temps;
    root.transformed = true;
}

/// Create codegen node for root
fn create_root_codegen<'a>(ctx: &mut TransformContext<'a>, root: &mut RootNode<'a>) {
    if root.children.is_empty() {
        return;
    }

    if root.children.len() > 1 {
        // Multiple root children need to be wrapped in a fragment
        ctx.helper(RuntimeHelper::OpenBlock);
        ctx.helper(RuntimeHelper::CreateElementBlock);
        ctx.helper(RuntimeHelper::Fragment);
    }

    // Root codegen node is handled in codegen directly for now
    root.codegen_node = None;
}

#[cfg(test)]
mod tests {
    use super::transform;
    use crate::codegen::generate;
    use crate::options::{CodegenOptions, TransformOptions};
    use crate::parser::parse;
    use bumpalo::Bump;

    #[test]
    fn test_transform_simple_element() {
        assert_transform!("<div>hello</div>" => helpers: [CreateElementVNode]);
    }

    #[test]
    fn test_transform_interpolation() {
        assert_transform!("{{ msg }}" => helpers: [ToDisplayString]);
    }

    #[test]
    fn test_transform_component() {
        assert_transform!("<MyComponent></MyComponent>" => components: ["MyComponent"]);
        assert_transform!("<MyComponent></MyComponent>" => helpers: [ResolveComponent]);
    }

    #[test]
    fn test_transform_v_if() {
        assert_transform!("<div v-if=\"show\">hello</div>" => helpers: [OpenBlock, CreateBlock, Fragment, CreateComment]);
    }

    #[test]
    fn test_transform_v_for() {
        assert_transform!("<div v-for=\"item in items\">{{ item }}</div>" => helpers: [RenderList, OpenBlock, CreateBlock, Fragment]);
    }

    #[test]
    fn test_v_if_creates_if_node() {
        let allocator = Bump::new();
        let (mut root, errors) = parse(&allocator, r#"<div v-if="show">visible</div>"#);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        transform(&allocator, &mut root, TransformOptions::default(), None);

        // After transform, root should have 1 child: an IfNode
        assert_eq!(
            root.children.len(),
            1,
            "Should have 1 child after transform"
        );

        match &root.children[0] {
            crate::ast::TemplateChildNode::If(if_node) => {
                assert_eq!(if_node.branches.len(), 1, "Should have 1 branch");
                // First branch should have condition "show"
                let branch = &if_node.branches[0];
                assert!(branch.condition.is_some(), "Branch should have condition");
            }
            other => panic!("Expected IfNode, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn test_v_if_else_creates_branches() {
        let allocator = Bump::new();
        let (mut root, errors) = parse(
            &allocator,
            r#"<div v-if="show">yes</div><div v-else>no</div>"#,
        );
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        transform(&allocator, &mut root, TransformOptions::default(), None);

        // After transform, should have 1 IfNode with 2 branches
        assert_eq!(
            root.children.len(),
            1,
            "Should have 1 child (IfNode) after transform, got {}",
            root.children.len()
        );

        match &root.children[0] {
            crate::ast::TemplateChildNode::If(if_node) => {
                assert_eq!(
                    if_node.branches.len(),
                    2,
                    "Should have 2 branches (if + else)"
                );
                // First branch has condition, second doesn't (v-else)
                assert!(
                    if_node.branches[0].condition.is_some(),
                    "First branch should have condition"
                );
                assert!(
                    if_node.branches[1].condition.is_none(),
                    "Second branch (else) should not have condition"
                );
            }
            other => panic!("Expected IfNode, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn test_v_for_creates_for_node() {
        let allocator = Bump::new();
        let (mut root, errors) =
            parse(&allocator, r#"<div v-for="item in items">{{ item }}</div>"#);
        assert!(errors.is_empty(), "Parse errors: {:?}", errors);

        transform(&allocator, &mut root, TransformOptions::default(), None);

        // After transform, root should have 1 child: a ForNode
        assert_eq!(
            root.children.len(),
            1,
            "Should have 1 child after transform"
        );

        match &root.children[0] {
            crate::ast::TemplateChildNode::For(for_node) => {
                // Check source is "items"
                match &for_node.source {
                    crate::ast::ExpressionNode::Simple(exp) => {
                        assert_eq!(exp.content.as_str(), "items", "Source should be 'items'");
                    }
                    _ => panic!("Expected Simple expression for source"),
                }
                // Check value alias is "item"
                assert!(for_node.value_alias.is_some(), "Should have value alias");
                match for_node.value_alias.as_ref().unwrap() {
                    crate::ast::ExpressionNode::Simple(exp) => {
                        assert_eq!(exp.content.as_str(), "item", "Value alias should be 'item'");
                    }
                    _ => panic!("Expected Simple expression for value alias"),
                }
            }
            other => panic!("Expected ForNode, got {:?}", std::mem::discriminant(other)),
        }
    }

    #[test]
    fn test_codegen_v_if() {
        let allocator = Bump::new();
        let (mut root, _) = parse(&allocator, r#"<div v-if="show">visible</div>"#);
        transform(&allocator, &mut root, TransformOptions::default(), None);

        let result = generate(&root, CodegenOptions::default());
        println!("v-if codegen:\n{}", result.code);

        // Should contain openBlock and createBlock for v-if
        assert!(
            result.code.contains("openBlock"),
            "Should contain openBlock"
        );
    }
}
