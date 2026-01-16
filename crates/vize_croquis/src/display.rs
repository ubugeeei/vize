//! Display types for VIR (Vize Intermediate Representation) output.
//!
//! Provides human-readable TOML-like format for semantic analysis results.

use crate::css::CssTracker;
use crate::hoist::{HoistTracker, PatchFlags};
use crate::macros::MacroTracker;
use crate::optimization::OptimizationTracker;
use vize_relief::BindingType;

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Severity {
    Error = 0,
    Warning = 1,
    Info = 2,
    Hint = 3,
}

/// Related information for a diagnostic
#[derive(Debug, Clone)]
pub struct RelatedInfo {
    pub message: String,
    pub start: u32,
    pub end: u32,
}

/// A diagnostic message
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub start: u32,
    pub end: u32,
    pub code: Option<String>,
    pub related: Vec<RelatedInfo>,
}

/// Scope kind for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    Module,
    Function,
    Block,
    VFor,
    VSlot,
    EventHandler,
    Callback,
    ScriptSetup,
    NonScriptSetup,
    Universal,
    ClientOnly,
    JsGlobalUniversal,
    JsGlobalBrowser,
    JsGlobalNode,
    JsGlobalDeno,
    JsGlobalBun,
    VueGlobal,
    ExternalModule,
    Closure,
}

impl From<crate::scope::ScopeKind> for ScopeKind {
    fn from(kind: crate::scope::ScopeKind) -> Self {
        match kind {
            crate::scope::ScopeKind::Module => Self::Module,
            crate::scope::ScopeKind::Function => Self::Function,
            crate::scope::ScopeKind::Block => Self::Block,
            crate::scope::ScopeKind::VFor => Self::VFor,
            crate::scope::ScopeKind::VSlot => Self::VSlot,
            crate::scope::ScopeKind::EventHandler => Self::EventHandler,
            crate::scope::ScopeKind::Callback => Self::Callback,
            crate::scope::ScopeKind::ScriptSetup => Self::ScriptSetup,
            crate::scope::ScopeKind::NonScriptSetup => Self::NonScriptSetup,
            crate::scope::ScopeKind::Universal => Self::Universal,
            crate::scope::ScopeKind::ClientOnly => Self::ClientOnly,
            crate::scope::ScopeKind::JsGlobalUniversal => Self::JsGlobalUniversal,
            crate::scope::ScopeKind::JsGlobalBrowser => Self::JsGlobalBrowser,
            crate::scope::ScopeKind::JsGlobalNode => Self::JsGlobalNode,
            crate::scope::ScopeKind::JsGlobalDeno => Self::JsGlobalDeno,
            crate::scope::ScopeKind::JsGlobalBun => Self::JsGlobalBun,
            crate::scope::ScopeKind::VueGlobal => Self::VueGlobal,
            crate::scope::ScopeKind::ExternalModule => Self::ExternalModule,
            crate::scope::ScopeKind::Closure => Self::Closure,
        }
    }
}

impl ScopeKind {
    /// Get the display prefix for this scope kind
    ///
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
            Self::Module => "module",
            Self::Function => "fn",
            Self::Block => "block",
            Self::VFor => "v-for",
            Self::VSlot => "v-slot",
            Self::EventHandler => "event",
            Self::Callback => "callback",
            Self::ScriptSetup => "setup",
            Self::NonScriptSetup => "plain",
            Self::Universal => "universal",
            Self::ClientOnly => "client",
            Self::JsGlobalUniversal => "universal",
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
            Self::Module => "module",
            Self::Function => "fn",
            Self::Block => "block",
            Self::VFor => "v-for",
            Self::VSlot => "v-slot",
            Self::EventHandler => "event",
            Self::Callback => "callback",
            Self::ScriptSetup => "setup",
            Self::NonScriptSetup => "plain",
            Self::Universal => "universal",
            Self::ClientOnly => "client",
            Self::JsGlobalUniversal => "universal",
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

/// Binding source for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingSource {
    ScriptSetup,
    Props,
    Data,
    Computed,
    Methods,
    Inject,
    Import,
    Local,
}

/// Binding metadata for display
#[derive(Debug, Clone)]
pub struct BindingMetadata {
    pub binding_type: BindingType,
    pub source: BindingSource,
    pub is_used: bool,
    pub is_mutated: bool,
}

/// Scope display info
#[derive(Debug, Clone)]
pub struct ScopeDisplay {
    pub id: u32,
    pub kind: ScopeKind,
    pub parent_ids: Vec<u32>,
    pub start: u32,
    pub end: u32,
    pub bindings: Vec<(String, BindingMetadata)>,
}

/// Binding display info
#[derive(Debug, Clone)]
pub struct BindingDisplay {
    pub name: String,
    pub binding_type: String,
    pub source: String,
}

/// Prop display info
#[derive(Debug, Clone)]
pub struct PropDisplay {
    pub name: String,
    pub prop_type: Option<String>,
    pub required: bool,
    pub has_default: bool,
}

/// Emit display info
#[derive(Debug, Clone)]
pub struct EmitDisplay {
    pub name: String,
    pub payload_type: Option<String>,
}

/// Macro display info
#[derive(Debug, Clone)]
pub struct MacroDisplay {
    pub name: String,
    pub kind: String,
    pub start: u32,
    pub end: u32,
}

/// Hoist display info
#[derive(Debug, Clone)]
pub struct HoistDisplay {
    pub id: u32,
    pub level: String,
    pub content: String,
}

/// Selector display info
#[derive(Debug, Clone)]
pub struct SelectorDisplay {
    pub raw: String,
    pub scoped: bool,
}

/// CSS display info
#[derive(Debug, Clone)]
pub struct CssDisplay {
    pub selectors: Vec<SelectorDisplay>,
    pub v_bind_count: u32,
    pub has_deep: bool,
    pub has_slotted: bool,
    pub has_global: bool,
}

/// Patch flag display info
#[derive(Debug, Clone)]
pub struct PatchFlagDisplay {
    pub value: i32,
    pub names: Vec<String>,
}

impl From<PatchFlags> for PatchFlagDisplay {
    fn from(flags: PatchFlags) -> Self {
        Self {
            value: flags.bits(),
            names: flags.flag_names().into_iter().map(String::from).collect(),
        }
    }
}

/// Block display info
#[derive(Debug, Clone)]
pub struct BlockDisplay {
    pub id: u32,
    pub block_type: String,
    pub parent_id: Option<u32>,
    pub dynamic_children: u32,
}

/// Event cache display info
#[derive(Debug, Clone)]
pub struct EventCacheDisplay {
    pub cache_index: u32,
    pub event_name: String,
    pub handler: String,
    pub is_inline: bool,
}

/// Once cache display info
#[derive(Debug, Clone)]
pub struct OnceCacheDisplay {
    pub cache_index: u32,
    pub content: String,
    pub start: u32,
    pub end: u32,
}

/// Memo cache display info
#[derive(Debug, Clone)]
pub struct MemoCacheDisplay {
    pub cache_index: u32,
    pub deps: String,
    pub content: String,
    pub start: u32,
    pub end: u32,
}

/// Top-level await display info
#[derive(Debug, Clone)]
pub struct TopLevelAwaitDisplay {
    pub expression: String,
    pub start: u32,
    pub end: u32,
}

/// Optimization display info
#[derive(Debug, Clone)]
pub struct OptimizationDisplay {
    pub patch_flags: Vec<PatchFlagDisplay>,
    pub blocks: Vec<BlockDisplay>,
    pub event_cache: Vec<EventCacheDisplay>,
    pub once_cache: Vec<OnceCacheDisplay>,
    pub memo_cache: Vec<MemoCacheDisplay>,
}

/// Analysis statistics
#[derive(Debug, Clone, Default)]
pub struct AnalysisStats {
    pub scope_count: u32,
    pub binding_count: u32,
    pub prop_count: u32,
    pub emit_count: u32,
    pub model_count: u32,
    pub hoist_count: u32,
    pub cache_count: u32,
}

/// Complete analysis summary
#[derive(Debug, Clone)]
pub struct Croquis {
    pub scopes: Vec<ScopeDisplay>,
    pub bindings: Vec<BindingDisplay>,
    pub props: Vec<PropDisplay>,
    pub emits: Vec<EmitDisplay>,
    pub macros: Vec<MacroDisplay>,
    pub hoists: Vec<HoistDisplay>,
    pub css: Option<CssDisplay>,
    pub optimization: OptimizationDisplay,
    pub diagnostics: Vec<Diagnostic>,
    pub stats: AnalysisStats,
    pub is_async: bool,
    pub top_level_awaits: Vec<TopLevelAwaitDisplay>,
}

/// Builder for Croquis
#[derive(Debug, Default)]
pub struct SummaryBuilder {
    summary: Croquis,
}

impl Default for Croquis {
    fn default() -> Self {
        Self {
            scopes: Vec::new(),
            bindings: Vec::new(),
            props: Vec::new(),
            emits: Vec::new(),
            macros: Vec::new(),
            hoists: Vec::new(),
            css: None,
            optimization: OptimizationDisplay {
                patch_flags: Vec::new(),
                blocks: Vec::new(),
                event_cache: Vec::new(),
                once_cache: Vec::new(),
                memo_cache: Vec::new(),
            },
            diagnostics: Vec::new(),
            stats: AnalysisStats::default(),
            is_async: false,
            top_level_awaits: Vec::new(),
        }
    }
}

impl SummaryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add macro tracker data
    pub fn with_macros(mut self, tracker: &MacroTracker) -> Self {
        // Add macro calls
        for call in tracker.all_calls() {
            self.summary.macros.push(MacroDisplay {
                name: call.name.to_string(),
                kind: format!("{:?}", call.kind),
                start: call.start,
                end: call.end,
            });
        }

        // Add props
        for prop in tracker.props() {
            self.summary.props.push(PropDisplay {
                name: prop.name.to_string(),
                prop_type: prop.prop_type.as_ref().map(|s| s.to_string()),
                required: prop.required,
                has_default: prop.default_value.is_some(),
            });
        }

        // Add emits
        for emit in tracker.emits() {
            self.summary.emits.push(EmitDisplay {
                name: emit.name.to_string(),
                payload_type: emit.payload_type.as_ref().map(|s| s.to_string()),
            });
        }

        // Add top-level awaits
        self.summary.is_async = tracker.is_async();
        for await_expr in tracker.top_level_awaits() {
            self.summary.top_level_awaits.push(TopLevelAwaitDisplay {
                expression: await_expr.expression.to_string(),
                start: await_expr.start,
                end: await_expr.end,
            });
        }

        self.summary.stats.prop_count = tracker.props().len() as u32;
        self.summary.stats.emit_count = tracker.emits().len() as u32;
        self.summary.stats.model_count = tracker.models().len() as u32;

        self
    }

    /// Add optimization tracker data
    pub fn with_optimization(mut self, tracker: &OptimizationTracker) -> Self {
        // Add blocks
        for block in tracker.blocks() {
            self.summary.optimization.blocks.push(BlockDisplay {
                id: block.id,
                block_type: format!("{:?}", block.block_type),
                parent_id: block.parent_id,
                dynamic_children: block.dynamic_children_count,
            });
        }

        // Add event cache
        for event in tracker.event_cache() {
            self.summary
                .optimization
                .event_cache
                .push(EventCacheDisplay {
                    cache_index: event.cache_index,
                    event_name: event.event_name.to_string(),
                    handler: event.handler.to_string(),
                    is_inline: event.is_inline,
                });
        }

        // Add once cache
        for once in tracker.once_cache() {
            self.summary.optimization.once_cache.push(OnceCacheDisplay {
                cache_index: once.cache_index,
                content: once.content.to_string(),
                start: once.start,
                end: once.end,
            });
        }

        // Add memo cache
        for memo in tracker.memo_cache() {
            self.summary.optimization.memo_cache.push(MemoCacheDisplay {
                cache_index: memo.cache_index,
                deps: memo.deps.to_string(),
                content: memo.content.to_string(),
                start: memo.start,
                end: memo.end,
            });
        }

        self.summary.stats.cache_count = tracker.current_cache_index();

        self
    }

    /// Add hoist tracker data
    pub fn with_hoists(mut self, tracker: &HoistTracker) -> Self {
        for hoist in tracker.hoists() {
            self.summary.hoists.push(HoistDisplay {
                id: hoist.id.as_u32(),
                level: format!("{:?}", hoist.level),
                content: hoist.content.to_string(),
            });
        }

        self.summary.stats.hoist_count = tracker.count() as u32;

        self
    }

    /// Add CSS tracker data
    pub fn with_css(mut self, tracker: &CssTracker) -> Self {
        let stats = tracker.stats();

        self.summary.css = Some(CssDisplay {
            selectors: tracker
                .selectors()
                .iter()
                .map(|s| SelectorDisplay {
                    raw: s.raw.to_string(),
                    scoped: true, // Assume scoped by default
                })
                .collect(),
            v_bind_count: stats.v_bind_count,
            has_deep: stats.deep_selectors > 0,
            has_slotted: stats.slotted_selectors > 0,
            has_global: stats.global_selectors > 0,
        });

        self
    }

    /// Add diagnostic
    pub fn add_diagnostic(mut self, diagnostic: Diagnostic) -> Self {
        self.summary.diagnostics.push(diagnostic);
        self
    }

    /// Build the summary
    pub fn build(self) -> Croquis {
        self.summary
    }
}

impl Croquis {
    /// Convert to VIR (TOML-like) format
    pub fn to_vir(&self) -> String {
        let mut output = String::with_capacity(4096);

        output.push_str("[analysis]\n");
        output.push_str(&format!("is_async = {}\n", self.is_async));
        output.push_str(&format!("scope_count = {}\n", self.stats.scope_count));
        output.push_str(&format!("binding_count = {}\n", self.stats.binding_count));
        output.push('\n');

        // Scopes
        if !self.scopes.is_empty() {
            output.push_str("[scopes]\n");

            // Assign display IDs per prefix type (separate counters for #, ~, !)
            use std::collections::HashMap;
            let mut prefix_counters: HashMap<&str, u32> = HashMap::new();
            prefix_counters.insert("#", 0);
            prefix_counters.insert("~", 0);
            prefix_counters.insert("!", 0);

            // Map internal id -> (prefix, display_id)
            let mut display_id_map: HashMap<u32, (String, u32)> = HashMap::new();
            for scope in &self.scopes {
                let prefix = scope.kind.prefix();
                let counter = prefix_counters.entry(prefix).or_insert(0);
                let display_id = *counter;
                *counter += 1;
                display_id_map.insert(scope.id, (prefix.to_string(), display_id));
            }

            for scope in &self.scopes {
                let (prefix, display_id) = display_id_map.get(&scope.id).unwrap();
                // Format: prefix+display_id kind @start:end {bindings} $ parent_refs
                output.push_str(&format!(
                    "{}{} {} @{}:{}",
                    prefix,
                    display_id,
                    scope.kind.to_display(),
                    scope.start,
                    scope.end
                ));

                // Bindings
                if !scope.bindings.is_empty() {
                    output.push_str(" {");
                    let mut first = true;
                    for (name, _) in &scope.bindings {
                        if !first {
                            output.push_str(", ");
                        }
                        output.push_str(name);
                        first = false;
                    }
                    output.push('}');
                }

                // Parent references (at the end)
                if !scope.parent_ids.is_empty() {
                    output.push_str(" $ ");
                    let mut first = true;
                    for parent_id in &scope.parent_ids {
                        if !first {
                            output.push_str(", ");
                        }
                        if let Some((p_prefix, p_display_id)) = display_id_map.get(parent_id) {
                            output.push_str(&format!("{}{}", p_prefix, p_display_id));
                        } else {
                            output.push_str(&format!("#{}", parent_id));
                        }
                        first = false;
                    }
                }
                output.push('\n');
            }
            output.push('\n');
        }

        // Props
        if !self.props.is_empty() {
            output.push_str("[props]\n");
            for prop in &self.props {
                output.push_str(&format!(
                    "  {} = {{ type = {:?}, required = {} }}\n",
                    prop.name,
                    prop.prop_type.as_deref().unwrap_or("any"),
                    prop.required
                ));
            }
            output.push('\n');
        }

        // Emits
        if !self.emits.is_empty() {
            output.push_str("[emits]\n");
            for emit in &self.emits {
                output.push_str(&format!("  {} = {:?}\n", emit.name, emit.payload_type));
            }
            output.push('\n');
        }

        // Top-level awaits
        if !self.top_level_awaits.is_empty() {
            output.push_str("[top_level_await]\n");
            for await_expr in &self.top_level_awaits {
                output.push_str(&format!(
                    "  {{ expression = \"{}\", span = [{}, {}] }}\n",
                    await_expr.expression, await_expr.start, await_expr.end
                ));
            }
            output.push('\n');
        }

        // Event cache
        if !self.optimization.event_cache.is_empty() {
            output.push_str("[event_cache]\n");
            for event in &self.optimization.event_cache {
                output.push_str(&format!(
                    "  _cache[{}] = {{ event = \"{}\", handler = \"{}\" }}\n",
                    event.cache_index, event.event_name, event.handler
                ));
            }
            output.push('\n');
        }

        // Blocks
        if !self.optimization.blocks.is_empty() {
            output.push_str("[blocks]\n");
            for block in &self.optimization.blocks {
                output.push_str(&format!(
                    "  block_{} = {{ type = \"{}\", dynamic_children = {} }}\n",
                    block.id, block.block_type, block.dynamic_children
                ));
            }
            output.push('\n');
        }

        // Diagnostics
        if !self.diagnostics.is_empty() {
            output.push_str("[diagnostics]\n");
            for diag in &self.diagnostics {
                let severity = match diag.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                    Severity::Hint => "hint",
                };
                output.push_str(&format!(
                    "  {{ severity = \"{}\", message = \"{}\" }}\n",
                    severity, diag.message
                ));
            }
        }

        output
    }
}
