//! Scope analysis for Vue templates and scripts.
//!
//! Provides a hierarchical scope chain that tracks variable visibility
//! across different contexts (module, function, block, v-for, v-slot).

use rustc_hash::FxHashMap;
use vize_relief::BindingType;

/// Unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(u32);

impl ScopeId {
    /// The root scope (module level)
    pub const ROOT: Self = Self(0);

    /// Create a new scope ID
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Kind of scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScopeKind {
    /// Module-level scope (top-level of script setup)
    Module = 0,
    /// Function scope
    Function = 1,
    /// Block scope (if, for, etc.)
    Block = 2,
    /// v-for scope (template)
    VFor = 3,
    /// v-slot scope (template)
    VSlot = 4,
}

/// A single scope in the scope chain
#[derive(Debug)]
pub struct Scope {
    /// Unique identifier
    pub id: ScopeId,
    /// Parent scope (None for root)
    pub parent: Option<ScopeId>,
    /// Kind of scope
    pub kind: ScopeKind,
    /// Bindings declared in this scope
    bindings: FxHashMap<String, ScopeBinding>,
}

impl Scope {
    /// Create a new scope
    pub fn new(id: ScopeId, parent: Option<ScopeId>, kind: ScopeKind) -> Self {
        Self {
            id,
            parent,
            kind,
            bindings: FxHashMap::default(),
        }
    }

    /// Add a binding to this scope
    pub fn add_binding(&mut self, name: String, binding: ScopeBinding) {
        self.bindings.insert(name, binding);
    }

    /// Get a binding by name (only in this scope, not parents)
    pub fn get_binding(&self, name: &str) -> Option<&ScopeBinding> {
        self.bindings.get(name)
    }

    /// Check if this scope has a binding
    pub fn has_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Iterate over all bindings in this scope
    pub fn bindings(&self) -> impl Iterator<Item = (&str, &ScopeBinding)> {
        self.bindings.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// A binding within a scope
#[derive(Debug, Clone)]
pub struct ScopeBinding {
    /// The type of binding
    pub binding_type: BindingType,
    /// Source location of the declaration (offset in source)
    pub declaration_offset: u32,
    /// Whether this binding is used
    pub is_used: bool,
    /// Whether this binding is mutated
    pub is_mutated: bool,
}

impl ScopeBinding {
    /// Create a new scope binding
    pub fn new(binding_type: BindingType, declaration_offset: u32) -> Self {
        Self {
            binding_type,
            declaration_offset,
            is_used: false,
            is_mutated: false,
        }
    }
}

/// Manages the scope chain during analysis
#[derive(Debug)]
pub struct ScopeChain {
    /// All scopes (indexed by ScopeId)
    scopes: Vec<Scope>,
    /// Current scope ID
    current: ScopeId,
}

impl Default for ScopeChain {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeChain {
    /// Create a new scope chain with a root module scope
    pub fn new() -> Self {
        let root = Scope::new(ScopeId::ROOT, None, ScopeKind::Module);
        Self {
            scopes: vec![root],
            current: ScopeId::ROOT,
        }
    }

    /// Get the current scope
    pub fn current_scope(&self) -> &Scope {
        &self.scopes[self.current.as_u32() as usize]
    }

    /// Get the current scope mutably
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        &mut self.scopes[self.current.as_u32() as usize]
    }

    /// Get a scope by ID
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id.as_u32() as usize)
    }

    /// Enter a new scope
    pub fn enter_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let scope = Scope::new(id, Some(self.current), kind);
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Exit the current scope and return to parent
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.current_scope().parent {
            self.current = parent;
        }
    }

    /// Look up a binding by name, searching up the scope chain
    pub fn lookup(&self, name: &str) -> Option<(&Scope, &ScopeBinding)> {
        let mut scope_id = Some(self.current);

        while let Some(id) = scope_id {
            let scope = &self.scopes[id.as_u32() as usize];
            if let Some(binding) = scope.get_binding(name) {
                return Some((scope, binding));
            }
            scope_id = scope.parent;
        }

        None
    }

    /// Check if a name is defined in the current scope chain
    pub fn is_defined(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Add a binding to the current scope
    pub fn add_binding(&mut self, name: String, binding: ScopeBinding) {
        self.current_scope_mut().add_binding(name, binding);
    }

    /// Mark a binding as used
    pub fn mark_used(&mut self, name: &str) {
        let mut scope_id = Some(self.current);

        while let Some(id) = scope_id {
            let scope = &mut self.scopes[id.as_u32() as usize];
            if let Some(binding) = scope.bindings.get_mut(name) {
                binding.is_used = true;
                return;
            }
            scope_id = scope.parent;
        }
    }

    /// Mark a binding as mutated
    pub fn mark_mutated(&mut self, name: &str) {
        let mut scope_id = Some(self.current);

        while let Some(id) = scope_id {
            let scope = &mut self.scopes[id.as_u32() as usize];
            if let Some(binding) = scope.bindings.get_mut(name) {
                binding.is_mutated = true;
                return;
            }
            scope_id = scope.parent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_chain_basic() {
        let mut chain = ScopeChain::new();

        // Add binding to root scope
        chain.add_binding(
            "foo".to_string(),
            ScopeBinding::new(BindingType::SetupRef, 0),
        );

        assert!(chain.is_defined("foo"));
        assert!(!chain.is_defined("bar"));

        // Enter a new scope
        chain.enter_scope(ScopeKind::Function);
        chain.add_binding(
            "bar".to_string(),
            ScopeBinding::new(BindingType::SetupLet, 10),
        );

        // Can see both foo and bar
        assert!(chain.is_defined("foo"));
        assert!(chain.is_defined("bar"));

        // Exit scope
        chain.exit_scope();

        // Can only see foo now
        assert!(chain.is_defined("foo"));
        assert!(!chain.is_defined("bar"));
    }

    #[test]
    fn test_scope_shadowing() {
        let mut chain = ScopeChain::new();

        chain.add_binding("x".to_string(), ScopeBinding::new(BindingType::SetupRef, 0));

        chain.enter_scope(ScopeKind::Block);
        chain.add_binding(
            "x".to_string(),
            ScopeBinding::new(BindingType::SetupLet, 10),
        );

        // Should find the inner binding
        let (scope, binding) = chain.lookup("x").unwrap();
        assert_eq!(scope.kind, ScopeKind::Block);
        assert_eq!(binding.binding_type, BindingType::SetupLet);
    }
}
