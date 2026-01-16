//! Analysis summary for Vue SFC semantic analysis.
//!
//! This module provides the `Croquis` struct that aggregates all
//! semantic analysis results from a Vue SFC. It serves as the bridge between
//! the parser and downstream consumers (linter, transformer, codegen).
//!
//! ## Architecture
//!
//! ```text
//! vize_armature (Parse)
//!        ↓
//!   vize_relief (AST)
//!        ↓
//!  vize_croquis (Semantic Analysis)
//!        ↓
//!   Croquis ←── This module
//!        ↓
//!  ┌─────┴─────┐
//!  ↓           ↓
//! patina    atelier
//! (lint)    (transform)
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use vize_croquis::{Analyzer, Croquis};
//!
//! let summary = Analyzer::new()
//!     .analyze_script(script_content)
//!     .analyze_template(template_ast)
//!     .finish();
//!
//! // Use in linter
//! let lint_ctx = LintContext::with_analysis(&summary);
//!
//! // Use in transformer
//! let transform_ctx = TransformContext::with_analysis(&summary);
//! ```

use crate::hoist::HoistTracker;
use crate::macros::MacroTracker;
use crate::provide::ProvideInjectTracker;
use crate::reactivity::ReactivityTracker;
use crate::types::TypeResolver;
use crate::{ScopeChain, SymbolTable};
use vize_carton::{CompactString, FxHashMap, FxHashSet};
use vize_relief::BindingType;

/// Template-level metadata collected during analysis.
#[derive(Debug, Clone, Default)]
pub struct TemplateInfo {
    /// Number of root elements at depth 0 in template.
    /// A value > 1 indicates multi-root component (fragments).
    pub root_element_count: usize,
    /// Whether $attrs is referenced anywhere in the template.
    pub uses_attrs: bool,
    /// Whether v-bind="$attrs" is explicitly used (not just $attrs.class etc.)
    pub binds_attrs_explicitly: bool,
    /// Whether inheritAttrs: false is set in defineOptions.
    pub inherit_attrs_disabled: bool,
}

impl TemplateInfo {
    /// Check if the component has multiple root elements.
    #[inline]
    pub fn has_multiple_roots(&self) -> bool {
        self.root_element_count > 1
    }

    /// Check if fallthrough attrs may be lost (multi-root without explicit binding).
    #[inline]
    pub fn may_lose_fallthrough_attrs(&self) -> bool {
        self.has_multiple_roots() && !self.binds_attrs_explicitly
    }
}

/// Complete semantic analysis summary for a Vue SFC.
///
/// This struct aggregates all analysis results and provides a unified
/// interface for downstream consumers (linter, transformer).
#[derive(Debug, Default)]
pub struct Croquis {
    /// Scope chain for template expressions
    pub scopes: ScopeChain,

    /// Symbol table for script bindings
    pub symbols: SymbolTable,

    /// Compiler macro information (defineProps, defineEmits, etc.)
    pub macros: MacroTracker,

    /// Reactivity tracking (ref, reactive, computed)
    pub reactivity: ReactivityTracker,

    /// Provide/Inject tracking
    pub provide_inject: ProvideInjectTracker,

    /// TypeScript type resolution
    pub types: TypeResolver,

    /// Hoisting analysis for template optimization
    pub hoists: HoistTracker,

    /// Script binding metadata (for template access)
    pub bindings: BindingMetadata,

    /// Template-level metadata (root count, $attrs usage, etc.)
    pub template_info: TemplateInfo,

    /// Components used in template (names only, for quick lookup)
    pub used_components: FxHashSet<CompactString>,

    /// Detailed component usage information (props, events, slots)
    pub component_usages: Vec<ComponentUsage>,

    /// Directives used in template
    pub used_directives: FxHashSet<CompactString>,

    /// Variables referenced in template but not defined
    pub undefined_refs: Vec<UndefinedRef>,

    /// Unused bindings (defined but not referenced in template)
    pub unused_bindings: Vec<CompactString>,

    /// Type exports from script setup (hoisted to module level)
    pub type_exports: Vec<TypeExport>,

    /// Invalid non-type exports in script setup
    pub invalid_exports: Vec<InvalidExport>,

    /// Template expressions for type checking (interpolations, v-bind, etc.)
    pub template_expressions: Vec<TemplateExpression>,
}

/// Template expression for type checking.
#[derive(Debug, Clone)]
pub struct TemplateExpression {
    /// The expression content
    pub content: CompactString,
    /// Kind of expression
    pub kind: TemplateExpressionKind,
    /// Start offset in template (relative to template block)
    pub start: u32,
    /// End offset in template (relative to template block)
    pub end: u32,
    /// The scope this expression belongs to
    pub scope_id: crate::scope::ScopeId,
}

/// Kind of template expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateExpressionKind {
    /// Mustache interpolation: {{ expr }}
    Interpolation,
    /// v-bind: :prop="expr" or v-bind:prop="expr"
    VBind,
    /// v-on handler (non-inline): @event="handler"
    VOn,
    /// v-if condition: v-if="cond"
    VIf,
    /// v-show condition: v-show="cond"
    VShow,
    /// v-model: v-model="value"
    VModel,
}

impl TemplateExpressionKind {
    /// Get the string representation without allocation.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Interpolation => "Interpolation",
            Self::VBind => "VBind",
            Self::VOn => "VOn",
            Self::VIf => "VIf",
            Self::VShow => "VShow",
            Self::VModel => "VModel",
        }
    }
}

use vize_carton::SmallVec;

/// Information about a component used in template.
///
/// Uses SmallVec to avoid heap allocations for typical component usage
/// (most components have < 8 props, < 4 events, < 2 slots).
#[derive(Debug, Clone)]
pub struct ComponentUsage {
    /// Component name (e.g., "MyButton", "user-card")
    pub name: CompactString,
    /// Start offset in template
    pub start: u32,
    /// End offset in template
    pub end: u32,
    /// Props passed to this component (stack-allocated for ≤8 props)
    pub props: SmallVec<[PassedProp; 8]>,
    /// Event listeners on this component (stack-allocated for ≤4 events)
    pub events: SmallVec<[EventListener; 4]>,
    /// Slots provided to this component (stack-allocated for ≤2 slots)
    pub slots: SmallVec<[SlotUsage; 2]>,
    /// Whether v-bind="$attrs" or similar spread is used
    pub has_spread_attrs: bool,
}

/// A prop passed to a component in template.
#[derive(Debug, Clone)]
pub struct PassedProp {
    /// Prop name (kebab-case or camelCase as written)
    pub name: CompactString,
    /// The expression if dynamic, or literal value if static
    pub value: Option<CompactString>,
    /// Start offset
    pub start: u32,
    /// End offset
    pub end: u32,
    /// Whether this is a dynamic binding (:prop or v-bind:prop)
    pub is_dynamic: bool,
}

/// An event listener on a component.
#[derive(Debug, Clone)]
pub struct EventListener {
    /// Event name (e.g., "click", "update:modelValue")
    pub name: CompactString,
    /// Handler expression
    pub handler: Option<CompactString>,
    /// Modifiers (stack-allocated for ≤4 modifiers)
    pub modifiers: SmallVec<[CompactString; 4]>,
    /// Start offset
    pub start: u32,
    /// End offset
    pub end: u32,
}

/// A slot provided to a component.
#[derive(Debug, Clone)]
pub struct SlotUsage {
    /// Slot name ("default" if unnamed)
    pub name: CompactString,
    /// Scope variable names if any (stack-allocated for ≤4 vars)
    pub scope_vars: SmallVec<[CompactString; 4]>,
    /// Start offset
    pub start: u32,
    /// End offset
    pub end: u32,
    /// Whether this slot has scope (v-slot:name="scope")
    pub has_scope: bool,
}

impl Croquis {
    /// Convert analysis summary to VIR (Vize Intermediate Representation) text format.
    ///
    /// This generates a TOML-like human-readable representation of the analysis.
    /// Performance: Pre-allocates buffer, uses write! macro for zero-copy formatting.
    pub fn to_vir(&self) -> String {
        use crate::macros::MacroKind;
        use std::fmt::Write;

        // Pre-allocate with estimated capacity
        let mut output = String::with_capacity(4096);

        // [vir]
        writeln!(output, "[vir]").ok();
        writeln!(output, "script_setup={}", self.bindings.is_script_setup).ok();
        writeln!(output, "scopes={}", self.scopes.len()).ok();
        writeln!(output, "bindings={}", self.bindings.bindings.len()).ok();
        writeln!(output).ok();

        // [surface] - Component Surface (Public API)
        let has_surface = !self.macros.props().is_empty()
            || !self.macros.emits().is_empty()
            || !self.macros.models().is_empty()
            || self
                .macros
                .all_calls()
                .iter()
                .any(|c| matches!(c.kind, MacroKind::DefineExpose | MacroKind::DefineSlots));

        if has_surface {
            // [surface.props] props (ist)
            if !self.macros.props().is_empty() {
                writeln!(output, "[surface.props]").ok();
                for prop in self.macros.props() {
                    let req = if prop.required { "!" } else { "?" };
                    let def = if prop.default_value.is_some() {
                        "="
                    } else {
                        ""
                    };
                    if let Some(ref ty) = prop.prop_type {
                        writeln!(output, "{}{}:{}{}", prop.name, req, ty, def).ok();
                    } else {
                        writeln!(output, "{}{}{}", prop.name, req, def).ok();
                    }
                }
                writeln!(output).ok();
            }

            // [surface.emits] emits
            if !self.macros.emits().is_empty() {
                writeln!(output, "[surface.emits]").ok();
                for emit in self.macros.emits() {
                    if let Some(ref ty) = emit.payload_type {
                        writeln!(output, "{}:{}", emit.name, ty).ok();
                    } else {
                        writeln!(output, "{}", emit.name).ok();
                    }
                }
                writeln!(output).ok();
            }

            // [surface.models] models
            if !self.macros.models().is_empty() {
                writeln!(output, "[surface.models]").ok();
                for model in self.macros.models() {
                    let name = if model.name.is_empty() {
                        "modelValue"
                    } else {
                        model.name.as_str()
                    };
                    if let Some(ref ty) = model.model_type {
                        writeln!(output, "{}:{}", name, ty).ok();
                    } else {
                        writeln!(output, "{}", name).ok();
                    }
                }
                writeln!(output).ok();
            }

            // [surface.expose]
            let expose_calls: Vec<_> = self
                .macros
                .all_calls()
                .iter()
                .filter(|c| c.kind == MacroKind::DefineExpose)
                .collect();
            if !expose_calls.is_empty() {
                writeln!(output, "[surface.expose]").ok();
                for call in &expose_calls {
                    if let Some(args) = &call.runtime_args {
                        writeln!(output, "{}", args).ok();
                    } else {
                        writeln!(output, "@{}:{}", call.start, call.end).ok();
                    }
                }
                writeln!(output).ok();
            }

            // [surface.slots]
            let slots_calls: Vec<_> = self
                .macros
                .all_calls()
                .iter()
                .filter(|c| c.kind == MacroKind::DefineSlots)
                .collect();
            if !slots_calls.is_empty() {
                writeln!(output, "[surface.slots]").ok();
                for call in &slots_calls {
                    if let Some(type_args) = &call.type_args {
                        writeln!(output, "{}", type_args).ok();
                    } else {
                        writeln!(output, "@{}:{}", call.start, call.end).ok();
                    }
                }
                writeln!(output).ok();
            }
        }

        // [macros] - moved up for importance
        if !self.macros.all_calls().is_empty() {
            writeln!(output, "[macros]").ok();
            for call in self.macros.all_calls() {
                if let Some(ref ty) = call.type_args {
                    writeln!(
                        output,
                        "@{}<{}> @{}:{}",
                        call.name, ty, call.start, call.end
                    )
                    .ok();
                } else {
                    writeln!(output, "@{} @{}:{}", call.name, call.start, call.end).ok();
                }
            }
            writeln!(output).ok();
        }

        // [reactivity]
        if self.reactivity.count() > 0 {
            writeln!(output, "[reactivity]").ok();
            for src in self.reactivity.sources() {
                writeln!(output, "{}={}", src.name, src.kind.to_display()).ok();
            }
            writeln!(output).ok();
        }

        // [extern] external imports
        let extern_scopes: Vec<_> = self
            .scopes
            .iter()
            .filter(|s| s.kind == crate::scope::ScopeKind::ExternalModule)
            .collect();
        if !extern_scopes.is_empty() {
            writeln!(output, "[extern]").ok();
            for scope in &extern_scopes {
                if let crate::scope::ScopeData::ExternalModule(data) = scope.data() {
                    let type_only = if data.is_type_only { "^" } else { "" };
                    let bd: Vec<_> = scope.bindings().map(|(n, _)| n).collect();
                    if bd.is_empty() {
                        writeln!(output, "{}{}", data.source, type_only).ok();
                    } else {
                        writeln!(output, "{}{} {{{}}}", data.source, type_only, bd.join(",")).ok();
                    }
                }
            }
            writeln!(output).ok();
        }

        // [types] type exports
        if !self.type_exports.is_empty() {
            writeln!(output, "[types]").ok();
            for te in &self.type_exports {
                let hoist = if te.hoisted { "^" } else { "" };
                let kind = match te.kind {
                    TypeExportKind::Type => "t",
                    TypeExportKind::Interface => "i",
                };
                writeln!(
                    output,
                    "{}{}{}@{}:{}",
                    te.name, hoist, kind, te.start, te.end
                )
                .ok();
            }
            writeln!(output).ok();
        }

        // [bindings] - grouped by kind
        if !self.bindings.bindings.is_empty() {
            use vize_relief::BindingType;

            writeln!(output, "[bindings]").ok();

            // Group bindings by type for compact output
            let mut by_type: FxHashMap<BindingType, Vec<&str>> = FxHashMap::default();
            for (name, bt) in &self.bindings.bindings {
                by_type.entry(*bt).or_default().push(name.as_str());
            }

            // Output in a consistent order
            let type_order = [
                BindingType::SetupConst,
                BindingType::SetupRef,
                BindingType::SetupMaybeRef,
                BindingType::SetupReactiveConst,
                BindingType::SetupLet,
                BindingType::Props,
                BindingType::PropsAliased,
                BindingType::Data,
                BindingType::Options,
                BindingType::LiteralConst,
                BindingType::JsGlobalUniversal,
                BindingType::JsGlobalBrowser,
                BindingType::JsGlobalNode,
                BindingType::JsGlobalDeno,
                BindingType::JsGlobalBun,
                BindingType::VueGlobal,
                BindingType::ExternalModule,
            ];

            for bt in type_order {
                if let Some(names) = by_type.get(&bt) {
                    writeln!(output, "{}:{}", bt.to_vir(), names.join(",")).ok();
                }
            }
            writeln!(output).ok();
        }

        // [scopes]
        if !self.scopes.is_empty() {
            // Build a map from scope ID -> prefixed display ID
            // Separate counters for ~, !, # prefixes
            let mut prefix_counters: FxHashMap<&str, u32> = FxHashMap::default();
            let mut id_to_display: FxHashMap<u32, String> = FxHashMap::default();

            // Helper to determine effective prefix by checking parent chain
            // If any ancestor is ClientOnly, child scopes should also be !
            // If any ancestor is server-only, child scopes should also be #
            let get_effective_prefix = |scope: &crate::scope::Scope| -> &'static str {
                // First check the scope's own prefix
                let own_prefix = scope.kind.prefix();
                if own_prefix != "~" {
                    return own_prefix;
                }

                // Check parent chain for client-only or server-only context
                let mut visited: vize_carton::SmallVec<[crate::scope::ScopeId; 8]> =
                    vize_carton::SmallVec::new();
                let mut queue: vize_carton::SmallVec<[crate::scope::ScopeId; 8]> =
                    scope.parents.iter().copied().collect();

                while let Some(parent_id) = queue.pop() {
                    if visited.contains(&parent_id) {
                        continue;
                    }
                    visited.push(parent_id);

                    if let Some(parent) = self.scopes.get_scope(parent_id) {
                        let parent_prefix = parent.kind.prefix();
                        if parent_prefix == "!" {
                            return "!"; // Client-only context propagates down
                        }
                        if parent_prefix == "#" {
                            return "#"; // Server-only context propagates down
                        }
                        // Add grandparents to queue
                        for &gp in &parent.parents {
                            if !visited.contains(&gp) {
                                queue.push(gp);
                            }
                        }
                    }
                }

                "~" // Default to universal
            };

            for scope in self.scopes.iter() {
                let prefix = get_effective_prefix(scope);
                let counter = prefix_counters.entry(prefix).or_insert(0);
                id_to_display.insert(scope.id.as_u32(), format!("{}{}", prefix, *counter));
                *counter += 1;
            }

            writeln!(output, "[scopes]").ok();
            for scope in self.scopes.iter() {
                let bd_count = scope.bindings().count();

                // Get scope display ID with prefix
                let scope_id_display = id_to_display
                    .get(&scope.id.as_u32())
                    .map(|s| s.as_str())
                    .unwrap_or("?");

                // Build parent references from the parents list using display IDs
                let par = if scope.parents.is_empty() {
                    String::new()
                } else {
                    let refs: Vec<_> = scope
                        .parents
                        .iter()
                        .filter_map(|p| id_to_display.get(&p.as_u32()))
                        .map(|s| s.as_str())
                        .collect();
                    if refs.is_empty() {
                        String::new()
                    } else {
                        format!(" < {}", refs.join(", "))
                    }
                };

                if bd_count > 0 {
                    let bd: Vec<_> = scope.bindings().map(|(n, _)| n).collect();
                    writeln!(
                        output,
                        "{} {} @{}:{} [{}]{}",
                        scope_id_display,
                        scope.display_name(),
                        scope.span.start,
                        scope.span.end,
                        bd.join(","),
                        par
                    )
                    .ok();
                } else {
                    writeln!(
                        output,
                        "{} {} @{}:{}{}",
                        scope_id_display,
                        scope.display_name(),
                        scope.span.start,
                        scope.span.end,
                        par
                    )
                    .ok();
                }
            }
            writeln!(output).ok();
        }

        // [errors]
        if !self.invalid_exports.is_empty() {
            writeln!(output, "[errors]").ok();
            for ie in &self.invalid_exports {
                writeln!(output, "{}={:?}@{}:{}", ie.name, ie.kind, ie.start, ie.end).ok();
            }
            writeln!(output).ok();
        }

        output
    }
}

/// Binding metadata extracted from script analysis.
///
/// This is compatible with the existing BindingMetadata in atelier_core
/// but uses CompactString for efficiency.
#[derive(Debug, Default, Clone)]
pub struct BindingMetadata {
    /// Binding name to type mapping
    pub bindings: FxHashMap<CompactString, BindingType>,

    /// Whether this is from script setup
    pub is_script_setup: bool,

    /// Props aliases (local name -> prop key)
    pub props_aliases: FxHashMap<CompactString, CompactString>,
}

impl BindingMetadata {
    /// Create new empty binding metadata
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create for script setup
    #[inline]
    pub fn script_setup() -> Self {
        Self {
            is_script_setup: true,
            ..Default::default()
        }
    }

    /// Add a binding
    #[inline]
    pub fn add(&mut self, name: impl Into<CompactString>, binding_type: BindingType) {
        self.bindings.insert(name.into(), binding_type);
    }

    /// Get binding type for a name
    #[inline]
    pub fn get(&self, name: &str) -> Option<BindingType> {
        self.bindings.get(name).copied()
    }

    /// Check if a binding exists
    #[inline]
    pub fn contains(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Check if a binding is a ref (needs .value in script)
    #[inline]
    pub fn is_ref(&self, name: &str) -> bool {
        matches!(
            self.get(name),
            Some(BindingType::SetupRef | BindingType::SetupMaybeRef)
        )
    }

    /// Check if a binding is from props
    #[inline]
    pub fn is_prop(&self, name: &str) -> bool {
        matches!(
            self.get(name),
            Some(BindingType::Props | BindingType::PropsAliased)
        )
    }

    /// Iterate over all bindings
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&str, BindingType)> {
        self.bindings.iter().map(|(k, v)| (k.as_str(), *v))
    }
}

/// An undefined reference in template
#[derive(Debug, Clone)]
pub struct UndefinedRef {
    /// The identifier name
    pub name: CompactString,
    /// Source offset
    pub offset: u32,
    /// Context (e.g., "v-if expression", "interpolation")
    pub context: CompactString,
}

/// An unused template variable (v-for or v-slot)
#[derive(Debug, Clone)]
pub struct UnusedTemplateVar {
    /// The variable name
    pub name: CompactString,
    /// Source offset of the declaration
    pub offset: u32,
    /// Context where the variable is defined
    pub context: UnusedVarContext,
}

/// Context for unused template variable
#[derive(Debug, Clone)]
pub enum UnusedVarContext {
    /// Value variable in v-for (e.g., "item" in v-for="item in items")
    VForValue,
    /// Key variable in v-for (e.g., "key" in v-for="(item, key) in items")
    VForKey,
    /// Index variable in v-for (e.g., "index" in v-for="(item, index) in items")
    VForIndex,
    /// Slot prop in v-slot (e.g., "item" in v-slot="{ item }")
    VSlot { slot_name: String },
}

/// Type export from script setup (hoisted to module level)
#[derive(Debug, Clone)]
pub struct TypeExport {
    /// The type/interface name
    pub name: CompactString,
    /// Kind of export (type or interface)
    pub kind: TypeExportKind,
    /// Source offset
    pub start: u32,
    pub end: u32,
    /// Whether this is hoisted from script setup
    pub hoisted: bool,
}

/// Kind of type export
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeExportKind {
    Type = 0,
    Interface = 1,
}

/// Invalid export in script setup
#[derive(Debug, Clone)]
pub struct InvalidExport {
    /// The export name
    pub name: CompactString,
    /// Kind of invalid export
    pub kind: InvalidExportKind,
    /// Source offset
    pub start: u32,
    pub end: u32,
}

/// Kind of invalid export
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InvalidExportKind {
    Const = 0,
    Let = 1,
    Var = 2,
    Function = 3,
    Class = 4,
    Default = 5,
}

impl Croquis {
    /// Create a new empty analysis summary
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a variable is defined in any scope
    #[inline]
    pub fn is_defined(&self, name: &str) -> bool {
        self.scopes.is_defined(name) || self.bindings.contains(name)
    }

    /// Get the binding type for a name
    #[inline]
    pub fn get_binding_type(&self, name: &str) -> Option<BindingType> {
        // First check scope chain (template-local variables)
        if let Some((_, binding)) = self.scopes.lookup(name) {
            return Some(binding.binding_type);
        }
        // Then check script bindings
        self.bindings.get(name)
    }

    /// Check if a name needs .value access in template
    ///
    /// In templates, refs are auto-unwrapped, so this returns false.
    /// Use `needs_value_in_script` for script context.
    #[inline]
    pub fn needs_value_in_template(&self, _name: &str) -> bool {
        // Templates auto-unwrap refs
        false
    }

    /// Check if a name needs .value access in script
    #[inline]
    pub fn needs_value_in_script(&self, name: &str) -> bool {
        self.reactivity.needs_value_access(name)
    }

    /// Check if a component is registered/imported
    #[inline]
    pub fn is_component_registered(&self, name: &str) -> bool {
        // Check if it's in used_components or is a known const binding
        // Components are typically imported as SetupConst
        self.used_components.contains(name)
            || self
                .bindings
                .get(name)
                .is_some_and(|t| matches!(t, BindingType::SetupConst))
    }

    /// Get props defined via defineProps
    pub fn get_props(&self) -> impl Iterator<Item = (&str, bool)> {
        self.macros
            .props()
            .iter()
            .map(|p| (p.name.as_str(), p.required))
    }

    /// Get emits defined via defineEmits
    pub fn get_emits(&self) -> impl Iterator<Item = &str> {
        self.macros.emits().iter().map(|e| e.name.as_str())
    }

    /// Get models defined via defineModel
    pub fn get_models(&self) -> impl Iterator<Item = &str> {
        self.macros.models().iter().map(|m| m.name.as_str())
    }

    /// Check if component uses async setup (top-level await)
    #[inline]
    pub fn is_async(&self) -> bool {
        self.macros.is_async()
    }

    /// Get unused template variables (v-for, v-slot variables that are not used)
    pub fn unused_template_vars(&self) -> Vec<UnusedTemplateVar> {
        use crate::scope::{ScopeData, ScopeKind};

        let mut unused = Vec::new();

        for scope in self.scopes.iter() {
            // Only check v-for and v-slot scopes
            if !matches!(scope.kind, ScopeKind::VFor | ScopeKind::VSlot) {
                continue;
            }

            for (name, binding) in scope.bindings() {
                if !binding.is_used() {
                    let context = match scope.data() {
                        ScopeData::VFor(data) => {
                            // Determine which kind of variable this is
                            if data.value_alias.as_str() == name {
                                UnusedVarContext::VForValue
                            } else if data.key_alias.as_ref().is_some_and(|k| k.as_str() == name) {
                                UnusedVarContext::VForKey
                            } else if data
                                .index_alias
                                .as_ref()
                                .is_some_and(|i| i.as_str() == name)
                            {
                                UnusedVarContext::VForIndex
                            } else {
                                UnusedVarContext::VForValue
                            }
                        }
                        ScopeData::VSlot(data) => UnusedVarContext::VSlot {
                            slot_name: data.name.to_string(),
                        },
                        _ => continue,
                    };

                    unused.push(UnusedTemplateVar {
                        name: CompactString::new(name),
                        offset: binding.declaration_offset,
                        context,
                    });
                }
            }
        }

        unused
    }

    /// Get analysis statistics for debugging
    pub fn stats(&self) -> AnalysisStats {
        AnalysisStats {
            scope_count: self.scopes.len(),
            symbol_count: self.symbols.len(),
            binding_count: self.bindings.bindings.len(),
            macro_count: self.macros.all_calls().len(),
            prop_count: self.macros.props().len(),
            emit_count: self.macros.emits().len(),
            model_count: self.macros.models().len(),
            hoist_count: self.hoists.count(),
            used_components: self.used_components.len(),
            used_directives: self.used_directives.len(),
            undefined_ref_count: self.undefined_refs.len(),
            unused_binding_count: self.unused_bindings.len(),
        }
    }
}

/// Statistics about the analysis
#[derive(Debug, Clone, Default)]
pub struct AnalysisStats {
    pub scope_count: usize,
    pub symbol_count: usize,
    pub binding_count: usize,
    pub macro_count: usize,
    pub prop_count: usize,
    pub emit_count: usize,
    pub model_count: usize,
    pub hoist_count: usize,
    pub used_components: usize,
    pub used_directives: usize,
    pub undefined_ref_count: usize,
    pub unused_binding_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binding_metadata() {
        let mut meta = BindingMetadata::script_setup();
        meta.add("count", BindingType::SetupRef);
        meta.add("state", BindingType::SetupReactiveConst);
        meta.add("msg", BindingType::Props);

        assert!(meta.is_script_setup);
        assert!(meta.is_ref("count"));
        assert!(!meta.is_ref("state"));
        assert!(meta.is_prop("msg"));
    }

    #[test]
    fn test_analysis_summary() {
        let mut summary = Croquis::new();
        summary.bindings.add("foo", BindingType::SetupRef);

        assert!(summary.is_defined("foo"));
        assert!(!summary.is_defined("bar"));
        assert_eq!(summary.get_binding_type("foo"), Some(BindingType::SetupRef));
    }
}
