//! Symbol table for tracking declarations and references.
//!
//! Provides efficient lookup and tracking of symbols across
//! the entire compilation unit.

use rustc_hash::FxHashMap;
use vize_relief::BindingType;

use crate::{ScopeBinding, ScopeId};

/// Unique identifier for a symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

impl SymbolId {
    /// Create a new symbol ID
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

/// A symbol in the symbol table
#[derive(Debug)]
pub struct Symbol {
    /// Unique identifier
    pub id: SymbolId,
    /// Symbol name
    pub name: String,
    /// The type of binding
    pub binding_type: BindingType,
    /// Scope where this symbol is declared
    pub scope_id: ScopeId,
    /// Source offset of declaration
    pub declaration_offset: u32,
    /// All references to this symbol (source offsets)
    pub references: Vec<u32>,
    /// Flags for the symbol
    pub flags: SymbolFlags,
}

impl Symbol {
    /// Create a new symbol
    pub fn new(
        id: SymbolId,
        name: String,
        binding_type: BindingType,
        scope_id: ScopeId,
        declaration_offset: u32,
    ) -> Self {
        Self {
            id,
            name,
            binding_type,
            scope_id,
            declaration_offset,
            references: Vec::new(),
            flags: SymbolFlags::empty(),
        }
    }

    /// Add a reference to this symbol
    pub fn add_reference(&mut self, offset: u32) {
        self.references.push(offset);
    }

    /// Check if this symbol is used
    pub fn is_used(&self) -> bool {
        !self.references.is_empty()
    }
}

bitflags::bitflags! {
    /// Flags for symbol properties
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SymbolFlags: u8 {
        /// Symbol is exported
        const EXPORTED = 1 << 0;
        /// Symbol is imported
        const IMPORTED = 1 << 1;
        /// Symbol is mutated after declaration
        const MUTATED = 1 << 2;
        /// Symbol is a component
        const COMPONENT = 1 << 3;
        /// Symbol is a directive
        const DIRECTIVE = 1 << 4;
        /// Symbol is from props
        const FROM_PROPS = 1 << 5;
        /// Symbol is from defineModel
        const FROM_MODEL = 1 << 6;
    }
}

/// Symbol table for the entire compilation unit
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// All symbols (indexed by SymbolId)
    symbols: Vec<Symbol>,
    /// Name to symbol ID mapping for quick lookup
    name_to_id: FxHashMap<String, SymbolId>,
}

impl SymbolTable {
    /// Create a new empty symbol table
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a symbol to the table
    pub fn add_symbol(
        &mut self,
        name: String,
        binding_type: BindingType,
        scope_id: ScopeId,
        declaration_offset: u32,
    ) -> SymbolId {
        let id = SymbolId::new(self.symbols.len() as u32);
        let symbol = Symbol::new(id, name.clone(), binding_type, scope_id, declaration_offset);
        self.symbols.push(symbol);
        self.name_to_id.insert(name, id);
        id
    }

    /// Get a symbol by ID
    pub fn get(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id.as_u32() as usize)
    }

    /// Get a symbol by ID mutably
    pub fn get_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(id.as_u32() as usize)
    }

    /// Look up a symbol by name
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.name_to_id.get(name).and_then(|id| self.get(*id))
    }

    /// Look up a symbol ID by name
    pub fn lookup_id(&self, name: &str) -> Option<SymbolId> {
        self.name_to_id.get(name).copied()
    }

    /// Add a reference to a symbol
    pub fn add_reference(&mut self, name: &str, offset: u32) -> bool {
        if let Some(&id) = self.name_to_id.get(name) {
            if let Some(symbol) = self.symbols.get_mut(id.as_u32() as usize) {
                symbol.add_reference(offset);
                return true;
            }
        }
        false
    }

    /// Iterate over all symbols
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }

    /// Get all unused symbols
    pub fn unused_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter().filter(|s| !s.is_used())
    }

    /// Get the number of symbols
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Check if the table is empty
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

/// Convert from ScopeBinding to add to symbol table
impl From<&ScopeBinding> for SymbolFlags {
    fn from(binding: &ScopeBinding) -> Self {
        let mut flags = SymbolFlags::empty();
        if binding.is_mutated {
            flags |= SymbolFlags::MUTATED;
        }
        if matches!(
            binding.binding_type,
            BindingType::Props | BindingType::PropsAliased
        ) {
            flags |= SymbolFlags::FROM_PROPS;
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table() {
        let mut table = SymbolTable::new();

        let id = table.add_symbol("count".to_string(), BindingType::SetupRef, ScopeId::ROOT, 0);

        assert!(table.lookup("count").is_some());
        assert!(table.lookup("unknown").is_none());

        // Add reference
        table.add_reference("count", 50);
        table.add_reference("count", 100);

        let symbol = table.get(id).unwrap();
        assert_eq!(symbol.references.len(), 2);
        assert!(symbol.is_used());
    }

    #[test]
    fn test_unused_symbols() {
        let mut table = SymbolTable::new();

        table.add_symbol("used".to_string(), BindingType::SetupRef, ScopeId::ROOT, 0);
        table.add_symbol(
            "unused".to_string(),
            BindingType::SetupRef,
            ScopeId::ROOT,
            10,
        );

        table.add_reference("used", 50);

        let unused: Vec<_> = table.unused_symbols().collect();
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].name, "unused");
    }
}
