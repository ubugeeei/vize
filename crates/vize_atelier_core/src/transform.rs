//! Transform infrastructure for Vue template AST.
//!
//! This module provides the transform context, traversal, and base transform traits.

use vize_carton::{is_builtin_directive, Box, Bump, CompactString, FxHashSet, String, Vec};
use vize_croquis::{BindingType, Croquis, ScopeBinding, ScopeChain, ScopeKind, VForScopeData};

use crate::ast::*;
use crate::errors::{CompilerError, ErrorCode};
use crate::options::TransformOptions;

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
    node_removed: bool,
    /// Semantic analysis summary (optional, for enhanced transforms)
    analysis: Option<&'a Croquis>,
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

impl<'a> TransformContext<'a> {
    /// Create a new transform context
    pub fn new(allocator: &'a Bump, source: String, options: TransformOptions) -> Self {
        let ssr = options.ssr;
        Self {
            allocator,
            source,
            options,
            root: None,
            parent: None,
            grandparent: None,
            current_node: None,
            child_index: 0,
            helpers: FxHashSet::default(),
            components: std::vec::Vec::new(),
            directives: std::vec::Vec::new(),
            hoists: Vec::new_in(allocator),
            cached: Vec::new_in(allocator),
            temps: 0,
            scope_chain: ScopeChain::new(),
            scoped_slots: 0,
            in_v_once: false,
            in_ssr: ssr,
            errors: std::vec::Vec::new(),
            node_removed: false,
            analysis: None,
        }
    }

    /// Create a new transform context with semantic analysis data
    pub fn with_analysis(
        allocator: &'a Bump,
        source: String,
        options: TransformOptions,
        analysis: &'a Croquis,
    ) -> Self {
        let mut ctx = Self::new(allocator, source, options);
        ctx.analysis = Some(analysis);
        ctx
    }

    /// Set the analysis summary
    pub fn set_analysis(&mut self, analysis: &'a Croquis) {
        self.analysis = Some(analysis);
    }

    /// Get the analysis summary if available
    #[inline]
    pub fn analysis(&self) -> Option<&Croquis> {
        self.analysis
    }

    /// Check if analysis data is available
    #[inline]
    pub fn has_analysis(&self) -> bool {
        self.analysis.is_some()
    }

    /// Check if a variable is defined (from analysis or binding metadata)
    ///
    /// This checks both the scope chain (template-local) and script bindings.
    pub fn is_variable_defined(&self, name: &str) -> bool {
        // First check scope chain (v-for, v-slot variables)
        if self.scope_chain.is_defined(name) {
            return true;
        }

        // Check analysis summary if available
        if let Some(analysis) = &self.analysis {
            return analysis.is_defined(name);
        }

        // Fall back to binding metadata from options
        if let Some(metadata) = &self.options.binding_metadata {
            return metadata.bindings.contains_key(name);
        }

        false
    }

    /// Get the binding type for a name
    pub fn get_binding_type(&self, name: &str) -> Option<BindingType> {
        // First check scope chain
        if let Some((_, binding)) = self.scope_chain.lookup(name) {
            return Some(binding.binding_type);
        }

        // Check analysis summary
        if let Some(analysis) = &self.analysis {
            return analysis.get_binding_type(name);
        }

        // Fall back to binding metadata
        if let Some(metadata) = &self.options.binding_metadata {
            return metadata.bindings.get(name).copied();
        }

        None
    }

    /// Check if a name needs $setup prefix (for script setup bindings)
    ///
    /// Returns true if the name is a script binding and should use $setup prefix.
    pub fn needs_setup_prefix(&self, name: &str) -> bool {
        // Skip if in scope (v-for, v-slot)
        if self.scope_chain.is_defined(name) {
            return false;
        }

        // Check binding type
        if let Some(binding_type) = self.get_binding_type(name) {
            matches!(
                binding_type,
                BindingType::SetupConst
                    | BindingType::SetupLet
                    | BindingType::SetupRef
                    | BindingType::SetupMaybeRef
                    | BindingType::SetupReactiveConst
            )
        } else {
            false
        }
    }

    /// Check if a binding is a ref that needs .value in script
    pub fn is_ref(&self, name: &str) -> bool {
        if let Some(analysis) = &self.analysis {
            analysis.bindings.is_ref(name)
        } else if let Some(binding_type) = self.get_binding_type(name) {
            matches!(
                binding_type,
                BindingType::SetupRef | BindingType::SetupMaybeRef
            )
        } else {
            false
        }
    }

    /// Check if a binding is from props
    pub fn is_prop(&self, name: &str) -> bool {
        if let Some(analysis) = &self.analysis {
            analysis.bindings.is_prop(name)
        } else {
            matches!(
                self.get_binding_type(name),
                Some(BindingType::Props | BindingType::PropsAliased)
            )
        }
    }

    /// Check if a component is registered (from analysis or binding metadata)
    pub fn is_component_registered(&self, name: &str) -> bool {
        if let Some(analysis) = &self.analysis {
            return analysis.is_component_registered(name);
        }

        // Fall back to checking binding metadata
        if let Some(metadata) = &self.options.binding_metadata {
            return metadata.bindings.contains_key(name);
        }

        false
    }

    /// Add a helper
    pub fn helper(&mut self, helper: RuntimeHelper) {
        self.helpers.insert(helper);
    }

    /// Remove a helper
    pub fn remove_helper(&mut self, helper: RuntimeHelper) {
        self.helpers.remove(&helper);
    }

    /// Check if helper exists
    pub fn has_helper(&self, helper: RuntimeHelper) -> bool {
        self.helpers.contains(&helper)
    }

    /// Add a component (maintains insertion order for code generation)
    pub fn add_component(&mut self, component: impl Into<String>) {
        let component = component.into();
        if !self.components.contains(&component) {
            self.components.push(component);
        }
    }

    /// Add a directive (maintains insertion order for code generation)
    pub fn add_directive(&mut self, directive: impl Into<String>) {
        let directive = directive.into();
        if !self.directives.contains(&directive) {
            self.directives.push(directive);
        }
    }

    /// Add an identifier to current scope
    pub fn add_identifier(&mut self, id: impl Into<CompactString>) {
        self.scope_chain
            .add_binding(id.into(), ScopeBinding::new(BindingType::SetupConst, 0));
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self, kind: ScopeKind) {
        self.scope_chain.enter_scope(kind);
    }

    /// Exit the current scope
    pub fn exit_scope(&mut self) {
        self.scope_chain.exit_scope();
    }

    /// Enter a v-for scope with the given aliases
    pub fn enter_v_for_scope(
        &mut self,
        value_alias: Option<&str>,
        key_alias: Option<&str>,
        index_alias: Option<&str>,
        source: &str,
    ) {
        self.scope_chain.enter_v_for_scope(
            VForScopeData {
                value_alias: CompactString::new(value_alias.unwrap_or("")),
                key_alias: key_alias.map(CompactString::new),
                index_alias: index_alias.map(CompactString::new),
                source: CompactString::new(source),
                key_expression: None,
            },
            0,
            0,
        );
    }

    /// Check if identifier is in scope
    pub fn is_in_scope(&self, id: &str) -> bool {
        self.scope_chain.is_defined(id)
    }

    /// Hoist an expression
    pub fn hoist(&mut self, node: JsChildNode<'a>) -> usize {
        let index = self.hoists.len();
        self.hoists.push(Some(node));
        index
    }

    /// Cache an expression
    pub fn cache(&mut self, exp: CacheExpression<'a>) -> usize {
        let index = self.cached.len();
        let boxed = Box::new_in(exp, self.allocator);
        self.cached.push(Some(boxed));
        index
    }

    /// Report an error
    pub fn on_error(&mut self, code: ErrorCode, loc: Option<SourceLocation>) {
        self.errors.push(CompilerError::new(code, loc));
    }

    /// Replace current node with a new node
    pub fn replace_node(&mut self, new_node: TemplateChildNode<'a>) {
        if let Some(parent) = &self.parent {
            let children = parent.children_mut();
            if self.child_index < children.len() {
                children[self.child_index] = new_node;
                self.current_node = Some(&mut children[self.child_index] as *mut _);
            }
        }
    }

    /// Take the current node, replacing it with a placeholder
    pub fn take_current_node(&mut self) -> Option<TemplateChildNode<'a>> {
        if let Some(parent) = &self.parent {
            let children = parent.children_mut();
            if self.child_index < children.len() {
                let placeholder = TemplateChildNode::Comment(Box::new_in(
                    CommentNode::new("", SourceLocation::STUB),
                    self.allocator,
                ));
                let taken = std::mem::replace(&mut children[self.child_index], placeholder);
                return Some(taken);
            }
        }
        None
    }

    /// Remove current node
    pub fn remove_node(&mut self) {
        if let Some(parent) = &self.parent {
            let children = parent.children_mut();
            if self.child_index < children.len() {
                children.remove(self.child_index);
                self.current_node = None;
                self.node_removed = true;
            }
        }
    }

    /// Remove a specific node
    pub fn remove_node_at(&mut self, index: usize) {
        if let Some(parent) = &self.parent {
            let children = parent.children_mut();
            if index < children.len() {
                children.remove(index);
                if index < self.child_index {
                    self.child_index -= 1;
                }
                self.node_removed = true;
            }
        }
    }

    /// Check if node was removed
    pub fn was_node_removed(&self) -> bool {
        self.node_removed
    }

    /// Reset node removed flag
    pub fn reset_node_removed(&mut self) {
        self.node_removed = false;
    }
}

/// Clone an expression into the arena
fn clone_expression<'a>(allocator: &'a Bump, exp: &ExpressionNode<'a>) -> ExpressionNode<'a> {
    match exp {
        ExpressionNode::Simple(s) => ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode {
                content: s.content.clone(),
                is_static: s.is_static,
                const_type: s.const_type,
                loc: s.loc.clone(),
                js_ast: None,
                hoisted: None,
                identifiers: None,
                is_handler_key: s.is_handler_key,
                is_ref_transformed: s.is_ref_transformed,
            },
            allocator,
        )),
        ExpressionNode::Compound(c) => {
            // For compound expressions, we recreate from source
            ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: c.loc.source.clone(),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: c.loc.clone(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: c.is_handler_key,
                    is_ref_transformed: false,
                },
                allocator,
            ))
        }
    }
}

/// Transform the root AST node
pub fn transform<'a>(allocator: &'a Bump, root: &mut RootNode<'a>, options: TransformOptions) {
    let source = root.source.clone();
    let mut ctx = TransformContext::new(allocator, source, options);
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

/// Traverse children of a parent node
fn traverse_children<'a>(ctx: &mut TransformContext<'a>, parent: ParentNode<'a>) {
    let children = parent.children_mut();
    let mut i = 0;

    while i < children.len() {
        ctx.grandparent = ctx.parent;
        ctx.parent = Some(parent);
        ctx.child_index = i;
        ctx.reset_node_removed();

        traverse_node(ctx, &mut children[i]);

        if ctx.was_node_removed() {
            // Node was removed, don't increment i
        } else {
            i += 1;
        }
    }
}

/// Traverse a single node
fn traverse_node<'a>(ctx: &mut TransformContext<'a>, node: &mut TemplateChildNode<'a>) {
    ctx.current_node = Some(node as *mut _);

    // Collect exit functions from transforms
    let mut exit_fns: std::vec::Vec<ExitFn<'a>> = std::vec::Vec::new();

    // Apply node transforms based on node type
    match node {
        TemplateChildNode::Element(el) => {
            // Check for structural directives first
            let structural_result = check_structural_directive(el);

            if let Some((dir_name, exp, exp_loc)) = structural_result {
                // Remove the directive from props
                remove_structural_directive(el, &dir_name);

                // Handle the structural directive
                match dir_name.as_str() {
                    "if" => {
                        if let Some(exits) = transform_v_if(ctx, exp.as_ref(), exp_loc, true) {
                            exit_fns.extend(exits);
                        }
                    }
                    "else-if" | "else" => {
                        if let Some(exits) = transform_v_if(ctx, exp.as_ref(), exp_loc, false) {
                            exit_fns.extend(exits);
                        }
                    }
                    "for" => {
                        if let Some(exits) = transform_v_for(ctx, exp.as_ref(), exp_loc) {
                            exit_fns.extend(exits);
                        }
                    }
                    _ => {}
                }

                // If node was replaced (e.g., by v-if transform), we need to traverse the new node
                if let Some(current_ptr) = ctx.current_node {
                    let current = unsafe { &mut *current_ptr };
                    match current {
                        TemplateChildNode::If(if_node) => {
                            // Traverse if branches that were just created
                            for i in 0..if_node.branches.len() {
                                let branch_ptr = &mut if_node.branches[i] as *mut IfBranchNode<'a>;
                                traverse_children(ctx, ParentNode::IfBranch(branch_ptr));
                            }
                            // Run exit functions and return early
                            for exit_fn in exit_fns.into_iter().rev() {
                                exit_fn(ctx);
                            }
                            return;
                        }
                        TemplateChildNode::For(for_node) => {
                            // Enter v-for scope with aliases
                            let value = for_node.value_alias.as_ref().and_then(|e| {
                                if let ExpressionNode::Simple(exp) = e {
                                    Some(exp.content.as_str())
                                } else {
                                    None
                                }
                            });
                            let key = for_node.key_alias.as_ref().and_then(|e| {
                                if let ExpressionNode::Simple(exp) = e {
                                    Some(exp.content.as_str())
                                } else {
                                    None
                                }
                            });
                            let index = for_node.object_index_alias.as_ref().and_then(|e| {
                                if let ExpressionNode::Simple(exp) = e {
                                    Some(exp.content.as_str())
                                } else {
                                    None
                                }
                            });
                            let source = match &for_node.source {
                                ExpressionNode::Simple(exp) => exp.content.as_str(),
                                ExpressionNode::Compound(c) => c.loc.source.as_str(),
                            };
                            ctx.enter_v_for_scope(value, key, index, source);

                            // Traverse for children
                            let for_ptr = for_node.as_mut() as *mut ForNode<'a>;
                            traverse_children(ctx, ParentNode::For(for_ptr));

                            // Exit v-for scope
                            ctx.exit_scope();

                            // Add helpers
                            ctx.helper(RuntimeHelper::RenderList);
                            ctx.helper(RuntimeHelper::Fragment);

                            // Run exit functions and return early
                            for exit_fn in exit_fns.into_iter().rev() {
                                exit_fn(ctx);
                            }
                            return;
                        }
                        TemplateChildNode::Element(el) => {
                            // Still an element, process it
                            if let Some(exits) = transform_element(ctx, el) {
                                exit_fns.extend(exits);
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Node was removed, return early
                    return;
                }
            } else {
                // No structural directive, process element normally
                if let Some(exits) = transform_element(ctx, el) {
                    exit_fns.extend(exits);
                }
            }
        }
        TemplateChildNode::Interpolation(interp) => {
            transform_interpolation(ctx, interp);
        }
        TemplateChildNode::Text(_) => {
            ctx.helper(RuntimeHelper::CreateText);
        }
        TemplateChildNode::Comment(_) => {
            ctx.helper(RuntimeHelper::CreateComment);
        }
        TemplateChildNode::If(if_node) => {
            // Traverse if branches
            for i in 0..if_node.branches.len() {
                let branch_ptr = &mut if_node.branches[i] as *mut IfBranchNode<'a>;
                traverse_children(ctx, ParentNode::IfBranch(branch_ptr));
            }
        }
        TemplateChildNode::For(for_node) => {
            // Enter v-for scope with aliases
            let value = for_node.value_alias.as_ref().and_then(|e| {
                if let ExpressionNode::Simple(exp) = e {
                    Some(exp.content.as_str())
                } else {
                    None
                }
            });
            let key = for_node.key_alias.as_ref().and_then(|e| {
                if let ExpressionNode::Simple(exp) = e {
                    Some(exp.content.as_str())
                } else {
                    None
                }
            });
            let index = for_node.object_index_alias.as_ref().and_then(|e| {
                if let ExpressionNode::Simple(exp) = e {
                    Some(exp.content.as_str())
                } else {
                    None
                }
            });
            let source = match &for_node.source {
                ExpressionNode::Simple(exp) => exp.content.as_str(),
                ExpressionNode::Compound(c) => c.loc.source.as_str(),
            };
            ctx.enter_v_for_scope(value, key, index, source);

            // Traverse for children
            let for_ptr = for_node.as_mut() as *mut ForNode<'a>;
            traverse_children(ctx, ParentNode::For(for_ptr));

            // Exit v-for scope
            ctx.exit_scope();

            // Add helpers
            ctx.helper(RuntimeHelper::RenderList);
            ctx.helper(RuntimeHelper::Fragment);
        }
        _ => {}
    }

    // Traverse children for element nodes
    if let TemplateChildNode::Element(el) = node {
        let el_ptr = el.as_mut() as *mut ElementNode<'a>;
        traverse_children(ctx, ParentNode::Element(el_ptr));
    }

    // Call exit functions in reverse order
    ctx.current_node = Some(node as *mut _);
    for exit_fn in exit_fns.into_iter().rev() {
        exit_fn(ctx);
    }
}

/// Check if element has a structural directive
fn check_structural_directive<'a>(
    el: &ElementNode<'a>,
) -> Option<(
    String,
    Option<SimpleExpressionContent>,
    Option<SourceLocation>,
)> {
    for prop in el.props.iter() {
        if let PropNode::Directive(dir) = prop {
            match dir.name.as_str() {
                "if" | "else-if" | "else" | "for" => {
                    let exp_content = dir.exp.as_ref().map(|e| match e {
                        ExpressionNode::Simple(s) => SimpleExpressionContent {
                            content: s.content.clone(),
                            is_static: s.is_static,
                            loc: s.loc.clone(),
                        },
                        ExpressionNode::Compound(c) => SimpleExpressionContent {
                            content: c.loc.source.clone(),
                            is_static: false,
                            loc: c.loc.clone(),
                        },
                    });
                    let exp_loc = dir.exp.as_ref().map(|e| e.loc().clone());
                    return Some((dir.name.clone(), exp_content, exp_loc));
                }
                _ => {}
            }
        }
    }
    None
}

/// Simple expression content for passing between functions
struct SimpleExpressionContent {
    content: String,
    is_static: bool,
    loc: SourceLocation,
}

/// Extract and remove key prop from element
fn extract_key_prop<'a>(el: &mut ElementNode<'a>) -> Option<PropNode<'a>> {
    let mut key_index = None;
    for (i, prop) in el.props.iter().enumerate() {
        match prop {
            PropNode::Attribute(attr) if attr.name == "key" => {
                key_index = Some(i);
                break;
            }
            PropNode::Directive(dir) if dir.name == "bind" => {
                if let Some(ExpressionNode::Simple(arg)) = &dir.arg {
                    if arg.content == "key" {
                        key_index = Some(i);
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    key_index.map(|i| el.props.remove(i))
}

/// Remove structural directive from element props
fn remove_structural_directive<'a>(el: &mut Box<'a, ElementNode<'a>>, dir_name: &str) {
    let mut i = 0;
    while i < el.props.len() {
        if let PropNode::Directive(dir) = &el.props[i] {
            if dir.name.as_str() == dir_name {
                el.props.remove(i);
                return;
            }
        }
        i += 1;
    }
}

/// Transform v-if directive
fn transform_v_if<'a>(
    ctx: &mut TransformContext<'a>,
    exp: Option<&SimpleExpressionContent>,
    _exp_loc: Option<SourceLocation>,
    is_root: bool,
) -> Option<std::vec::Vec<ExitFn<'a>>> {
    let allocator = ctx.allocator;

    if is_root {
        // Take the current element from parent
        let taken = ctx.take_current_node();
        let taken_node = taken?;

        // Get element info before moving
        let (element_loc, is_template_if) = match &taken_node {
            TemplateChildNode::Element(el) => {
                (el.loc.clone(), el.tag_type == ElementType::Template)
            }
            _ => return None,
        };

        // Create condition expression and process it for identifier prefixing
        let condition = exp.map(|e| {
            let raw_exp = ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: e.content.clone(),
                    is_static: e.is_static,
                    const_type: if e.is_static {
                        ConstantType::CanStringify
                    } else {
                        ConstantType::NotConstant
                    },
                    loc: e.loc.clone(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            ));
            // Process expression to add $setup. prefix
            if ctx.options.prefix_identifiers || ctx.options.is_ts {
                crate::transforms::transform_expression::process_expression(ctx, &raw_exp, false)
            } else {
                raw_exp
            }
        });

        // Extract user key from the element if present
        let mut user_key = None;
        let taken_node = match taken_node {
            TemplateChildNode::Element(mut el) => {
                user_key = extract_key_prop(&mut el);
                TemplateChildNode::Element(el)
            }
            other => other,
        };

        // Create branch with the taken element
        let mut branch_children = Vec::new_in(allocator);
        branch_children.push(taken_node);

        let branch = IfBranchNode {
            condition,
            children: branch_children,
            user_key,
            is_template_if,
            loc: element_loc.clone(),
        };

        let mut branches = Vec::new_in(allocator);
        branches.push(branch);

        let if_node = IfNode {
            branches,
            codegen_node: None,
            loc: element_loc,
        };

        // Replace placeholder with IfNode
        ctx.replace_node(TemplateChildNode::If(Box::new_in(if_node, allocator)));

        // Add helpers
        ctx.helper(RuntimeHelper::OpenBlock);
        ctx.helper(RuntimeHelper::CreateBlock);
        ctx.helper(RuntimeHelper::Fragment);
        ctx.helper(RuntimeHelper::CreateComment);

        None
    } else {
        // Find previous v-if node and add branch to it
        let child_index = ctx.child_index;

        // First, find the if node index
        let found_if_idx = if let Some(parent) = &ctx.parent {
            let children = parent.children_mut();
            let mut found = None;

            // Look backwards for v-if node
            for j in (0..child_index).rev() {
                match &children[j] {
                    TemplateChildNode::If(_) => {
                        found = Some(j);
                        break;
                    }
                    TemplateChildNode::Comment(_) => continue,
                    TemplateChildNode::Text(t) if t.content.trim().is_empty() => continue,
                    _ => break,
                }
            }
            found
        } else {
            None
        };

        if let Some(if_idx) = found_if_idx {
            // Take current element
            let taken = ctx.take_current_node();
            let taken_node = taken?;

            let (element_loc, is_template_if) = match &taken_node {
                TemplateChildNode::Element(el) => {
                    (el.loc.clone(), el.tag_type == ElementType::Template)
                }
                _ => return None,
            };

            // Create condition for else-if, None for else
            let condition = exp.map(|e| {
                let raw_exp = ExpressionNode::Simple(Box::new_in(
                    SimpleExpressionNode {
                        content: e.content.clone(),
                        is_static: e.is_static,
                        const_type: if e.is_static {
                            ConstantType::CanStringify
                        } else {
                            ConstantType::NotConstant
                        },
                        loc: e.loc.clone(),
                        js_ast: None,
                        hoisted: None,
                        identifiers: None,
                        is_handler_key: false,
                        is_ref_transformed: false,
                    },
                    allocator,
                ));
                // Process expression to add $setup. prefix
                if ctx.options.prefix_identifiers || ctx.options.is_ts {
                    crate::transforms::transform_expression::process_expression(
                        ctx, &raw_exp, false,
                    )
                } else {
                    raw_exp
                }
            });

            // Extract user key from the element if present
            let mut user_key = None;
            let taken_node = match taken_node {
                TemplateChildNode::Element(mut el) => {
                    user_key = extract_key_prop(&mut el);
                    TemplateChildNode::Element(el)
                }
                other => other,
            };

            // Create new branch
            let mut branch_children = Vec::new_in(allocator);
            branch_children.push(taken_node);

            let branch = IfBranchNode {
                condition,
                children: branch_children,
                user_key,
                is_template_if,
                loc: element_loc,
            };

            // Add branch to if node and traverse its children
            // Save context state before traversing (traverse_children modifies parent)
            let saved_parent = ctx.parent;
            let saved_grandparent = ctx.grandparent;
            let saved_child_index = ctx.child_index;

            if let Some(parent) = &ctx.parent {
                let children = parent.children_mut();
                if let TemplateChildNode::If(if_node) = &mut children[if_idx] {
                    if_node.branches.push(branch);
                    // Traverse the newly added branch to process components in it
                    let branch_idx = if_node.branches.len() - 1;
                    let branch_ptr = &mut if_node.branches[branch_idx] as *mut IfBranchNode<'a>;
                    traverse_children(ctx, ParentNode::IfBranch(branch_ptr));
                }
            }

            // Restore context state before removing node
            ctx.parent = saved_parent;
            ctx.grandparent = saved_grandparent;
            ctx.child_index = saved_child_index;

            // Remove the placeholder we left
            ctx.remove_node();
        } else {
            ctx.on_error(ErrorCode::VElseNoAdjacentIf, None);
        }

        None
    }
}

/// Transform v-for directive
fn transform_v_for<'a>(
    ctx: &mut TransformContext<'a>,
    exp: Option<&SimpleExpressionContent>,
    _exp_loc: Option<SourceLocation>,
) -> Option<std::vec::Vec<ExitFn<'a>>> {
    let allocator = ctx.allocator;

    let Some(exp) = exp else {
        ctx.on_error(ErrorCode::VForNoExpression, None);
        return None;
    };

    // Take the current element from parent
    let taken = ctx.take_current_node();
    let taken_node = taken?;

    let element_loc = match &taken_node {
        TemplateChildNode::Element(el) => el.loc.clone(),
        _ => return None,
    };

    // Parse v-for expression: "item in items" or "(item, index) in items"
    let (mut source, value_alias, key_alias, index_alias) =
        parse_v_for_expression(allocator, &exp.content, &exp.loc);

    // Process source expression to add _ctx. prefix if needed
    if ctx.options.prefix_identifiers || ctx.options.is_ts {
        use crate::transforms::transform_expression::prefix_identifiers_in_expression;
        if let ExpressionNode::Simple(ref mut source_exp) = source {
            let processed = prefix_identifiers_in_expression(&source_exp.content);
            source_exp.content = processed.into();
        }
    }

    // Create ForNode children with taken element
    let mut for_children = Vec::new_in(allocator);
    for_children.push(taken_node);

    // Create parse result (clone expressions for parse_result)
    let parse_result = ForParseResult {
        source: clone_expression(allocator, &source),
        value: value_alias.as_ref().map(|e| clone_expression(allocator, e)),
        key: key_alias.as_ref().map(|e| clone_expression(allocator, e)),
        index: index_alias.as_ref().map(|e| clone_expression(allocator, e)),
        finalized: false,
    };

    let for_node = ForNode {
        source,
        value_alias,
        key_alias,
        object_index_alias: index_alias,
        parse_result,
        children: for_children,
        codegen_node: None,
        loc: element_loc,
    };

    // Replace placeholder with ForNode
    ctx.replace_node(TemplateChildNode::For(Box::new_in(for_node, allocator)));

    // Add helpers
    ctx.helper(RuntimeHelper::RenderList);
    ctx.helper(RuntimeHelper::OpenBlock);
    ctx.helper(RuntimeHelper::CreateBlock);
    ctx.helper(RuntimeHelper::CreateElementBlock);
    ctx.helper(RuntimeHelper::Fragment);

    None
}

/// Parse v-for expression
fn parse_v_for_expression<'a>(
    allocator: &'a Bump,
    content: &str,
    loc: &SourceLocation,
) -> (
    ExpressionNode<'a>,
    Option<ExpressionNode<'a>>,
    Option<ExpressionNode<'a>>,
    Option<ExpressionNode<'a>>,
) {
    // Match patterns like "item in items" or "(item, index) in items"
    let (alias_part, source_part) = if let Some(idx) = content.find(" in ") {
        (&content[..idx], &content[idx + 4..])
    } else if let Some(idx) = content.find(" of ") {
        (&content[..idx], &content[idx + 4..])
    } else {
        // Return source as-is
        let source = ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode {
                content: String::new(content),
                is_static: false,
                const_type: ConstantType::NotConstant,
                loc: loc.clone(),
                js_ast: None,
                hoisted: None,
                identifiers: None,
                is_handler_key: false,
                is_ref_transformed: false,
            },
            allocator,
        ));
        return (source, None, None, None);
    };

    let source_str = source_part.trim();
    let alias_str = alias_part.trim();

    // Parse source expression
    let source = ExpressionNode::Simple(Box::new_in(
        SimpleExpressionNode {
            content: String::new(source_str),
            is_static: false,
            const_type: ConstantType::NotConstant,
            loc: SourceLocation::default(),
            js_ast: None,
            hoisted: None,
            identifiers: None,
            is_handler_key: false,
            is_ref_transformed: false,
        },
        allocator,
    ));

    // Parse aliases
    let (value, key, index) = if alias_str.starts_with('(') && alias_str.ends_with(')') {
        let inner = &alias_str[1..alias_str.len() - 1];
        let aliases: std::vec::Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

        let value = if !aliases.is_empty() && !aliases[0].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(aliases[0]),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: SourceLocation::default(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            )))
        } else {
            None
        };

        let key = if aliases.len() > 1 && !aliases[1].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(aliases[1]),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: SourceLocation::default(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            )))
        } else {
            None
        };

        let index = if aliases.len() > 2 && !aliases[2].is_empty() {
            Some(ExpressionNode::Simple(Box::new_in(
                SimpleExpressionNode {
                    content: String::new(aliases[2]),
                    is_static: false,
                    const_type: ConstantType::NotConstant,
                    loc: SourceLocation::default(),
                    js_ast: None,
                    hoisted: None,
                    identifiers: None,
                    is_handler_key: false,
                    is_ref_transformed: false,
                },
                allocator,
            )))
        } else {
            None
        };

        (value, key, index)
    } else {
        // Simple alias
        let value = Some(ExpressionNode::Simple(Box::new_in(
            SimpleExpressionNode {
                content: String::new(alias_str),
                is_static: false,
                const_type: ConstantType::NotConstant,
                loc: SourceLocation::default(),
                js_ast: None,
                hoisted: None,
                identifiers: None,
                is_handler_key: false,
                is_ref_transformed: false,
            },
            allocator,
        )));
        (value, None, None)
    };

    (source, value, key, index)
}

/// Transform element node
fn transform_element<'a>(
    ctx: &mut TransformContext<'a>,
    el: &mut Box<'a, ElementNode<'a>>,
) -> Option<std::vec::Vec<ExitFn<'a>>> {
    // Process props and directives
    process_element_props(ctx, el);

    // Determine helpers based on element type
    match el.tag_type {
        ElementType::Element => {
            ctx.helper(RuntimeHelper::CreateElementVNode);
        }
        ElementType::Component => {
            ctx.helper(RuntimeHelper::CreateVNode);
            // Only add ResolveComponent if component is not in binding metadata
            let is_in_bindings = ctx
                .options
                .binding_metadata
                .as_ref()
                .map(|m| m.bindings.contains_key(el.tag.as_str()))
                .unwrap_or(false);
            if !is_in_bindings {
                ctx.helper(RuntimeHelper::ResolveComponent);
            }
            ctx.add_component(el.tag.clone());
        }
        ElementType::Slot => {
            ctx.helper(RuntimeHelper::RenderSlot);
        }
        ElementType::Template => {
            ctx.helper(RuntimeHelper::Fragment);
        }
    }

    None
}

/// Process directive expressions with _ctx prefix
fn process_directive_expressions<'a>(
    ctx: &mut TransformContext<'a>,
    el: &mut Box<'a, ElementNode<'a>>,
) {
    use crate::transforms::transform_expression::{process_expression, process_inline_handler};

    for prop in el.props.iter_mut() {
        if let PropNode::Directive(dir) = prop {
            match dir.name.as_str() {
                "bind" | "show" | "if" | "else-if" | "for" | "memo" => {
                    // Process value expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_expression(ctx, exp, false);
                        dir.exp = Some(processed);
                    }
                }
                "on" => {
                    // Process event handler expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_inline_handler(ctx, exp);
                        dir.exp = Some(processed);
                    }
                }
                "model" => {
                    // Process v-model expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_expression(ctx, exp, false);
                        dir.exp = Some(processed);
                    }
                }
                _ => {
                    // Custom directives - process value expression
                    if let Some(exp) = &dir.exp {
                        let processed = process_expression(ctx, exp, false);
                        dir.exp = Some(processed);
                    }
                    // Process dynamic argument
                    if let Some(arg) = &dir.arg {
                        if let ExpressionNode::Simple(simple_arg) = arg {
                            if !simple_arg.is_static {
                                let processed = process_expression(ctx, arg, false);
                                dir.arg = Some(processed);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Process element properties and directives
fn process_element_props<'a>(ctx: &mut TransformContext<'a>, el: &mut Box<'a, ElementNode<'a>>) {
    let allocator = ctx.allocator;
    let is_component = el.tag_type == ElementType::Component;

    // Process directive expressions with _ctx prefix if needed
    if ctx.options.prefix_identifiers || ctx.options.is_ts {
        process_directive_expressions(ctx, el);
    }

    // Collect indices of v-model directives to process
    let mut model_indices: std::vec::Vec<usize> = std::vec::Vec::new();
    for (i, prop) in el.props.iter().enumerate() {
        if let PropNode::Directive(dir) = prop {
            match dir.name.as_str() {
                "model" => {
                    model_indices.push(i);
                }
                "slot" => {
                    ctx.helper(RuntimeHelper::RenderSlot);
                }
                // v-show is a built-in directive - it uses vShow helper directly
                // No need to add to ctx.directives (which would use resolveDirective)
                "show" => {}
                // Handle custom directives - register them for resolveDirective
                _ if !is_builtin_directive(&dir.name) => {
                    ctx.helper(RuntimeHelper::WithDirectives);
                    ctx.helper(RuntimeHelper::ResolveDirective);
                    ctx.add_directive(dir.name.clone());
                }
                _ => {}
            }
        }
    }

    // Collect all v-model data first, then modify props
    struct VModelData {
        idx: usize,
        value_exp: String,
        prop_name: String,
        event_name: std::string::String,
        handler: std::string::String,
        dir_loc: SourceLocation,
        modifiers_obj: Option<std::string::String>,
        modifiers_key: Option<String>,
        is_dynamic: bool,
    }

    let mut vmodel_data: std::vec::Vec<VModelData> = std::vec::Vec::new();

    for &idx in model_indices.iter() {
        if let Some(PropNode::Directive(dir)) = el.props.get(idx) {
            // Get value expression
            let value_exp = match &dir.exp {
                Some(ExpressionNode::Simple(s)) => s.content.clone(),
                Some(ExpressionNode::Compound(c)) => c.loc.source.clone(),
                None => continue,
            };

            // Check if arg is dynamic
            let is_dynamic = dir.arg.as_ref().is_some_and(|arg| match arg {
                ExpressionNode::Simple(exp) => !exp.is_static,
                ExpressionNode::Compound(_) => true,
            });

            // Get prop name (default: modelValue for components, value for inputs)
            let prop_name = dir
                .arg
                .as_ref()
                .map(|arg| match arg {
                    ExpressionNode::Simple(exp) => exp.content.clone(),
                    ExpressionNode::Compound(exp) => exp.loc.source.clone(),
                })
                .unwrap_or_else(|| {
                    if is_component {
                        String::new("modelValue")
                    } else {
                        String::new("value")
                    }
                });

            // Create event name
            let event_name = if is_component {
                format!("onUpdate:{}", prop_name)
            } else {
                // For native elements, use input event (or change for lazy)
                let has_lazy = dir.modifiers.iter().any(|m| m.content == "lazy");
                if has_lazy {
                    "onChange".to_string()
                } else {
                    "onInput".to_string()
                }
            };

            // Build handler expression
            let handler = if is_component {
                format!("$event => (({}) = $event)", value_exp)
            } else {
                // For native elements, check modifiers
                let has_number = dir.modifiers.iter().any(|m| m.content == "number");
                let has_trim = dir.modifiers.iter().any(|m| m.content == "trim");

                let mut target_value = "$event.target.value".to_string();
                if has_trim {
                    target_value = format!("{}.trim()", target_value);
                }
                if has_number {
                    target_value = format!("_toNumber({})", target_value);
                }
                format!("$event => (({}) = {})", value_exp, target_value)
            };

            let dir_loc = dir.loc.clone();

            // Collect modifiers info for components
            let (modifiers_obj, modifiers_key) = if is_component && !dir.modifiers.is_empty() {
                let modifiers_content: std::vec::Vec<std::string::String> = dir
                    .modifiers
                    .iter()
                    .map(|m| format!("{}: true", m.content))
                    .collect();
                let obj = format!("{{ {} }}", modifiers_content.join(", "));
                let key = if prop_name == "modelValue" {
                    String::new("modelModifiers")
                } else {
                    format!("{}Modifiers", prop_name).into()
                };
                (Some(obj), Some(key))
            } else {
                (None, None)
            };

            vmodel_data.push(VModelData {
                idx,
                value_exp,
                prop_name,
                event_name,
                handler,
                dir_loc,
                modifiers_obj,
                modifiers_key,
                is_dynamic,
            });
        }
    }

    if is_component {
        // Separate static and dynamic v-model data
        let static_vmodel: std::vec::Vec<_> =
            vmodel_data.iter().filter(|d| !d.is_dynamic).collect();
        let dynamic_vmodel: std::vec::Vec<_> =
            vmodel_data.iter().filter(|d| d.is_dynamic).collect();

        // For static v-model: remove directives and add generated props
        // First remove all static v-model directives in reverse order (to preserve indices)
        for data in static_vmodel.iter().rev() {
            el.props.remove(data.idx);
        }

        // Then add all generated props in forward order
        for data in static_vmodel.iter() {
            // Add :propName prop
            let value_prop = PropNode::Directive(Box::new_in(
                DirectiveNode {
                    name: String::new("bind"),
                    raw_name: None,
                    arg: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new(
                            data.prop_name.clone(),
                            true,
                            data.dir_loc.clone(),
                        ),
                        allocator,
                    ))),
                    exp: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new(
                            data.value_exp.clone(),
                            false,
                            data.dir_loc.clone(),
                        ),
                        allocator,
                    ))),
                    modifiers: Vec::new_in(allocator),
                    for_parse_result: None,
                    loc: data.dir_loc.clone(),
                },
                allocator,
            ));
            el.props.push(value_prop);

            // Add @update:propName prop
            let event_prop = PropNode::Directive(Box::new_in(
                DirectiveNode {
                    name: String::new("on"),
                    raw_name: None,
                    arg: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new(
                            &data.event_name[2..],
                            true,
                            data.dir_loc.clone(),
                        ), // Remove "on" prefix
                        allocator,
                    ))),
                    exp: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new(&data.handler, false, data.dir_loc.clone()),
                        allocator,
                    ))),
                    modifiers: Vec::new_in(allocator),
                    for_parse_result: None,
                    loc: data.dir_loc.clone(),
                },
                allocator,
            ));
            el.props.push(event_prop);

            // Add modelModifiers prop for components with modifiers
            if let (Some(modifiers_obj), Some(modifiers_key)) =
                (&data.modifiers_obj, &data.modifiers_key)
            {
                let modifiers_prop = PropNode::Directive(Box::new_in(
                    DirectiveNode {
                        name: String::new("bind"),
                        raw_name: None,
                        arg: Some(ExpressionNode::Simple(Box::new_in(
                            SimpleExpressionNode::new(
                                modifiers_key.clone(),
                                true,
                                data.dir_loc.clone(),
                            ),
                            allocator,
                        ))),
                        exp: Some(ExpressionNode::Simple(Box::new_in(
                            SimpleExpressionNode::new(modifiers_obj, false, data.dir_loc.clone()),
                            allocator,
                        ))),
                        modifiers: Vec::new_in(allocator),
                        for_parse_result: None,
                        loc: SourceLocation::STUB,
                    },
                    allocator,
                ));
                el.props.push(modifiers_prop);
            }
        }

        // For dynamic v-model: keep the directive (will be handled in codegen with normalizeProps)
        // Just mark that we have dynamic v-model by adding helper
        if !dynamic_vmodel.is_empty() {
            ctx.helper(RuntimeHelper::NormalizeProps);
        }
    } else {
        // For native elements: process in reverse order to preserve indices during insertion
        for data in vmodel_data.iter().rev() {
            // Keep v-model directive, insert onUpdate:modelValue handler right after it
            let handler = format!("$event => (({}) = $event)", data.value_exp);
            let event_prop = PropNode::Directive(Box::new_in(
                DirectiveNode {
                    name: String::new("on"),
                    raw_name: None,
                    arg: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new("update:modelValue", true, data.dir_loc.clone()),
                        allocator,
                    ))),
                    exp: Some(ExpressionNode::Simple(Box::new_in(
                        SimpleExpressionNode::new(&handler, false, data.dir_loc.clone()),
                        allocator,
                    ))),
                    modifiers: Vec::new_in(allocator),
                    for_parse_result: None,
                    loc: data.dir_loc.clone(),
                },
                allocator,
            ));
            // Insert right after the v-model directive to ensure proper ordering
            el.props.insert(data.idx + 1, event_prop);
        }
    }
}

/// Transform interpolation node
fn transform_interpolation<'a>(
    ctx: &mut TransformContext<'a>,
    interp: &mut Box<'a, InterpolationNode<'a>>,
) {
    ctx.helper(RuntimeHelper::ToDisplayString);

    // Process the expression to add _ctx. prefix and/or strip TypeScript if needed
    if ctx.options.prefix_identifiers || ctx.options.is_ts {
        use crate::transforms::transform_expression::process_expression;
        let processed = process_expression(ctx, &interp.content, false);
        interp.content = processed;
    }
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

        transform(&allocator, &mut root, TransformOptions::default());

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

        transform(&allocator, &mut root, TransformOptions::default());

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

        transform(&allocator, &mut root, TransformOptions::default());

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
        transform(&allocator, &mut root, TransformOptions::default());

        let result = generate(&root, CodegenOptions::default());
        println!("v-if codegen:\n{}", result.code);

        // Should contain openBlock and createBlock for v-if
        assert!(
            result.code.contains("openBlock"),
            "Should contain openBlock"
        );
    }
}
