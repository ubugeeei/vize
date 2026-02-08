//! TransformContext implementation.

use vize_carton::{Box, Bump, CompactString, String};
use vize_croquis::reactivity::ReactiveKind;
use vize_croquis::{BindingType, Croquis, ScopeBinding, ScopeKind, VForScopeData};

use crate::ast::*;
use crate::errors::{CompilerError, ErrorCode};
use crate::options::TransformOptions;

use super::TransformContext;

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
            helpers: vize_carton::FxHashSet::default(),
            components: std::vec::Vec::new(),
            directives: std::vec::Vec::new(),
            hoists: vize_carton::Vec::new_in(allocator),
            cached: vize_carton::Vec::new_in(allocator),
            temps: 0,
            scope_chain: vize_croquis::ScopeChain::new(),
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

    /// Get the ReactiveKind for a name (from Croquis ReactivityTracker)
    pub fn get_reactive_kind(&self, name: &str) -> Option<ReactiveKind> {
        self.analysis?.reactivity.lookup(name).map(|s| s.kind)
    }

    /// Check if a binding is read-only (Computed, Readonly, ShallowReadonly)
    pub fn is_readonly_binding(&self, name: &str) -> bool {
        self.get_reactive_kind(name).is_some_and(|k| {
            matches!(
                k,
                ReactiveKind::Computed | ReactiveKind::Readonly | ReactiveKind::ShallowReadonly
            )
        })
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
pub(super) fn clone_expression<'a>(
    allocator: &'a Bump,
    exp: &ExpressionNode<'a>,
) -> ExpressionNode<'a> {
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
