//! Scope analysis for Vue templates and scripts.
//!
//! Provides a hierarchical scope chain that tracks variable visibility
//! across different contexts (module, function, block, v-for, v-slot).
//!
//! ## Performance Optimizations
//!
//! - Uses `CompactString` instead of `String` for identifier names (SSO for short strings)
//! - Uses `SmallVec` for parameter lists (stack-allocated for small counts)
//! - Bitflags for binding properties to reduce memory and improve cache locality
//! - `#[inline]` hints for hot path functions

use vize_carton::{bitflags, smallvec, CompactString, FxHashMap, SmallVec};
use vize_relief::BindingType;

/// Maximum parameters typically seen in v-for/v-slot/callbacks
/// Stack-allocated up to this count, heap-allocated beyond
const PARAM_INLINE_CAP: usize = 4;

/// Type alias for parameter name lists (stack-allocated for small counts)
pub type ParamNames = SmallVec<[CompactString; PARAM_INLINE_CAP]>;

/// Unique identifier for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ScopeId(u32);

impl ScopeId {
    /// The root scope (SFC level)
    pub const ROOT: Self = Self(0);

    /// Create a new scope ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Kind of scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScopeKind {
    /// SFC (Single File Component) level scope
    /// This is the root scope that contains script setup/non-script-setup scopes
    Module = 0,
    /// Function scope
    Function = 1,
    /// Block scope (if, for, etc.)
    Block = 2,
    /// v-for scope (template)
    VFor = 3,
    /// v-slot scope (template)
    VSlot = 4,
    /// Event handler scope (@click, etc.)
    EventHandler = 5,
    /// Callback/arrow function scope in expressions
    Callback = 6,
    /// Script setup scope (`<script setup>`)
    ScriptSetup = 7,
    /// Non-script setup scope (Options API, regular `<script>`)
    NonScriptSetup = 8,
    /// Universal scope (SSR - runs on both server and client)
    Universal = 9,
    /// Client-only scope (onMounted, onBeforeUnmount, etc.)
    ClientOnly = 10,
    /// Universal JavaScript global scope (console, Math, Object, Array, etc.)
    /// Works in all runtimes
    JsGlobalUniversal = 11,
    /// Browser-only JavaScript global scope (window, document, navigator, localStorage, etc.)
    /// WARNING: Not available in SSR server context
    JsGlobalBrowser = 12,
    /// Node.js-only JavaScript global scope (process, Buffer, __dirname, require, etc.)
    /// WARNING: Not available in browser context
    JsGlobalNode = 13,
    /// Deno-only JavaScript global scope (Deno namespace)
    JsGlobalDeno = 14,
    /// Bun-only JavaScript global scope (Bun namespace)
    JsGlobalBun = 15,
    /// Vue global scope ($refs, $emit, $slots, $attrs, etc.)
    VueGlobal = 16,
    /// External module scope (imported modules)
    ExternalModule = 17,
    /// Closure scope (function declaration, function expression, arrow function)
    /// Has access to arguments, this, and local variables
    Closure = 18,
}

impl ScopeKind {
    /// Get the display prefix for this scope kind
    /// - `~` = universal (works on both client and server)
    /// - `!` = client only (requires client API: window, document, etc.)
    /// - `#` = server private (reserved for future Server Components)
    #[inline]
    pub const fn prefix(&self) -> &'static str {
        match self {
            // Client-only (requires client API)
            Self::ClientOnly | Self::JsGlobalBrowser => "!",
            // Server private (reserved for future Server Components)
            Self::JsGlobalNode | Self::JsGlobalDeno | Self::JsGlobalBun => "#",
            // Universal (works on both)
            _ => "~",
        }
    }

    /// Get the display name for this scope kind
    #[inline]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Module => "mod",
            Self::Function => "fn",
            Self::Block => "block",
            Self::VFor => "v-for",
            Self::VSlot => "v-slot",
            Self::EventHandler => "event",
            Self::Callback => "cb",
            Self::ScriptSetup => "setup",
            Self::NonScriptSetup => "plain",
            Self::Universal => "universal",
            Self::ClientOnly => "client",
            Self::JsGlobalUniversal => "univ",
            Self::JsGlobalBrowser => "client",
            Self::JsGlobalNode => "server",
            Self::JsGlobalDeno => "server",
            Self::JsGlobalBun => "server",
            Self::VueGlobal => "vue",
            Self::ExternalModule => "extern",
            Self::Closure => "closure",
        }
    }

    /// Format for VIR display (zero allocation)
    #[inline]
    pub const fn to_display(&self) -> &'static str {
        match self {
            Self::Module => "mod",
            Self::Function => "fn",
            Self::Block => "block",
            Self::VFor => "v-for",
            Self::VSlot => "v-slot",
            Self::EventHandler => "event",
            Self::Callback => "cb",
            Self::ScriptSetup => "setup",
            Self::NonScriptSetup => "plain",
            Self::Universal => "universal",
            Self::ClientOnly => "client",
            Self::JsGlobalUniversal => "univ",
            Self::JsGlobalBrowser => "client",
            Self::JsGlobalNode => "server",
            Self::JsGlobalDeno => "server",
            Self::JsGlobalBun => "server",
            Self::VueGlobal => "vue",
            Self::ExternalModule => "extern",
            Self::Closure => "closure",
        }
    }

    /// Get reference prefix for parent scope references
    /// - `~` = universal (works on both client and server)
    /// - `!` = client only (requires client API)
    /// - `#` = server private (reserved for future Server Components)
    #[inline]
    pub const fn ref_prefix(&self) -> &'static str {
        match self {
            Self::ClientOnly | Self::JsGlobalBrowser => "!",
            Self::JsGlobalNode | Self::JsGlobalDeno | Self::JsGlobalBun => "#",
            _ => "~",
        }
    }
}

/// Data specific to v-for scope
#[derive(Debug, Clone)]
pub struct VForScopeData {
    /// The value alias (e.g., "item" in v-for="item in items")
    pub value_alias: CompactString,
    /// The key alias (e.g., "key" in v-for="(item, key) in items")
    pub key_alias: Option<CompactString>,
    /// The index alias (e.g., "index" in v-for="(item, index) in items")
    pub index_alias: Option<CompactString>,
    /// The source expression (e.g., "items")
    pub source: CompactString,
    /// The :key expression if present (e.g., "item.id")
    pub key_expression: Option<CompactString>,
}

/// Data specific to v-slot scope
#[derive(Debug, Clone)]
pub struct VSlotScopeData {
    /// Slot name
    pub name: CompactString,
    /// Props pattern (e.g., "{ item, index }" in v-slot="{ item, index }")
    pub props_pattern: Option<CompactString>,
    /// Extracted prop names (stack-allocated for typical cases)
    pub prop_names: ParamNames,
}

/// Data specific to event handler scope
#[derive(Debug, Clone)]
pub struct EventHandlerScopeData {
    /// Event name (e.g., "click")
    pub event_name: CompactString,
    /// Whether this handler has implicit $event
    pub has_implicit_event: bool,
    /// Explicit parameter names (stack-allocated for typical cases)
    pub param_names: ParamNames,
    /// The handler expression (e.g., "handleClick" or "handleClick($event)")
    pub handler_expression: Option<CompactString>,
}

/// Data specific to callback scope
#[derive(Debug, Clone)]
pub struct CallbackScopeData {
    /// Parameter names (stack-allocated for typical cases)
    pub param_names: ParamNames,
    /// Context description (for debugging)
    pub context: CompactString,
}

/// Data specific to script setup scope
#[derive(Debug, Clone)]
pub struct ScriptSetupScopeData {
    /// Whether this is TypeScript
    pub is_ts: bool,
    /// Whether async setup
    pub is_async: bool,
    /// Generic type parameter from `<script setup generic="T">`
    pub generic: Option<CompactString>,
}

/// Data specific to non-script-setup scope (Options API, regular script)
#[derive(Debug, Clone)]
pub struct NonScriptSetupScopeData {
    /// Whether this is TypeScript
    pub is_ts: bool,
    /// Whether using defineComponent
    pub has_define_component: bool,
}

/// Data specific to client-only scope (onMounted, onBeforeUnmount, etc.)
#[derive(Debug, Clone)]
pub struct ClientOnlyScopeData {
    /// The lifecycle hook name (e.g., "onMounted", "onBeforeUnmount")
    pub hook_name: CompactString,
}

/// Data specific to universal scope (SSR - runs on both server and client)
#[derive(Debug, Clone)]
pub struct UniversalScopeData {
    /// Context description
    pub context: CompactString,
}

/// Runtime environment for JavaScript globals
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JsRuntime {
    /// Universal - works in all runtimes (console, Math, Object, Array, JSON, etc.)
    Universal = 0,
    /// Browser - window, document, navigator, localStorage, etc.
    Browser = 1,
    /// Node.js - process, Buffer, __dirname, __filename, require, etc.
    Node = 2,
    /// Deno - Deno namespace
    Deno = 3,
    /// Bun - Bun namespace
    Bun = 4,
}

/// Data specific to JavaScript global scope
#[derive(Debug, Clone)]
pub struct JsGlobalScopeData {
    /// Runtime environment
    pub runtime: JsRuntime,
    /// Known JS globals for this runtime
    pub globals: ParamNames,
}

/// Data specific to Vue global scope
#[derive(Debug, Clone)]
pub struct VueGlobalScopeData {
    /// Known Vue globals ($refs, $emit, $slots, $attrs, $el, etc.)
    pub globals: ParamNames,
}

/// Data specific to external module scope
#[derive(Debug, Clone)]
pub struct ExternalModuleScopeData {
    /// Module source path
    pub source: CompactString,
    /// Whether this is a type-only import
    pub is_type_only: bool,
}

/// Data specific to closure scope (function declaration, function expression, arrow function)
#[derive(Debug, Clone)]
pub struct ClosureScopeData {
    /// Function name (if named function)
    pub name: Option<CompactString>,
    /// Parameter names
    pub param_names: ParamNames,
    /// Whether this is an arrow function (no `arguments`, no `this` binding)
    pub is_arrow: bool,
    /// Whether this is async
    pub is_async: bool,
    /// Whether this is a generator
    pub is_generator: bool,
}

/// Block kind for block scopes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BlockKind {
    Block,
    If,
    Else,
    For,
    ForIn,
    ForOf,
    While,
    DoWhile,
    Switch,
    Try,
    Catch,
    Finally,
    With,
}

impl BlockKind {
    /// Get the display name for this block kind
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::If => "if",
            Self::Else => "else",
            Self::For => "for",
            Self::ForIn => "for-in",
            Self::ForOf => "for-of",
            Self::While => "while",
            Self::DoWhile => "do-while",
            Self::Switch => "switch",
            Self::Try => "try",
            Self::Catch => "catch",
            Self::Finally => "finally",
            Self::With => "with",
        }
    }
}

/// Data specific to block scope (if, for, switch, etc.)
#[derive(Debug, Clone, Copy)]
pub struct BlockScopeData {
    /// Block kind
    pub kind: BlockKind,
}

/// Scope-specific data
#[derive(Debug, Clone)]
pub enum ScopeData {
    /// No additional data
    None,
    /// v-for specific data
    VFor(VForScopeData),
    /// v-slot specific data
    VSlot(VSlotScopeData),
    /// Event handler specific data
    EventHandler(EventHandlerScopeData),
    /// Callback specific data
    Callback(CallbackScopeData),
    /// Script setup specific data
    ScriptSetup(ScriptSetupScopeData),
    /// Non-script-setup specific data
    NonScriptSetup(NonScriptSetupScopeData),
    /// Client-only specific data
    ClientOnly(ClientOnlyScopeData),
    /// Universal scope specific data
    Universal(UniversalScopeData),
    /// JavaScript global specific data (with runtime info)
    JsGlobal(JsGlobalScopeData),
    /// Vue global specific data
    VueGlobal(VueGlobalScopeData),
    /// External module specific data
    ExternalModule(ExternalModuleScopeData),
    /// Closure specific data
    Closure(ClosureScopeData),
    /// Block specific data
    Block(BlockScopeData),
}

impl JsRuntime {
    /// Get the corresponding ScopeKind for this runtime
    #[inline]
    pub const fn to_scope_kind(self) -> ScopeKind {
        match self {
            JsRuntime::Universal => ScopeKind::JsGlobalUniversal,
            JsRuntime::Browser => ScopeKind::JsGlobalBrowser,
            JsRuntime::Node => ScopeKind::JsGlobalNode,
            JsRuntime::Deno => ScopeKind::JsGlobalDeno,
            JsRuntime::Bun => ScopeKind::JsGlobalBun,
        }
    }

    /// Get the corresponding BindingType for this runtime
    #[inline]
    pub const fn to_binding_type(self) -> BindingType {
        match self {
            JsRuntime::Universal => BindingType::JsGlobalUniversal,
            JsRuntime::Browser => BindingType::JsGlobalBrowser,
            JsRuntime::Node => BindingType::JsGlobalNode,
            JsRuntime::Deno => BindingType::JsGlobalDeno,
            JsRuntime::Bun => BindingType::JsGlobalBun,
        }
    }
}

impl Default for ScopeData {
    #[inline]
    fn default() -> Self {
        Self::None
    }
}

/// Source span
#[derive(Debug, Clone, Copy, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    #[inline(always)]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

/// Parent scope references (typically 1-2 parents)
pub type ParentScopes = SmallVec<[ScopeId; 2]>;

/// A single scope in the scope chain
#[derive(Debug)]
pub struct Scope {
    /// Unique identifier
    pub id: ScopeId,
    /// Parent scopes (empty for root, can have multiple for template scopes)
    /// First parent is the lexical parent, additional parents are accessible scopes (e.g., Vue globals)
    pub parents: ParentScopes,
    /// Kind of scope
    pub kind: ScopeKind,
    /// Bindings declared in this scope
    bindings: FxHashMap<CompactString, ScopeBinding>,
    /// Scope-specific data
    data: ScopeData,
    /// Source span
    pub span: Span,
}

impl Scope {
    /// Create a new scope with single parent
    #[inline]
    pub fn new(id: ScopeId, parent: Option<ScopeId>, kind: ScopeKind) -> Self {
        Self {
            id,
            parents: parent.map(|p| smallvec![p]).unwrap_or_default(),
            kind,
            bindings: FxHashMap::default(),
            data: ScopeData::None,
            span: Span::default(),
        }
    }

    /// Create a new scope with multiple parents
    #[inline]
    pub fn with_parents(id: ScopeId, parents: ParentScopes, kind: ScopeKind) -> Self {
        Self {
            id,
            parents,
            kind,
            bindings: FxHashMap::default(),
            data: ScopeData::None,
            span: Span::default(),
        }
    }

    /// Create a new scope with span
    #[inline]
    pub fn with_span(
        id: ScopeId,
        parent: Option<ScopeId>,
        kind: ScopeKind,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            id,
            parents: parent.map(|p| smallvec![p]).unwrap_or_default(),
            kind,
            bindings: FxHashMap::default(),
            data: ScopeData::None,
            span: Span::new(start, end),
        }
    }

    /// Create a new scope with span and multiple parents
    #[inline]
    pub fn with_span_parents(
        id: ScopeId,
        parents: ParentScopes,
        kind: ScopeKind,
        start: u32,
        end: u32,
    ) -> Self {
        Self {
            id,
            parents,
            kind,
            bindings: FxHashMap::default(),
            data: ScopeData::None,
            span: Span::new(start, end),
        }
    }

    /// Get the primary (lexical) parent
    #[inline]
    pub fn parent(&self) -> Option<ScopeId> {
        self.parents.first().copied()
    }

    /// Add an additional parent scope
    #[inline]
    pub fn add_parent(&mut self, parent: ScopeId) {
        if !self.parents.contains(&parent) {
            self.parents.push(parent);
        }
    }

    /// Set scope-specific data
    #[inline]
    pub fn set_data(&mut self, data: ScopeData) {
        self.data = data;
    }

    /// Get scope-specific data
    #[inline]
    pub fn data(&self) -> &ScopeData {
        &self.data
    }

    /// Add a binding to this scope
    #[inline]
    pub fn add_binding(&mut self, name: CompactString, binding: ScopeBinding) {
        self.bindings.insert(name, binding);
    }

    /// Get a binding by name (only in this scope, not parents)
    #[inline]
    pub fn get_binding(&self, name: &str) -> Option<&ScopeBinding> {
        self.bindings.get(name)
    }

    /// Get a mutable binding by name
    #[inline]
    pub fn get_binding_mut(&mut self, name: &str) -> Option<&mut ScopeBinding> {
        self.bindings.get_mut(name)
    }

    /// Check if this scope has a binding
    #[inline]
    pub fn has_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Iterate over all bindings in this scope
    #[inline]
    pub fn bindings(&self) -> impl Iterator<Item = (&str, &ScopeBinding)> {
        self.bindings.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of bindings in this scope
    #[inline]
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }

    /// Get display name for this scope (includes hook name for ClientOnly scopes)
    pub fn display_name(&self) -> String {
        match (&self.kind, &self.data) {
            (ScopeKind::ClientOnly, ScopeData::ClientOnly(data)) => {
                // Use hook name without "on" prefix: onMounted -> mounted
                data.hook_name
                    .strip_prefix("on")
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_else(|| data.hook_name.to_string())
            }
            _ => self.kind.to_display().to_string(),
        }
    }
}

bitflags! {
    /// Binding flags for tracking usage and mutation
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct BindingFlags: u8 {
        /// Binding has been referenced
        const USED = 1 << 0;
        /// Binding has been mutated
        const MUTATED = 1 << 1;
        /// Binding is a rest parameter
        const REST = 1 << 2;
        /// Binding has a default value
        const HAS_DEFAULT = 1 << 3;
    }
}

/// A binding within a scope
#[derive(Debug, Clone, Copy)]
pub struct ScopeBinding {
    /// The type of binding
    pub binding_type: BindingType,
    /// Source location of the declaration (offset in source)
    pub declaration_offset: u32,
    /// Binding flags
    flags: BindingFlags,
}

impl ScopeBinding {
    /// Create a new scope binding
    #[inline]
    pub const fn new(binding_type: BindingType, declaration_offset: u32) -> Self {
        Self {
            binding_type,
            declaration_offset,
            flags: BindingFlags::empty(),
        }
    }

    /// Check if binding is used
    #[inline]
    pub const fn is_used(&self) -> bool {
        self.flags.contains(BindingFlags::USED)
    }

    /// Check if binding is mutated
    #[inline]
    pub const fn is_mutated(&self) -> bool {
        self.flags.contains(BindingFlags::MUTATED)
    }

    /// Mark as used
    #[inline]
    pub fn mark_used(&mut self) {
        self.flags.insert(BindingFlags::USED);
    }

    /// Mark as mutated
    #[inline]
    pub fn mark_mutated(&mut self) {
        self.flags.insert(BindingFlags::MUTATED);
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
    /// Create a new scope chain with JS universal globals as root
    /// ECMAScript standard built-ins only (ECMA-262)
    #[inline]
    pub fn new() -> Self {
        let mut root = Scope::new(ScopeId::ROOT, None, ScopeKind::JsGlobalUniversal);
        for name in [
            "AggregateError",
            "arguments", // Function scope closure
            "Array",
            "ArrayBuffer",
            "AsyncFunction",
            "AsyncGenerator",
            "AsyncGeneratorFunction",
            "AsyncIterator",
            "Atomics",
            "BigInt",
            "BigInt64Array",
            "BigUint64Array",
            "Boolean",
            "console", // Non-standard but universally available
            "DataView",
            "Date",
            "decodeURI",
            "decodeURIComponent",
            "encodeURI",
            "encodeURIComponent",
            "Error",
            "eval",
            "EvalError",
            "Float32Array",
            "Float64Array",
            "Function",
            "Generator",
            "GeneratorFunction",
            "globalThis",
            "Infinity",
            "Int16Array",
            "Int32Array",
            "Int8Array",
            "Intl",
            "isFinite",
            "isNaN",
            "Iterator",
            "JSON",
            "Map",
            "Math",
            "NaN",
            "Number",
            "Object",
            "parseFloat",
            "parseInt",
            "Promise",
            "Proxy",
            "RangeError",
            "ReferenceError",
            "Reflect",
            "RegExp",
            "Set",
            "SharedArrayBuffer",
            "String",
            "Symbol",
            "SyntaxError",
            "this", // Function scope closure
            "TypeError",
            "Uint16Array",
            "Uint32Array",
            "Uint8Array",
            "Uint8ClampedArray",
            "undefined",
            "URIError",
            "WeakMap",
            "WeakSet",
        ] {
            root.add_binding(
                CompactString::new(name),
                ScopeBinding::new(BindingType::JsGlobalUniversal, 0),
            );
        }
        Self {
            scopes: vec![root],
            current: ScopeId::ROOT,
        }
    }

    /// Create with pre-allocated capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut root = Scope::new(ScopeId::ROOT, None, ScopeKind::JsGlobalUniversal);
        for name in [
            "AggregateError",
            "arguments",
            "Array",
            "ArrayBuffer",
            "AsyncFunction",
            "AsyncGenerator",
            "AsyncGeneratorFunction",
            "AsyncIterator",
            "Atomics",
            "BigInt",
            "BigInt64Array",
            "BigUint64Array",
            "Boolean",
            "console",
            "DataView",
            "Date",
            "decodeURI",
            "decodeURIComponent",
            "encodeURI",
            "encodeURIComponent",
            "Error",
            "eval",
            "EvalError",
            "Float32Array",
            "Float64Array",
            "Function",
            "Generator",
            "GeneratorFunction",
            "globalThis",
            "Infinity",
            "Int16Array",
            "Int32Array",
            "Int8Array",
            "Intl",
            "isFinite",
            "isNaN",
            "Iterator",
            "JSON",
            "Map",
            "Math",
            "NaN",
            "Number",
            "Object",
            "parseFloat",
            "parseInt",
            "Promise",
            "Proxy",
            "RangeError",
            "ReferenceError",
            "Reflect",
            "RegExp",
            "Set",
            "SharedArrayBuffer",
            "String",
            "Symbol",
            "SyntaxError",
            "this",
            "TypeError",
            "Uint16Array",
            "Uint32Array",
            "Uint8Array",
            "Uint8ClampedArray",
            "undefined",
            "URIError",
            "WeakMap",
            "WeakSet",
        ] {
            root.add_binding(
                CompactString::new(name),
                ScopeBinding::new(BindingType::JsGlobalUniversal, 0),
            );
        }
        let mut scopes = Vec::with_capacity(capacity);
        scopes.push(root);
        Self {
            scopes,
            current: ScopeId::ROOT,
        }
    }

    /// Get the current scope
    #[inline]
    pub fn current_scope(&self) -> &Scope {
        // SAFETY: current is always a valid index
        unsafe { self.scopes.get_unchecked(self.current.as_u32() as usize) }
    }

    /// Get the current scope mutably
    #[inline]
    pub fn current_scope_mut(&mut self) -> &mut Scope {
        let idx = self.current.as_u32() as usize;
        // SAFETY: current is always a valid index
        unsafe { self.scopes.get_unchecked_mut(idx) }
    }

    /// Get a scope by ID
    #[inline]
    pub fn get_scope(&self, id: ScopeId) -> Option<&Scope> {
        self.scopes.get(id.as_u32() as usize)
    }

    /// Get a scope by ID (unchecked)
    ///
    /// # Safety
    /// Caller must ensure id is valid
    #[inline]
    pub unsafe fn get_scope_unchecked(&self, id: ScopeId) -> &Scope {
        self.scopes.get_unchecked(id.as_u32() as usize)
    }

    /// Current scope ID
    #[inline]
    pub const fn current_id(&self) -> ScopeId {
        self.current
    }

    /// Number of scopes
    #[inline]
    pub fn len(&self) -> usize {
        self.scopes.len()
    }

    /// Check if empty (only root scope)
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.scopes.len() == 1
    }

    /// Iterate over all scopes
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Scope> {
        self.scopes.iter()
    }

    /// Find a scope by kind (returns the first match)
    #[inline]
    pub fn find_scope_by_kind(&self, kind: ScopeKind) -> Option<ScopeId> {
        self.scopes.iter().find(|s| s.kind == kind).map(|s| s.id)
    }

    /// Get mutable scope by ID
    #[inline]
    pub fn get_scope_mut(&mut self, id: ScopeId) -> Option<&mut Scope> {
        self.scopes.get_mut(id.as_u32() as usize)
    }

    /// Enter a new scope
    #[inline]
    pub fn enter_scope(&mut self, kind: ScopeKind) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let scope = Scope::new(id, Some(self.current), kind);
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a new scope with Vue global access (for template scopes)
    #[inline]
    pub fn enter_scope_with_vue_global(&mut self, kind: ScopeKind) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut parents: ParentScopes = smallvec![self.current];

        // Add Vue global scope as additional parent if it exists
        if let Some(vue_id) = self.find_scope_by_kind(ScopeKind::VueGlobal) {
            if !parents.contains(&vue_id) {
                parents.push(vue_id);
            }
        }

        let scope = Scope::with_parents(id, parents, kind);
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Exit the current scope and return to primary parent
    #[inline]
    pub fn exit_scope(&mut self) {
        if let Some(parent) = self.current_scope().parent() {
            self.current = parent;
        }
    }

    /// Build parents list including Vue global for template scopes
    fn build_template_parents(&self) -> ParentScopes {
        let mut parents: ParentScopes = smallvec![self.current];
        if let Some(vue_id) = self.find_scope_by_kind(ScopeKind::VueGlobal) {
            if !parents.contains(&vue_id) {
                parents.push(vue_id);
            }
        }
        parents
    }

    /// Enter a v-for scope with the given data
    pub fn enter_v_for_scope(&mut self, data: VForScopeData, start: u32, end: u32) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let parents = self.build_template_parents();
        let mut scope = Scope::with_span_parents(id, parents, ScopeKind::VFor, start, end);

        // Add value alias as binding
        scope.add_binding(
            data.value_alias.clone(),
            ScopeBinding::new(BindingType::SetupConst, start),
        );

        // Add key alias if present
        if let Some(ref key) = data.key_alias {
            scope.add_binding(
                key.clone(),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        // Add index alias if present
        if let Some(ref index) = data.index_alias {
            scope.add_binding(
                index.clone(),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        scope.set_data(ScopeData::VFor(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a v-slot scope with the given data
    pub fn enter_v_slot_scope(&mut self, data: VSlotScopeData, start: u32, end: u32) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let parents = self.build_template_parents();
        let mut scope = Scope::with_span_parents(id, parents, ScopeKind::VSlot, start, end);

        // Add prop names as bindings
        for prop_name in &data.prop_names {
            scope.add_binding(
                prop_name.clone(),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        scope.set_data(ScopeData::VSlot(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter an event handler scope
    pub fn enter_event_handler_scope(
        &mut self,
        data: EventHandlerScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let parents = self.build_template_parents();
        let mut scope = Scope::with_span_parents(id, parents, ScopeKind::EventHandler, start, end);

        // Add implicit $event binding if applicable
        if data.has_implicit_event {
            scope.add_binding(
                CompactString::const_new("$event"),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        // Add explicit parameter names as bindings
        for param_name in &data.param_names {
            scope.add_binding(
                param_name.clone(),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        scope.set_data(ScopeData::EventHandler(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a callback/arrow function scope (script context - no vue global)
    pub fn enter_callback_scope(
        &mut self,
        data: CallbackScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        // Script callbacks only have current scope as parent (no vue global)
        let mut scope = Scope::with_span(id, Some(self.current), ScopeKind::Callback, start, end);

        // Add parameter names as bindings
        for param_name in &data.param_names {
            scope.add_binding(
                param_name.clone(),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        scope.set_data(ScopeData::Callback(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a callback scope with vue global access (for template inline expressions)
    pub fn enter_template_callback_scope(
        &mut self,
        data: CallbackScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let parents = self.build_template_parents();
        let mut scope = Scope::with_span_parents(id, parents, ScopeKind::Callback, start, end);

        // Add parameter names as bindings
        for param_name in &data.param_names {
            scope.add_binding(
                param_name.clone(),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        scope.set_data(ScopeData::Callback(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a module scope
    pub fn enter_module_scope(&mut self, start: u32, end: u32) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let scope = Scope::with_span(id, Some(self.current), ScopeKind::Module, start, end);
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Set the current scope directly (used for switching between sibling scopes)
    #[inline]
    pub fn set_current(&mut self, id: ScopeId) {
        self.current = id;
    }

    /// Enter a script setup scope
    pub fn enter_script_setup_scope(
        &mut self,
        data: ScriptSetupScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut scope =
            Scope::with_span(id, Some(self.current), ScopeKind::ScriptSetup, start, end);
        scope.set_data(ScopeData::ScriptSetup(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a non-script-setup scope (Options API, regular script)
    pub fn enter_non_script_setup_scope(
        &mut self,
        data: NonScriptSetupScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut scope = Scope::with_span(
            id,
            Some(self.current),
            ScopeKind::NonScriptSetup,
            start,
            end,
        );
        scope.set_data(ScopeData::NonScriptSetup(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a universal scope (SSR - runs on both server and client)
    pub fn enter_universal_scope(
        &mut self,
        data: UniversalScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut scope = Scope::with_span(id, Some(self.current), ScopeKind::Universal, start, end);
        scope.set_data(ScopeData::Universal(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a client-only scope (onMounted, onBeforeUnmount, etc.)
    /// Parents: current scope + !js (browser globals)
    pub fn enter_client_only_scope(
        &mut self,
        data: ClientOnlyScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);

        // Build parents: current scope + !js (browser globals)
        let mut parents: ParentScopes = smallvec![self.current];
        if let Some(browser_id) = self.find_scope_by_kind(ScopeKind::JsGlobalBrowser) {
            if !parents.contains(&browser_id) {
                parents.push(browser_id);
            }
        }

        let mut scope = Scope::with_span_parents(id, parents, ScopeKind::ClientOnly, start, end);
        scope.set_data(ScopeData::ClientOnly(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a JavaScript global scope with specific runtime
    pub fn enter_js_global_scope(
        &mut self,
        data: JsGlobalScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let scope_kind = data.runtime.to_scope_kind();
        let binding_type = data.runtime.to_binding_type();
        let mut scope = Scope::with_span(id, Some(self.current), scope_kind, start, end);

        // Add JS globals as bindings with runtime-specific type
        for global in &data.globals {
            scope.add_binding(global.clone(), ScopeBinding::new(binding_type, start));
        }

        scope.set_data(ScopeData::JsGlobal(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a Vue global scope
    pub fn enter_vue_global_scope(
        &mut self,
        data: VueGlobalScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut scope = Scope::with_span(id, Some(self.current), ScopeKind::VueGlobal, start, end);

        // Add Vue globals as bindings
        for global in &data.globals {
            scope.add_binding(
                global.clone(),
                ScopeBinding::new(BindingType::VueGlobal, start),
            );
        }

        scope.set_data(ScopeData::VueGlobal(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter an external module scope
    pub fn enter_external_module_scope(
        &mut self,
        data: ExternalModuleScopeData,
        start: u32,
        end: u32,
    ) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut scope = Scope::with_span(
            id,
            Some(self.current),
            ScopeKind::ExternalModule,
            start,
            end,
        );
        scope.set_data(ScopeData::ExternalModule(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a closure scope (function declaration, function expression, arrow function)
    pub fn enter_closure_scope(&mut self, data: ClosureScopeData, start: u32, end: u32) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut scope = Scope::with_span(id, Some(self.current), ScopeKind::Closure, start, end);

        // Add parameter names as bindings
        for param in &data.param_names {
            scope.add_binding(
                param.clone(),
                ScopeBinding::new(BindingType::SetupConst, start),
            );
        }

        scope.set_data(ScopeData::Closure(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Enter a block scope (if, for, switch, try, catch, etc.)
    pub fn enter_block_scope(&mut self, data: BlockScopeData, start: u32, end: u32) -> ScopeId {
        let id = ScopeId::new(self.scopes.len() as u32);
        let mut scope = Scope::with_span(id, Some(self.current), ScopeKind::Block, start, end);
        scope.set_data(ScopeData::Block(data));
        self.scopes.push(scope);
        self.current = id;
        id
    }

    /// Look up a binding by name, searching through all parent scopes
    /// Uses BFS to search all accessible scopes (lexical parents + additional parents like Vue globals)
    #[inline]
    pub fn lookup(&self, name: &str) -> Option<(&Scope, &ScopeBinding)> {
        let mut visited: SmallVec<[ScopeId; 8]> = SmallVec::new();
        let mut queue: SmallVec<[ScopeId; 8]> = smallvec![self.current];

        while let Some(id) = queue.pop() {
            if visited.contains(&id) {
                continue;
            }
            visited.push(id);

            let scope = unsafe { self.scopes.get_unchecked(id.as_u32() as usize) };
            if let Some(binding) = scope.get_binding(name) {
                return Some((scope, binding));
            }

            // Add all parents to queue
            for &parent_id in &scope.parents {
                if !visited.contains(&parent_id) {
                    queue.push(parent_id);
                }
            }
        }

        None
    }

    /// Check if a name is defined in any accessible scope
    #[inline]
    pub fn is_defined(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Add a binding to the current scope
    #[inline]
    pub fn add_binding(&mut self, name: CompactString, binding: ScopeBinding) {
        self.current_scope_mut().add_binding(name, binding);
    }

    /// Mark a binding as used (searches through all parent scopes)
    pub fn mark_used(&mut self, name: &str) {
        let mut visited: SmallVec<[ScopeId; 8]> = SmallVec::new();
        let mut queue: SmallVec<[ScopeId; 8]> = smallvec![self.current];

        while let Some(id) = queue.pop() {
            if visited.contains(&id) {
                continue;
            }
            visited.push(id);

            let scope = &mut self.scopes[id.as_u32() as usize];
            if let Some(binding) = scope.get_binding_mut(name) {
                binding.mark_used();
                return;
            }

            // Collect parents before continuing (to avoid borrow issues)
            let parents: SmallVec<[ScopeId; 2]> = scope.parents.clone();
            for parent_id in parents {
                if !visited.contains(&parent_id) {
                    queue.push(parent_id);
                }
            }
        }
    }

    /// Check if a binding has been marked as used (searches through all scopes)
    pub fn is_used(&self, name: &str) -> bool {
        for scope in &self.scopes {
            if let Some(binding) = scope.get_binding(name) {
                return binding.is_used();
            }
        }
        false
    }

    /// Mark a binding as mutated (searches through all parent scopes)
    pub fn mark_mutated(&mut self, name: &str) {
        let mut visited: SmallVec<[ScopeId; 8]> = SmallVec::new();
        let mut queue: SmallVec<[ScopeId; 8]> = smallvec![self.current];

        while let Some(id) = queue.pop() {
            if visited.contains(&id) {
                continue;
            }
            visited.push(id);

            let scope = &mut self.scopes[id.as_u32() as usize];
            if let Some(binding) = scope.get_binding_mut(name) {
                binding.mark_mutated();
                return;
            }

            // Collect parents before continuing (to avoid borrow issues)
            let parents: SmallVec<[ScopeId; 2]> = scope.parents.clone();
            for parent_id in parents {
                if !visited.contains(&parent_id) {
                    queue.push(parent_id);
                }
            }
        }
    }

    /// Calculate the depth of a scope (distance from root via primary parent chain)
    #[inline]
    pub fn depth(&self, id: ScopeId) -> u32 {
        let mut depth = 0u32;
        let mut current_id = self.get_scope(id).and_then(|s| s.parent());
        while let Some(pid) = current_id {
            depth += 1;
            current_id = self.get_scope(pid).and_then(|s| s.parent());
        }
        depth
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
            CompactString::new("foo"),
            ScopeBinding::new(BindingType::SetupRef, 0),
        );

        assert!(chain.is_defined("foo"));
        assert!(!chain.is_defined("bar"));

        // Enter a new scope
        chain.enter_scope(ScopeKind::Function);
        chain.add_binding(
            CompactString::new("bar"),
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

        chain.add_binding(
            CompactString::new("x"),
            ScopeBinding::new(BindingType::SetupRef, 0),
        );

        chain.enter_scope(ScopeKind::Block);
        chain.add_binding(
            CompactString::new("x"),
            ScopeBinding::new(BindingType::SetupLet, 10),
        );

        // Should find the inner binding
        let (scope, binding) = chain.lookup("x").unwrap();
        assert_eq!(scope.kind, ScopeKind::Block);
        assert_eq!(binding.binding_type, BindingType::SetupLet);
    }

    #[test]
    fn test_v_for_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_v_for_scope(
            VForScopeData {
                value_alias: CompactString::new("item"),
                key_alias: Some(CompactString::new("key")),
                index_alias: Some(CompactString::new("index")),
                source: CompactString::new("items"),
                key_expression: Some(CompactString::new("item.id")),
            },
            0,
            100,
        );

        assert!(chain.is_defined("item"));
        assert!(chain.is_defined("key"));
        assert!(chain.is_defined("index"));
        assert!(!chain.is_defined("items")); // source is not a binding
    }

    #[test]
    fn test_v_slot_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_v_slot_scope(
            VSlotScopeData {
                name: CompactString::new("default"),
                props_pattern: Some(CompactString::new("{ item, index }")),
                prop_names: vize_carton::smallvec![
                    CompactString::new("item"),
                    CompactString::new("index")
                ],
            },
            0,
            100,
        );

        assert!(chain.is_defined("item"));
        assert!(chain.is_defined("index"));
    }

    #[test]
    fn test_event_handler_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_event_handler_scope(
            EventHandlerScopeData {
                event_name: CompactString::new("click"),
                has_implicit_event: true,
                param_names: vize_carton::smallvec![],
                handler_expression: None,
            },
            0,
            50,
        );

        // $event should be available
        assert!(chain.is_defined("$event"));
    }

    #[test]
    fn test_event_handler_scope_with_params() {
        let mut chain = ScopeChain::new();

        // @click="(e, extra) => handle(e, extra)"
        chain.enter_event_handler_scope(
            EventHandlerScopeData {
                event_name: CompactString::new("click"),
                has_implicit_event: false,
                param_names: vize_carton::smallvec![
                    CompactString::new("e"),
                    CompactString::new("extra")
                ],
                handler_expression: None,
            },
            0,
            50,
        );

        // Explicit params should be available
        assert!(chain.is_defined("e"));
        assert!(chain.is_defined("extra"));
        // $event should NOT be available (explicit params used)
        assert!(!chain.is_defined("$event"));
    }

    #[test]
    fn test_callback_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_callback_scope(
            CallbackScopeData {
                param_names: vize_carton::smallvec![CompactString::new("item")],
                context: CompactString::new(":class callback"),
            },
            0,
            50,
        );

        // Callback param should be available
        assert!(chain.is_defined("item"));
        assert_eq!(chain.current_scope().kind, ScopeKind::Callback);
    }

    #[test]
    fn test_nested_v_for() {
        let mut chain = ScopeChain::new();

        // Outer v-for
        chain.enter_v_for_scope(
            VForScopeData {
                value_alias: CompactString::new("row"),
                key_alias: None,
                index_alias: Some(CompactString::new("rowIndex")),
                source: CompactString::new("rows"),
                key_expression: None,
            },
            0,
            200,
        );

        // Inner v-for
        chain.enter_v_for_scope(
            VForScopeData {
                value_alias: CompactString::new("cell"),
                key_alias: None,
                index_alias: Some(CompactString::new("cellIndex")),
                source: CompactString::new("row.cells"),
                key_expression: None,
            },
            50,
            150,
        );

        // All bindings should be visible
        assert!(chain.is_defined("row"));
        assert!(chain.is_defined("rowIndex"));
        assert!(chain.is_defined("cell"));
        assert!(chain.is_defined("cellIndex"));

        // Exit inner
        chain.exit_scope();

        // Inner bindings no longer visible
        assert!(chain.is_defined("row"));
        assert!(chain.is_defined("rowIndex"));
        assert!(!chain.is_defined("cell"));
        assert!(!chain.is_defined("cellIndex"));
    }

    #[test]
    fn test_nested_callback_in_v_for() {
        let mut chain = ScopeChain::new();

        // v-for="item in items"
        chain.enter_v_for_scope(
            VForScopeData {
                value_alias: CompactString::new("item"),
                key_alias: None,
                index_alias: Some(CompactString::new("index")),
                source: CompactString::new("items"),
                key_expression: None,
            },
            0,
            200,
        );

        // @click="(e) => handleClick(item, e)"
        chain.enter_event_handler_scope(
            EventHandlerScopeData {
                event_name: CompactString::new("click"),
                has_implicit_event: false,
                param_names: vize_carton::smallvec![CompactString::new("e")],
                handler_expression: None,
            },
            50,
            100,
        );

        // Both v-for bindings and event params should be visible
        assert!(chain.is_defined("item"));
        assert!(chain.is_defined("index"));
        assert!(chain.is_defined("e"));
    }

    #[test]
    fn test_binding_flags() {
        let mut binding = ScopeBinding::new(BindingType::SetupRef, 0);
        assert!(!binding.is_used());
        assert!(!binding.is_mutated());

        binding.mark_used();
        assert!(binding.is_used());
        assert!(!binding.is_mutated());

        binding.mark_mutated();
        assert!(binding.is_used());
        assert!(binding.is_mutated());
    }

    #[test]
    fn test_script_setup_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_script_setup_scope(
            ScriptSetupScopeData {
                is_ts: true,
                is_async: false,
                generic: None,
            },
            0,
            500,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::ScriptSetup);

        // Add some bindings in script setup
        chain.add_binding(
            CompactString::new("counter"),
            ScopeBinding::new(BindingType::SetupRef, 10),
        );
        chain.add_binding(
            CompactString::new("message"),
            ScopeBinding::new(BindingType::SetupConst, 20),
        );

        assert!(chain.is_defined("counter"));
        assert!(chain.is_defined("message"));
    }

    #[test]
    fn test_non_script_setup_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_non_script_setup_scope(
            NonScriptSetupScopeData {
                is_ts: false,
                has_define_component: true,
            },
            0,
            500,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::NonScriptSetup);
    }

    #[test]
    fn test_universal_scope() {
        let mut chain = ScopeChain::new();

        // Script setup scope (runs on both server and client)
        chain.enter_script_setup_scope(
            ScriptSetupScopeData {
                is_ts: true,
                is_async: false,
                generic: None,
            },
            0,
            500,
        );

        // Enter universal scope (e.g., setup() content before lifecycle hooks)
        chain.enter_universal_scope(
            UniversalScopeData {
                context: CompactString::new("setup"),
            },
            10,
            400,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::Universal);

        // Universal code should be able to access parent script setup bindings
        chain.exit_scope(); // Exit universal
        chain.add_binding(
            CompactString::new("sharedData"),
            ScopeBinding::new(BindingType::SetupReactiveConst, 50),
        );
        chain.enter_universal_scope(
            UniversalScopeData {
                context: CompactString::new("setup"),
            },
            60,
            400,
        );

        assert!(chain.is_defined("sharedData"));
    }

    #[test]
    fn test_client_only_scope() {
        let mut chain = ScopeChain::new();

        // Script setup scope
        chain.enter_script_setup_scope(
            ScriptSetupScopeData {
                is_ts: true,
                is_async: false,
                generic: None,
            },
            0,
            500,
        );

        // Add binding in script setup
        chain.add_binding(
            CompactString::new("count"),
            ScopeBinding::new(BindingType::SetupRef, 10),
        );

        // Enter onMounted (client-only)
        chain.enter_client_only_scope(
            ClientOnlyScopeData {
                hook_name: CompactString::new("onMounted"),
            },
            100,
            200,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::ClientOnly);

        // Should be able to access parent bindings
        assert!(chain.is_defined("count"));

        chain.exit_scope();

        // Enter onBeforeUnmount (client-only)
        chain.enter_client_only_scope(
            ClientOnlyScopeData {
                hook_name: CompactString::new("onBeforeUnmount"),
            },
            250,
            300,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::ClientOnly);
        assert!(chain.is_defined("count"));
    }

    #[test]
    fn test_js_global_universal_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_js_global_scope(
            JsGlobalScopeData {
                runtime: JsRuntime::Universal,
                globals: vize_carton::smallvec![
                    CompactString::new("console"),
                    CompactString::new("Math"),
                    CompactString::new("Object"),
                    CompactString::new("Array"),
                ],
            },
            0,
            0,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::JsGlobalUniversal);

        // All JS globals should be defined
        assert!(chain.is_defined("console"));
        assert!(chain.is_defined("Math"));
        assert!(chain.is_defined("Object"));
        assert!(chain.is_defined("Array"));

        // Check binding type
        let (_, binding) = chain.lookup("console").unwrap();
        assert_eq!(binding.binding_type, BindingType::JsGlobalUniversal);
    }

    #[test]
    fn test_js_global_browser_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_js_global_scope(
            JsGlobalScopeData {
                runtime: JsRuntime::Browser,
                globals: vize_carton::smallvec![
                    CompactString::new("window"),
                    CompactString::new("document"),
                    CompactString::new("navigator"),
                    CompactString::new("localStorage"),
                ],
            },
            0,
            0,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::JsGlobalBrowser);

        // All browser globals should be defined
        assert!(chain.is_defined("window"));
        assert!(chain.is_defined("document"));
        assert!(chain.is_defined("navigator"));
        assert!(chain.is_defined("localStorage"));

        // Check binding type - should be browser-specific
        let (_, binding) = chain.lookup("window").unwrap();
        assert_eq!(binding.binding_type, BindingType::JsGlobalBrowser);
    }

    #[test]
    fn test_js_global_node_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_js_global_scope(
            JsGlobalScopeData {
                runtime: JsRuntime::Node,
                globals: vize_carton::smallvec![
                    CompactString::new("process"),
                    CompactString::new("Buffer"),
                    CompactString::new("__dirname"),
                    CompactString::new("require"),
                ],
            },
            0,
            0,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::JsGlobalNode);

        // All Node.js globals should be defined
        assert!(chain.is_defined("process"));
        assert!(chain.is_defined("Buffer"));
        assert!(chain.is_defined("__dirname"));
        assert!(chain.is_defined("require"));

        // Check binding type - should be Node-specific
        let (_, binding) = chain.lookup("process").unwrap();
        assert_eq!(binding.binding_type, BindingType::JsGlobalNode);
    }

    #[test]
    fn test_vue_global_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_vue_global_scope(
            VueGlobalScopeData {
                globals: vize_carton::smallvec![
                    CompactString::new("$refs"),
                    CompactString::new("$emit"),
                    CompactString::new("$slots"),
                    CompactString::new("$attrs"),
                ],
            },
            0,
            0,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::VueGlobal);

        // All Vue globals should be defined
        assert!(chain.is_defined("$refs"));
        assert!(chain.is_defined("$emit"));
        assert!(chain.is_defined("$slots"));
        assert!(chain.is_defined("$attrs"));

        // Check binding type
        let (_, binding) = chain.lookup("$refs").unwrap();
        assert_eq!(binding.binding_type, BindingType::VueGlobal);
    }

    #[test]
    fn test_external_module_scope() {
        let mut chain = ScopeChain::new();

        chain.enter_external_module_scope(
            ExternalModuleScopeData {
                source: CompactString::new("vue"),
                is_type_only: false,
            },
            0,
            50,
        );

        assert_eq!(chain.current_scope().kind, ScopeKind::ExternalModule);

        // Add imports from external module
        chain.add_binding(
            CompactString::new("ref"),
            ScopeBinding::new(BindingType::ExternalModule, 10),
        );
        chain.add_binding(
            CompactString::new("computed"),
            ScopeBinding::new(BindingType::ExternalModule, 15),
        );

        assert!(chain.is_defined("ref"));
        assert!(chain.is_defined("computed"));

        let (_, binding) = chain.lookup("ref").unwrap();
        assert_eq!(binding.binding_type, BindingType::ExternalModule);
    }

    #[test]
    fn test_nested_ssr_scopes() {
        let mut chain = ScopeChain::new();

        // Root: Universal JS Global
        chain.enter_js_global_scope(
            JsGlobalScopeData {
                runtime: JsRuntime::Universal,
                globals: vize_carton::smallvec![CompactString::new("console")],
            },
            0,
            0,
        );

        // Vue global
        chain.enter_vue_global_scope(
            VueGlobalScopeData {
                globals: vize_carton::smallvec![CompactString::new("$emit")],
            },
            0,
            0,
        );

        // Script setup
        chain.enter_script_setup_scope(
            ScriptSetupScopeData {
                is_ts: true,
                is_async: false,
                generic: None,
            },
            0,
            500,
        );

        chain.add_binding(
            CompactString::new("count"),
            ScopeBinding::new(BindingType::SetupRef, 10),
        );

        // Universal scope (setup logic)
        chain.enter_universal_scope(
            UniversalScopeData {
                context: CompactString::new("setup-body"),
            },
            20,
            400,
        );

        // Client-only scope (onMounted)
        chain.enter_client_only_scope(
            ClientOnlyScopeData {
                hook_name: CompactString::new("onMounted"),
            },
            100,
            200,
        );

        // All scopes should be accessible
        assert!(chain.is_defined("console")); // JS global
        assert!(chain.is_defined("$emit")); // Vue global
        assert!(chain.is_defined("count")); // Script setup binding

        // Current scope is client-only
        assert_eq!(chain.current_scope().kind, ScopeKind::ClientOnly);
    }
}
