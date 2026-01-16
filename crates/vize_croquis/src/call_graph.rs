//! Function call graph for tracking Vue API calls and composables.
//!
//! Tracks whether Vue APIs (ref, reactive, provide, inject, etc.) are called
//! within the setup context, either directly or through composable functions.
//!
//! ## Key Features
//!
//! - Tracks function definitions and their containing scopes
//! - Tracks Vue API calls (ref, reactive, computed, provide, inject, watch, etc.)
//! - Tracks composable function calls (use* pattern)
//! - Validates that Vue APIs are called within appropriate contexts
//!
//! ## Performance
//!
//! - Uses FxHashMap for O(1) lookups
//! - SmallVec for typical small collections
//! - Minimal allocations during analysis

use vize_carton::{CompactString, FxHashMap, FxHashSet, SmallVec};

use crate::scope::ScopeId;

/// Unique identifier for a function in the call graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FunctionId(u32);

impl FunctionId {
    /// Create a new function ID.
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[inline(always)]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Category of Vue API function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VueApiCategory {
    /// Reactivity primitives: ref, reactive, computed, etc.
    Reactivity,
    /// Lifecycle hooks: onMounted, onUnmounted, etc.
    Lifecycle,
    /// Dependency injection: provide, inject
    DependencyInjection,
    /// Watchers: watch, watchEffect, watchPostEffect, watchSyncEffect
    Watcher,
    /// Template refs: useTemplateRef
    TemplateRef,
    /// Other Vue APIs: nextTick, defineComponent, etc.
    Other,
}

/// A Vue API call detected in the code.
#[derive(Debug, Clone)]
pub struct VueApiCall {
    /// Name of the API function (e.g., "ref", "provide", "onMounted").
    pub name: CompactString,
    /// Category of the API.
    pub category: VueApiCategory,
    /// Scope where this call occurs.
    pub scope_id: ScopeId,
    /// Containing function (if inside a function).
    pub containing_function: Option<FunctionId>,
    /// Whether this call is inside the setup context (directly or transitively).
    pub in_setup_context: bool,
    /// Source offset.
    pub start: u32,
    pub end: u32,
}

/// A function definition in the code.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    /// Function ID.
    pub id: FunctionId,
    /// Function name (None for anonymous functions).
    pub name: Option<CompactString>,
    /// Scope where this function is defined.
    pub scope_id: ScopeId,
    /// Parent function (if nested).
    pub parent_function: Option<FunctionId>,
    /// Whether this is an arrow function.
    pub is_arrow: bool,
    /// Whether this function is called within setup context.
    pub called_in_setup: bool,
    /// Whether this function uses Vue APIs.
    pub uses_vue_apis: bool,
    /// Whether this is a composable (use* pattern and uses Vue APIs).
    pub is_composable: bool,
    /// Source offset.
    pub start: u32,
    pub end: u32,
}

/// A composable function call (use* pattern).
#[derive(Debug, Clone)]
pub struct ComposableCallInfo {
    /// Name of the composable function.
    pub name: CompactString,
    /// Import source (if imported).
    pub source: Option<CompactString>,
    /// Scope where this call occurs.
    pub scope_id: ScopeId,
    /// Containing function (if inside a function).
    pub containing_function: Option<FunctionId>,
    /// Whether this composable is called in setup context.
    pub in_setup_context: bool,
    /// Local binding name (if assigned).
    pub local_binding: Option<CompactString>,
    /// Vue APIs used by this composable (if known from analysis).
    pub vue_apis_used: SmallVec<[CompactString; 4]>,
    /// Source offset.
    pub start: u32,
    pub end: u32,
}

/// Call edge between two functions.
#[derive(Debug, Clone)]
pub struct CallEdge {
    /// Caller function.
    pub caller: FunctionId,
    /// Callee function.
    pub callee: FunctionId,
    /// Call site offset.
    pub call_site: u32,
}

/// Kind of setup context entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SetupContextKind {
    /// Direct setup function body (`<script setup>` or `setup()` function).
    SetupBody,
    /// Inside a composable called from setup.
    Composable,
    /// Inside a callback passed to a composable (e.g., computed callback).
    ComposableCallback,
    /// Not in setup context.
    None,
}

/// Tracks function calls and Vue API usage.
#[derive(Debug, Default)]
pub struct CallGraph {
    /// All function definitions.
    functions: Vec<FunctionDef>,
    /// Vue API calls.
    vue_api_calls: Vec<VueApiCall>,
    /// Composable calls.
    composable_calls: Vec<ComposableCallInfo>,
    /// Call edges between functions.
    call_edges: Vec<CallEdge>,
    /// Map from function name to function IDs (for resolution).
    function_by_name: FxHashMap<CompactString, SmallVec<[FunctionId; 2]>>,
    /// Functions that are part of setup context (directly or transitively).
    setup_context_functions: FxHashSet<FunctionId>,
    /// The setup function ID (if found).
    setup_function: Option<FunctionId>,
    /// Next function ID.
    next_id: u32,
}

impl CallGraph {
    /// Create a new empty call graph.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(functions: usize, calls: usize) -> Self {
        Self {
            functions: Vec::with_capacity(functions),
            vue_api_calls: Vec::with_capacity(calls),
            composable_calls: Vec::with_capacity(calls / 4),
            call_edges: Vec::with_capacity(calls),
            function_by_name: FxHashMap::default(),
            setup_context_functions: FxHashSet::default(),
            setup_function: None,
            next_id: 0,
        }
    }

    /// Add a function definition.
    pub fn add_function(
        &mut self,
        name: Option<CompactString>,
        scope_id: ScopeId,
        parent_function: Option<FunctionId>,
        is_arrow: bool,
        start: u32,
        end: u32,
    ) -> FunctionId {
        let id = FunctionId::new(self.next_id);
        self.next_id += 1;

        // Check if this looks like a composable
        let is_composable = name
            .as_ref()
            .map(|n| n.starts_with("use") && n.len() > 3)
            .unwrap_or(false);

        let def = FunctionDef {
            id,
            name: name.clone(),
            scope_id,
            parent_function,
            is_arrow,
            called_in_setup: false,
            uses_vue_apis: false,
            is_composable,
            start,
            end,
        };

        self.functions.push(def);

        // Index by name
        if let Some(name) = name {
            self.function_by_name.entry(name).or_default().push(id);
        }

        id
    }

    /// Mark a function as the setup function.
    #[inline]
    pub fn set_setup_function(&mut self, id: FunctionId) {
        self.setup_function = Some(id);
        self.setup_context_functions.insert(id);
    }

    /// Add a Vue API call.
    pub fn add_vue_api_call(
        &mut self,
        name: CompactString,
        scope_id: ScopeId,
        containing_function: Option<FunctionId>,
        start: u32,
        end: u32,
    ) {
        let category = categorize_vue_api(&name);
        let in_setup_context = self.is_in_setup_context(containing_function);

        self.vue_api_calls.push(VueApiCall {
            name,
            category,
            scope_id,
            containing_function,
            in_setup_context,
            start,
            end,
        });

        // Mark containing function as using Vue APIs
        if let Some(func_id) = containing_function {
            if let Some(func) = self.functions.get_mut(func_id.as_u32() as usize) {
                func.uses_vue_apis = true;
            }
        }
    }

    /// Add a composable call.
    #[allow(clippy::too_many_arguments)]
    pub fn add_composable_call(
        &mut self,
        name: CompactString,
        source: Option<CompactString>,
        scope_id: ScopeId,
        containing_function: Option<FunctionId>,
        local_binding: Option<CompactString>,
        start: u32,
        end: u32,
    ) {
        let in_setup_context = self.is_in_setup_context(containing_function);

        self.composable_calls.push(ComposableCallInfo {
            name,
            source,
            scope_id,
            containing_function,
            in_setup_context,
            local_binding,
            vue_apis_used: SmallVec::new(),
            start,
            end,
        });
    }

    /// Add a call edge between functions.
    pub fn add_call_edge(&mut self, caller: FunctionId, callee: FunctionId, call_site: u32) {
        self.call_edges.push(CallEdge {
            caller,
            callee,
            call_site,
        });

        // If caller is in setup context, callee is too
        if self.setup_context_functions.contains(&caller) {
            self.setup_context_functions.insert(callee);
            if let Some(func) = self.functions.get_mut(callee.as_u32() as usize) {
                func.called_in_setup = true;
            }
        }
    }

    /// Check if a function (or None for top-level) is in setup context.
    #[inline]
    pub fn is_in_setup_context(&self, func_id: Option<FunctionId>) -> bool {
        match func_id {
            Some(id) => self.setup_context_functions.contains(&id),
            None => {
                // Top-level in script setup is setup context
                true
            }
        }
    }

    /// Propagate setup context through call edges.
    /// Call this after all functions and edges are added.
    pub fn propagate_setup_context(&mut self) {
        if self.setup_function.is_none() {
            return;
        }

        // BFS from setup function
        let mut queue: SmallVec<[FunctionId; 16]> = SmallVec::new();
        queue.extend(self.setup_context_functions.iter().copied());

        while let Some(func_id) = queue.pop() {
            // Find all functions called by this function
            for edge in &self.call_edges {
                if edge.caller == func_id && !self.setup_context_functions.contains(&edge.callee) {
                    self.setup_context_functions.insert(edge.callee);
                    queue.push(edge.callee);

                    // Update function def
                    if let Some(func) = self.functions.get_mut(edge.callee.as_u32() as usize) {
                        func.called_in_setup = true;
                    }
                }
            }
        }

        // Update vue_api_calls in_setup_context
        // Collect updates first to avoid borrow conflict
        let vue_updates: Vec<_> = self
            .vue_api_calls
            .iter()
            .enumerate()
            .map(|(i, call)| {
                (
                    i,
                    self.setup_context_functions.contains(
                        &call
                            .containing_function
                            .unwrap_or(FunctionId::new(u32::MAX)),
                    ),
                )
            })
            .collect();
        for (i, in_setup) in vue_updates {
            // Top-level is always in setup context for script setup
            let containing = self.vue_api_calls[i].containing_function;
            self.vue_api_calls[i].in_setup_context = containing.is_none() || in_setup;
        }

        // Update composable_calls in_setup_context
        let composable_updates: Vec<_> = self
            .composable_calls
            .iter()
            .enumerate()
            .map(|(i, call)| {
                (
                    i,
                    self.setup_context_functions.contains(
                        &call
                            .containing_function
                            .unwrap_or(FunctionId::new(u32::MAX)),
                    ),
                )
            })
            .collect();
        for (i, in_setup) in composable_updates {
            let containing = self.composable_calls[i].containing_function;
            self.composable_calls[i].in_setup_context = containing.is_none() || in_setup;
        }
    }

    /// Get all Vue API calls.
    #[inline]
    pub fn vue_api_calls(&self) -> &[VueApiCall] {
        &self.vue_api_calls
    }

    /// Get Vue API calls outside setup context (potential issues).
    pub fn vue_api_calls_outside_setup(&self) -> impl Iterator<Item = &VueApiCall> {
        self.vue_api_calls.iter().filter(|c| !c.in_setup_context)
    }

    /// Get all composable calls.
    #[inline]
    pub fn composable_calls(&self) -> &[ComposableCallInfo] {
        &self.composable_calls
    }

    /// Get composable calls outside setup context (potential issues).
    pub fn composable_calls_outside_setup(&self) -> impl Iterator<Item = &ComposableCallInfo> {
        self.composable_calls.iter().filter(|c| !c.in_setup_context)
    }

    /// Get all function definitions.
    #[inline]
    pub fn functions(&self) -> &[FunctionDef] {
        &self.functions
    }

    /// Get a function by ID.
    #[inline]
    pub fn get_function(&self, id: FunctionId) -> Option<&FunctionDef> {
        self.functions.get(id.as_u32() as usize)
    }

    /// Get functions by name.
    pub fn get_functions_by_name(&self, name: &str) -> Option<&[FunctionId]> {
        self.function_by_name.get(name).map(|v| v.as_slice())
    }

    /// Get all call edges.
    #[inline]
    pub fn call_edges(&self) -> &[CallEdge] {
        &self.call_edges
    }

    /// Get the setup function ID.
    #[inline]
    pub fn setup_function(&self) -> Option<FunctionId> {
        self.setup_function
    }

    /// Check if a function is a composable.
    #[inline]
    pub fn is_composable(&self, id: FunctionId) -> bool {
        self.get_function(id)
            .map(|f| f.is_composable)
            .unwrap_or(false)
    }

    /// Get the setup context kind for a given location.
    pub fn get_setup_context_kind(&self, func_id: Option<FunctionId>) -> SetupContextKind {
        match func_id {
            None => {
                // Top-level - check if we have a setup function
                if self.setup_function.is_some() {
                    SetupContextKind::SetupBody
                } else {
                    SetupContextKind::None
                }
            }
            Some(id) => {
                if Some(id) == self.setup_function {
                    SetupContextKind::SetupBody
                } else if self.setup_context_functions.contains(&id) {
                    // Check if this function is a composable
                    if self.is_composable(id) {
                        SetupContextKind::Composable
                    } else {
                        // It's a callback or nested function in setup context
                        SetupContextKind::ComposableCallback
                    }
                } else {
                    SetupContextKind::None
                }
            }
        }
    }

    /// Generate a markdown visualization of the call graph.
    pub fn to_markdown(&self) -> String {
        let mut out = String::with_capacity(2048);

        out.push_str("## Function Call Graph\n\n");

        // Setup function
        if let Some(setup_id) = self.setup_function {
            if let Some(func) = self.get_function(setup_id) {
                out.push_str(&format!(
                    "**Setup Function**: `{}` (offset: {}..{})\n\n",
                    func.name.as_deref().unwrap_or("<anonymous>"),
                    func.start,
                    func.end
                ));
            }
        }

        // Functions in setup context
        out.push_str("### Functions in Setup Context\n\n");
        for func in &self.functions {
            if func.called_in_setup || Some(func.id) == self.setup_function {
                let marker = if func.is_composable {
                    "🔧"
                } else if func.uses_vue_apis {
                    "⚡"
                } else {
                    "📦"
                };
                out.push_str(&format!(
                    "- {} `{}` ({}..{})\n",
                    marker,
                    func.name.as_deref().unwrap_or("<anonymous>"),
                    func.start,
                    func.end
                ));
            }
        }

        // Vue API calls
        out.push_str("\n### Vue API Calls\n\n");
        out.push_str("| API | Category | In Setup | Offset |\n");
        out.push_str("|-----|----------|----------|--------|\n");
        for call in &self.vue_api_calls {
            let in_setup = if call.in_setup_context { "✅" } else { "❌" };
            out.push_str(&format!(
                "| `{}` | {:?} | {} | {}..{} |\n",
                call.name, call.category, in_setup, call.start, call.end
            ));
        }

        // Composable calls
        if !self.composable_calls.is_empty() {
            out.push_str("\n### Composable Calls\n\n");
            out.push_str("| Composable | Source | In Setup | Offset |\n");
            out.push_str("|------------|--------|----------|--------|\n");
            for call in &self.composable_calls {
                let in_setup = if call.in_setup_context { "✅" } else { "❌" };
                let source = call.source.as_deref().unwrap_or("-");
                out.push_str(&format!(
                    "| `{}` | `{}` | {} | {}..{} |\n",
                    call.name, source, in_setup, call.start, call.end
                ));
            }
        }

        // Issues (Vue APIs outside setup)
        let issues: Vec<_> = self.vue_api_calls_outside_setup().collect();
        if !issues.is_empty() {
            out.push_str("\n### ⚠️ Issues: Vue APIs Outside Setup Context\n\n");
            for call in issues {
                out.push_str(&format!(
                    "- `{}` at {}..{} - Vue {} API called outside setup context\n",
                    call.name,
                    call.start,
                    call.end,
                    match call.category {
                        VueApiCategory::Reactivity => "reactivity",
                        VueApiCategory::Lifecycle => "lifecycle",
                        VueApiCategory::DependencyInjection => "dependency injection",
                        VueApiCategory::Watcher => "watcher",
                        VueApiCategory::TemplateRef => "template ref",
                        VueApiCategory::Other => "",
                    }
                ));
            }
        }

        out
    }
}

/// Categorize a Vue API by name.
fn categorize_vue_api(name: &str) -> VueApiCategory {
    match name {
        // Reactivity
        "ref" | "shallowRef" | "triggerRef" | "customRef" | "reactive" | "shallowReactive"
        | "readonly" | "shallowReadonly" | "computed" | "toRef" | "toRefs" | "toValue"
        | "toRaw" | "markRaw" | "isRef" | "isReactive" | "isReadonly" | "isProxy" | "unref" => {
            VueApiCategory::Reactivity
        }

        // Lifecycle
        "onMounted" | "onUpdated" | "onUnmounted" | "onBeforeMount" | "onBeforeUpdate"
        | "onBeforeUnmount" | "onErrorCaptured" | "onRenderTracked" | "onRenderTriggered"
        | "onActivated" | "onDeactivated" | "onServerPrefetch" => VueApiCategory::Lifecycle,

        // Dependency Injection
        "provide" | "inject" | "hasInjectionContext" => VueApiCategory::DependencyInjection,

        // Watchers
        "watch" | "watchEffect" | "watchPostEffect" | "watchSyncEffect" => VueApiCategory::Watcher,

        // Template Refs
        "useTemplateRef" => VueApiCategory::TemplateRef,

        // Other
        _ => VueApiCategory::Other,
    }
}

/// Check if a function name looks like a composable.
#[inline]
pub fn is_composable_name(name: &str) -> bool {
    name.starts_with("use")
        && name.len() > 3
        && name
            .chars()
            .nth(3)
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
}

/// Check if a name is a Vue API.
#[inline]
pub fn is_vue_api(name: &str) -> bool {
    matches!(
        name,
        "ref"
            | "shallowRef"
            | "triggerRef"
            | "customRef"
            | "reactive"
            | "shallowReactive"
            | "readonly"
            | "shallowReadonly"
            | "computed"
            | "toRef"
            | "toRefs"
            | "toValue"
            | "toRaw"
            | "markRaw"
            | "isRef"
            | "isReactive"
            | "isReadonly"
            | "isProxy"
            | "unref"
            | "onMounted"
            | "onUpdated"
            | "onUnmounted"
            | "onBeforeMount"
            | "onBeforeUpdate"
            | "onBeforeUnmount"
            | "onErrorCaptured"
            | "onRenderTracked"
            | "onRenderTriggered"
            | "onActivated"
            | "onDeactivated"
            | "onServerPrefetch"
            | "provide"
            | "inject"
            | "hasInjectionContext"
            | "watch"
            | "watchEffect"
            | "watchPostEffect"
            | "watchSyncEffect"
            | "useTemplateRef"
            | "nextTick"
            | "defineComponent"
            | "defineAsyncComponent"
            | "defineCustomElement"
            | "getCurrentInstance"
            | "useSlots"
            | "useAttrs"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_vue_api() {
        assert_eq!(categorize_vue_api("ref"), VueApiCategory::Reactivity);
        assert_eq!(categorize_vue_api("computed"), VueApiCategory::Reactivity);
        assert_eq!(categorize_vue_api("onMounted"), VueApiCategory::Lifecycle);
        assert_eq!(
            categorize_vue_api("provide"),
            VueApiCategory::DependencyInjection
        );
        assert_eq!(categorize_vue_api("watch"), VueApiCategory::Watcher);
        assert_eq!(
            categorize_vue_api("useTemplateRef"),
            VueApiCategory::TemplateRef
        );
        assert_eq!(categorize_vue_api("nextTick"), VueApiCategory::Other);
    }

    #[test]
    fn test_is_composable_name() {
        assert!(is_composable_name("useCounter"));
        assert!(is_composable_name("useAuth"));
        assert!(is_composable_name("useFetch"));
        assert!(!is_composable_name("use")); // Too short
        assert!(!is_composable_name("usecounter")); // Lowercase after use
        assert!(!is_composable_name("counter")); // Doesn't start with use
    }

    #[test]
    fn test_is_vue_api() {
        assert!(is_vue_api("ref"));
        assert!(is_vue_api("reactive"));
        assert!(is_vue_api("computed"));
        assert!(is_vue_api("onMounted"));
        assert!(is_vue_api("provide"));
        assert!(is_vue_api("inject"));
        assert!(is_vue_api("watch"));
        assert!(!is_vue_api("myFunction"));
        assert!(!is_vue_api("useState")); // React API, not Vue
    }

    #[test]
    fn test_call_graph_basic() {
        let mut graph = CallGraph::new();

        // Add setup function
        let setup_id = graph.add_function(
            Some(CompactString::new("setup")),
            ScopeId::new(1),
            None,
            false,
            0,
            100,
        );
        graph.set_setup_function(setup_id);

        // Add a helper function
        let helper_id = graph.add_function(
            Some(CompactString::new("useCounter")),
            ScopeId::new(2),
            None,
            false,
            110,
            200,
        );

        // Add call edge: setup -> useCounter
        graph.add_call_edge(setup_id, helper_id, 50);

        // Add Vue API call in helper
        graph.add_vue_api_call(
            CompactString::new("ref"),
            ScopeId::new(2),
            Some(helper_id),
            150,
            155,
        );

        // Propagate setup context
        graph.propagate_setup_context();

        // Verify
        assert!(graph.is_in_setup_context(Some(setup_id)));
        assert!(graph.is_in_setup_context(Some(helper_id)));

        let func = graph.get_function(helper_id).unwrap();
        assert!(func.called_in_setup);
        assert!(func.uses_vue_apis);
        assert!(func.is_composable);
    }

    #[test]
    fn test_vue_api_outside_setup() {
        let mut graph = CallGraph::new();

        // Add setup function
        let setup_id = graph.add_function(
            Some(CompactString::new("setup")),
            ScopeId::new(1),
            None,
            false,
            0,
            100,
        );
        graph.set_setup_function(setup_id);

        // Add a function NOT called from setup
        let outside_id = graph.add_function(
            Some(CompactString::new("outsideFunction")),
            ScopeId::new(2),
            None,
            false,
            200,
            300,
        );

        // Add Vue API call in the outside function
        graph.add_vue_api_call(
            CompactString::new("ref"),
            ScopeId::new(2),
            Some(outside_id),
            250,
            255,
        );

        // Propagate
        graph.propagate_setup_context();

        // Verify the issue is detected
        let issues: Vec<_> = graph.vue_api_calls_outside_setup().collect();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].name.as_str(), "ref");
    }

    #[test]
    fn test_setup_context_kind() {
        let mut graph = CallGraph::new();

        let setup_id = graph.add_function(
            Some(CompactString::new("setup")),
            ScopeId::new(1),
            None,
            false,
            0,
            100,
        );
        graph.set_setup_function(setup_id);

        let composable_id = graph.add_function(
            Some(CompactString::new("useAuth")),
            ScopeId::new(2),
            None,
            false,
            110,
            200,
        );
        graph.add_call_edge(setup_id, composable_id, 50);

        let callback_id =
            graph.add_function(None, ScopeId::new(3), Some(composable_id), true, 150, 180);
        graph.add_call_edge(composable_id, callback_id, 160);

        graph.propagate_setup_context();

        assert_eq!(
            graph.get_setup_context_kind(Some(setup_id)),
            SetupContextKind::SetupBody
        );
        assert_eq!(
            graph.get_setup_context_kind(Some(composable_id)),
            SetupContextKind::Composable
        );
        assert_eq!(
            graph.get_setup_context_kind(Some(callback_id)),
            SetupContextKind::ComposableCallback
        );
    }
}
