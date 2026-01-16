//! Cross-File Reactivity Tracking.
//!
//! NOTE: This module is under active development. Many items are reserved
//! for future cross-file analysis features.
#![allow(unused)]
//!
//! Tracks reactive values across module boundaries, including:
//! - Composable exports/imports
//! - Provide/inject chains
//! - Props passed between components
//! - Pinia store usage across components
//!
//! ## Design
//!
//! This analyzer builds a "Reactivity Flow Graph" that tracks how reactive
//! values flow between files. It detects when reactivity is accidentally
//! broken at module boundaries.
//!
//! ```text
//! useCounter.ts ──export──> Component.vue
//!      │                         │
//!      └── ref(0) ───────────> const { count } = useCounter()
//!                                    ↑
//!                               REACTIVITY LOST!
//! ```

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::{DependencyEdge, DependencyGraph};
use crate::cross_file::registry::{FileId, ModuleRegistry};
use crate::reactivity::ReactiveKind;
use vize_carton::{CompactString, FxHashMap, FxHashSet, SmallVec};

/// Unique identifier for a reactive value across the codebase.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReactiveValueId {
    /// File where the value is defined.
    pub file_id: FileId,
    /// Name of the binding.
    pub name: CompactString,
    /// Declaration offset for disambiguation.
    pub offset: u32,
}

/// How a reactive value is exposed for cross-file usage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactiveExposure {
    /// Exported from a module (composable return, named export).
    Export { export_name: CompactString },
    /// Provided via provide().
    Provide { key: CompactString },
    /// Passed as props to child component.
    Props {
        component_name: CompactString,
        prop_name: CompactString,
    },
    /// Exposed via Pinia store.
    PiniaStore {
        store_name: CompactString,
        property: CompactString,
    },
    /// Returned from composable function.
    ComposableReturn { composable_name: CompactString },
}

/// How a reactive value is consumed from another file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactiveConsumption {
    /// Imported from a module.
    Import {
        source_file: FileId,
        import_name: CompactString,
    },
    /// Injected via inject().
    Inject { key: CompactString },
    /// Received as props.
    Props { prop_name: CompactString },
    /// Used from Pinia store.
    PiniaStore { store_name: CompactString },
    /// Returned from composable call.
    ComposableCall {
        composable_name: CompactString,
        source_file: Option<FileId>,
    },
}

/// A tracked reactive value in the cross-file graph.
#[derive(Debug, Clone)]
pub struct CrossFileReactiveValue {
    /// Unique identifier.
    pub id: ReactiveValueId,
    /// Kind of reactive value (Ref, Reactive, Computed, etc.).
    pub kind: ReactiveKind,
    /// How it's exposed (if at all).
    pub exposures: SmallVec<[ReactiveExposure; 2]>,
    /// How it's consumed in other files.
    pub consumptions: SmallVec<[ReactiveConsumption; 2]>,
    /// Whether this value's reactivity was verified to flow correctly.
    pub reactivity_preserved: bool,
}

/// A flow of reactivity between files.
#[derive(Debug, Clone)]
pub struct ReactivityFlow {
    /// Source of the reactive value.
    pub source: ReactiveValueId,
    /// Target where it's consumed.
    pub target: ReactiveValueId,
    /// How the flow occurs.
    pub flow_kind: ReactivityFlowKind,
    /// Whether reactivity is preserved in this flow.
    pub preserved: bool,
    /// Reason if reactivity is lost.
    pub loss_reason: Option<ReactivityLossReason>,
}

/// Kind of reactivity flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactivityFlowKind {
    /// Composable export → import.
    ComposableExport,
    /// Provide → inject.
    ProvideInject,
    /// Parent props → child props.
    PropsFlow,
    /// Pinia store → consumer.
    StoreFlow,
    /// Direct module import.
    ModuleImport,
}

/// Reason why reactivity was lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactivityLossReason {
    /// Destructured at consumption site.
    Destructured { props: Vec<CompactString> },
    /// Spread operator used.
    Spread,
    /// Assigned to non-reactive variable.
    NonReactiveAssignment,
    /// Passed through non-reactive intermediate.
    NonReactiveIntermediate { intermediate: CompactString },
    /// Value extracted without toRef/toRefs.
    DirectExtraction,
    /// Composable return destructured without toRefs.
    ComposableDestructure,
    /// Store destructured without storeToRefs.
    StoreDestructure,
    /// Inject result destructured.
    InjectDestructure,
}

/// Cross-file reactivity issue.
#[derive(Debug, Clone)]
pub struct CrossFileReactivityIssue {
    /// File where the issue is detected.
    pub file_id: FileId,
    /// Kind of issue.
    pub kind: CrossFileReactivityIssueKind,
    /// Offset in source.
    pub offset: u32,
    /// Related file (source of reactive value).
    pub related_file: Option<FileId>,
    /// Severity.
    pub severity: DiagnosticSeverity,
}

/// Kind of cross-file reactivity issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossFileReactivityIssueKind {
    /// Composable return value destructured.
    ComposableReturnDestructured {
        composable_name: CompactString,
        destructured_props: Vec<CompactString>,
    },
    /// Injected value destructured.
    InjectValueDestructured {
        key: CompactString,
        destructured_props: Vec<CompactString>,
    },
    /// Pinia store destructured without storeToRefs.
    StoreDestructured {
        store_name: CompactString,
        destructured_props: Vec<CompactString>,
    },
    /// Props destructured without toRefs.
    PropsDestructured {
        destructured_props: Vec<CompactString>,
    },
    /// Provide value is not reactive.
    NonReactiveProvide { key: CompactString },
    /// Reactive value lost in prop chain.
    ReactivityLostInPropChain {
        prop_name: CompactString,
        parent_component: CompactString,
    },
    /// Composable exports non-reactive value.
    ComposableExportsNonReactive {
        composable_name: CompactString,
        property: CompactString,
    },
    /// Ref passed where reactive object expected.
    RefReactiveTypeMismatch {
        expected: CompactString,
        actual: CompactString,
    },
    /// Reactive value escapes module scope unsafely.
    ReactiveEscapeUnsafe {
        value_name: CompactString,
        escape_target: CompactString,
    },
    /// Circular reactive dependency detected.
    CircularReactiveDependency { cycle: Vec<CompactString> },
    /// Stale closure captures reactive value.
    StaleClosureCapture {
        value_name: CompactString,
        closure_context: CompactString,
    },
}

/// The cross-file reactivity analyzer.
pub struct CrossFileReactivityAnalyzer<'a> {
    registry: &'a ModuleRegistry,
    graph: &'a DependencyGraph,
    /// All tracked reactive values.
    reactive_values: FxHashMap<ReactiveValueId, CrossFileReactiveValue>,
    /// Reactivity flows between files.
    flows: Vec<ReactivityFlow>,
    /// Detected issues.
    issues: Vec<CrossFileReactivityIssue>,
    /// Composable definitions (file -> composable name -> return type info).
    composables: FxHashMap<FileId, Vec<ComposableInfo>>,
    /// Provide definitions across all files.
    provides: FxHashMap<CompactString, ProvideDefinition>,
}

/// Information about a composable function.
#[derive(Debug, Clone)]
struct ComposableInfo {
    name: CompactString,
    /// Returned reactive values.
    reactive_returns: Vec<(CompactString, ReactiveKind)>,
    /// File where defined.
    file_id: FileId,
    offset: u32,
}

/// A provide() call definition.
#[derive(Debug, Clone)]
struct ProvideDefinition {
    file_id: FileId,
    key: CompactString,
    value_name: CompactString,
    is_reactive: bool,
    reactive_kind: Option<ReactiveKind>,
    offset: u32,
}

impl<'a> CrossFileReactivityAnalyzer<'a> {
    /// Create a new analyzer.
    pub fn new(registry: &'a ModuleRegistry, graph: &'a DependencyGraph) -> Self {
        Self {
            registry,
            graph,
            reactive_values: FxHashMap::default(),
            flows: Vec::new(),
            issues: Vec::new(),
            composables: FxHashMap::default(),
            provides: FxHashMap::default(),
        }
    }

    /// Run the full analysis.
    pub fn analyze(mut self) -> (Vec<CrossFileReactivityIssue>, Vec<CrossFileDiagnostic>) {
        // Phase 1: Collect all reactive value definitions
        self.collect_reactive_definitions();

        // Phase 2: Collect composable definitions
        self.collect_composables();

        // Phase 3: Collect provide definitions
        self.collect_provides();

        // Phase 4: Track flows across file boundaries
        self.track_cross_file_flows();

        // Phase 5: Detect issues
        self.detect_issues();

        // Generate diagnostics
        let diagnostics = self.generate_diagnostics();

        (self.issues, diagnostics)
    }

    /// Phase 1: Collect all reactive value definitions from each file.
    fn collect_reactive_definitions(&mut self) {
        for entry in self.registry.vue_components() {
            let file_id = entry.id;
            let analysis = &entry.analysis;

            // Collect from reactivity sources
            for source in analysis.reactivity.sources() {
                let id = ReactiveValueId {
                    file_id,
                    name: source.name.clone(),
                    offset: source.declaration_offset,
                };

                self.reactive_values.insert(
                    id.clone(),
                    CrossFileReactiveValue {
                        id,
                        kind: source.kind,
                        exposures: SmallVec::new(),
                        consumptions: SmallVec::new(),
                        reactivity_preserved: true,
                    },
                );
            }
        }

        // Also collect from TypeScript/JavaScript modules
        for entry in self.registry.iter().filter(|e| !e.is_vue_sfc) {
            let file_id = entry.id;
            let analysis = &entry.analysis;

            for source in analysis.reactivity.sources() {
                let id = ReactiveValueId {
                    file_id,
                    name: source.name.clone(),
                    offset: source.declaration_offset,
                };

                self.reactive_values.insert(
                    id.clone(),
                    CrossFileReactiveValue {
                        id,
                        kind: source.kind,
                        exposures: SmallVec::new(),
                        consumptions: SmallVec::new(),
                        reactivity_preserved: true,
                    },
                );
            }
        }
    }

    /// Phase 2: Collect composable function definitions.
    fn collect_composables(&mut self) {
        // Composables are typically in .ts files with "use" prefix
        for entry in self.registry.iter().filter(|e| !e.is_vue_sfc) {
            let file_id = entry.id;
            let path = entry.path.to_string_lossy();
            let path_str = path.as_ref();

            // Check if this looks like a composable file
            let filename = path_str.rsplit('/').next().unwrap_or(path_str);
            if !filename.starts_with("use") && !path_str.contains("/composables/") {
                continue;
            }

            let analysis = &entry.analysis;
            let mut composable_infos = Vec::new();

            // Look for exported functions that start with "use"
            for scope in analysis.scopes.iter() {
                if let crate::scope::ScopeKind::Function = scope.kind {
                    for (name, _) in scope.bindings() {
                        if name.starts_with("use") {
                            // This is likely a composable
                            // Collect its reactive returns
                            let reactive_returns: Vec<(CompactString, ReactiveKind)> = analysis
                                .reactivity
                                .sources()
                                .iter()
                                .map(|s| (s.name.clone(), s.kind))
                                .collect();

                            composable_infos.push(ComposableInfo {
                                name: CompactString::new(name),
                                reactive_returns,
                                file_id,
                                offset: scope.span.start,
                            });
                        }
                    }
                }
            }

            if !composable_infos.is_empty() {
                self.composables.insert(file_id, composable_infos);
            }
        }
    }

    /// Phase 3: Collect provide() definitions.
    fn collect_provides(&mut self) {
        for entry in self.registry.vue_components() {
            let file_id = entry.id;
            let analysis = &entry.analysis;

            for provide in analysis.provide_inject.provides() {
                let key_str = match &provide.key {
                    crate::provide::ProvideKey::String(s) => s.clone(),
                    crate::provide::ProvideKey::Symbol(s) => {
                        CompactString::new(format!("Symbol:{}", s))
                    }
                };

                // Check if the provided value is reactive
                let is_reactive = analysis
                    .reactivity
                    .sources()
                    .iter()
                    .any(|s| s.name == provide.value);

                let reactive_kind = analysis
                    .reactivity
                    .sources()
                    .iter()
                    .find(|s| s.name == provide.value)
                    .map(|s| s.kind);

                self.provides.insert(
                    key_str.clone(),
                    ProvideDefinition {
                        file_id,
                        key: key_str,
                        value_name: provide.value.clone(),
                        is_reactive,
                        reactive_kind,
                        offset: provide.start,
                    },
                );
            }
        }
    }

    /// Phase 4: Track reactivity flows across file boundaries.
    fn track_cross_file_flows(&mut self) {
        // Track composable import flows
        self.track_composable_flows();

        // Track provide/inject flows
        self.track_provide_inject_flows();

        // Track props flows
        self.track_props_flows();
    }

    /// Track flows from composable exports to imports.
    fn track_composable_flows(&mut self) {
        for entry in self.registry.vue_components() {
            let consumer_file_id = entry.id;
            let analysis = &entry.analysis;

            // Check for composable calls
            for composable in analysis.provide_inject.composables() {
                // Find the source file for this composable
                let source_file = self.find_composable_source(&composable.source);

                // Record the consumption
                if let Some(source_id) = source_file {
                    // Check if the composable return is destructured
                    // This is a key reactivity loss pattern
                    self.check_composable_usage(
                        consumer_file_id,
                        &composable.name,
                        composable.local_name.as_ref(),
                        source_id,
                        composable.start,
                    );
                }
            }
        }
    }

    /// Find the source file for a composable import path.
    fn find_composable_source(&self, source_path: &str) -> Option<FileId> {
        // Try to resolve the import path to a file
        for node in self.graph.nodes() {
            if let Some(entry) = self.registry.get(node.file_id) {
                let path = entry.path.to_string_lossy();
                if path.ends_with(&format!("{}.ts", source_path))
                    || path.ends_with(&format!("{}/index.ts", source_path))
                    || path.contains(source_path)
                {
                    return Some(node.file_id);
                }
            }
        }
        None
    }

    /// Check how a composable is used and detect issues.
    fn check_composable_usage(
        &mut self,
        consumer_file_id: FileId,
        composable_name: &CompactString,
        local_name: Option<&CompactString>,
        _source_file_id: FileId,
        offset: u32,
    ) {
        // If the composable result is not assigned to a variable (destructured directly),
        // we need to check the pattern
        if local_name.is_none() {
            // The composable return was destructured
            // This is often a reactivity loss if the composable returns reactive values
            self.issues.push(CrossFileReactivityIssue {
                file_id: consumer_file_id,
                kind: CrossFileReactivityIssueKind::ComposableReturnDestructured {
                    composable_name: composable_name.clone(),
                    destructured_props: vec![CompactString::new("(unknown)")],
                },
                offset,
                related_file: None,
                severity: DiagnosticSeverity::Warning,
            });
        }
    }

    /// Track provide/inject flows.
    fn track_provide_inject_flows(&mut self) {
        for entry in self.registry.vue_components() {
            let consumer_file_id = entry.id;
            let analysis = &entry.analysis;

            for inject in analysis.provide_inject.injects() {
                let key_str = match &inject.key {
                    crate::provide::ProvideKey::String(s) => s.clone(),
                    crate::provide::ProvideKey::Symbol(s) => {
                        CompactString::new(format!("Symbol:{}", s))
                    }
                };

                // Find the provider
                if let Some(provider) = self.provides.get(&key_str) {
                    // Check if inject result is destructured
                    use crate::provide::InjectPattern;
                    match &inject.pattern {
                        InjectPattern::ObjectDestructure(props) => {
                            self.issues.push(CrossFileReactivityIssue {
                                file_id: consumer_file_id,
                                kind: CrossFileReactivityIssueKind::InjectValueDestructured {
                                    key: key_str.clone(),
                                    destructured_props: props.clone(),
                                },
                                offset: inject.start,
                                related_file: Some(provider.file_id),
                                severity: DiagnosticSeverity::Error,
                            });
                        }
                        InjectPattern::ArrayDestructure(_) => {
                            self.issues.push(CrossFileReactivityIssue {
                                file_id: consumer_file_id,
                                kind: CrossFileReactivityIssueKind::InjectValueDestructured {
                                    key: key_str.clone(),
                                    destructured_props: vec![CompactString::new(
                                        "(array destructure)",
                                    )],
                                },
                                offset: inject.start,
                                related_file: Some(provider.file_id),
                                severity: DiagnosticSeverity::Error,
                            });
                        }
                        InjectPattern::Simple => {
                            // OK - inject is assigned to a variable
                        }
                    }

                    // Check if provider provides non-reactive value
                    if !provider.is_reactive {
                        self.issues.push(CrossFileReactivityIssue {
                            file_id: provider.file_id,
                            kind: CrossFileReactivityIssueKind::NonReactiveProvide {
                                key: key_str.clone(),
                            },
                            offset: provider.offset,
                            related_file: Some(consumer_file_id),
                            severity: DiagnosticSeverity::Info,
                        });
                    }

                    // Create a flow record
                    let source_id = ReactiveValueId {
                        file_id: provider.file_id,
                        name: provider.value_name.clone(),
                        offset: provider.offset,
                    };
                    let target_id = ReactiveValueId {
                        file_id: consumer_file_id,
                        name: inject.local_name.clone(),
                        offset: inject.start,
                    };

                    let (preserved, loss_reason) = match &inject.pattern {
                        InjectPattern::Simple => (true, None),
                        InjectPattern::ObjectDestructure(_props) => {
                            (false, Some(ReactivityLossReason::InjectDestructure))
                        }
                        InjectPattern::ArrayDestructure(_) => (
                            false,
                            Some(ReactivityLossReason::Destructured { props: vec![] }),
                        ),
                    };

                    self.flows.push(ReactivityFlow {
                        source: source_id,
                        target: target_id,
                        flow_kind: ReactivityFlowKind::ProvideInject,
                        preserved,
                        loss_reason,
                    });
                }
            }
        }
    }

    /// Track props flows between parent and child components.
    fn track_props_flows(&mut self) {
        for node in self.graph.nodes() {
            let parent_file_id = node.file_id;

            // Check component usages from this file
            for (child_file_id, edge_type) in &node.imports {
                if *edge_type != DependencyEdge::ComponentUsage {
                    continue;
                }

                // Get the parent's component usages
                if let Some(parent_entry) = self.registry.get(parent_file_id) {
                    for usage in &parent_entry.analysis.component_usages {
                        // Check each prop passed
                        for prop in &usage.props {
                            // Skip if no value
                            let Some(value) = &prop.value else {
                                continue;
                            };

                            // Check if this prop is reactive in the parent
                            let is_reactive = parent_entry
                                .analysis
                                .reactivity
                                .sources()
                                .iter()
                                .any(|s| s.name == *value);

                            if is_reactive {
                                // Create a props flow
                                let source_id = ReactiveValueId {
                                    file_id: parent_file_id,
                                    name: value.clone(),
                                    offset: prop.start,
                                };
                                let target_id = ReactiveValueId {
                                    file_id: *child_file_id,
                                    name: prop.name.clone(),
                                    offset: 0, // We don't know child's offset here
                                };

                                self.flows.push(ReactivityFlow {
                                    source: source_id,
                                    target: target_id,
                                    flow_kind: ReactivityFlowKind::PropsFlow,
                                    preserved: true, // Props flow preserves reactivity
                                    loss_reason: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// Phase 5: Detect additional cross-file issues.
    fn detect_issues(&mut self) {
        // Check for Pinia store destructuring
        for entry in self.registry.vue_components() {
            let file_id = entry.id;
            let analysis = &entry.analysis;

            // Look for Pinia store usage patterns
            self.detect_pinia_issues(file_id, analysis);

            // Detect props destructuring
            self.detect_props_destructure_issues(file_id, analysis);
        }

        // Check for circular reactive dependencies
        self.detect_circular_dependencies();
    }

    /// Detect Pinia store usage issues.
    fn detect_pinia_issues(&mut self, file_id: FileId, analysis: &crate::Croquis) {
        // Look for imports from pinia
        for scope in analysis.scopes.iter() {
            if let crate::scope::ScopeKind::ExternalModule = scope.kind {
                if let crate::scope::ScopeData::ExternalModule(data) = scope.data() {
                    if data.source.as_str() == "pinia" {
                        // Check for storeToRefs usage
                        let has_store_to_refs =
                            scope.bindings().any(|(name, _)| name == "storeToRefs");

                        if !has_store_to_refs {
                            // Check if there are store calls that might be destructured
                            // This is a heuristic - stores are usually named `use*Store`
                            for composable in analysis.provide_inject.composables() {
                                if composable.name.ends_with("Store")
                                    && composable.local_name.is_none()
                                {
                                    self.issues.push(CrossFileReactivityIssue {
                                        file_id,
                                        kind: CrossFileReactivityIssueKind::StoreDestructured {
                                            store_name: composable.name.clone(),
                                            destructured_props: vec![],
                                        },
                                        offset: composable.start,
                                        related_file: None,
                                        severity: DiagnosticSeverity::Warning,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Detect props destructuring issues.
    fn detect_props_destructure_issues(&mut self, file_id: FileId, analysis: &crate::Croquis) {
        if let Some(destructure) = analysis.macros.props_destructure() {
            // Check if toRefs is used
            let has_to_refs = analysis
                .reactivity
                .sources()
                .iter()
                .any(|s| matches!(s.kind, ReactiveKind::ToRefs));

            if !has_to_refs && !destructure.bindings.is_empty() {
                let props: Vec<CompactString> = destructure.bindings.keys().cloned().collect();

                // Note: Modern Vue handles this via reactive props destructure transform
                // So we only emit an info-level diagnostic
                self.issues.push(CrossFileReactivityIssue {
                    file_id,
                    kind: CrossFileReactivityIssueKind::PropsDestructured {
                        destructured_props: props,
                    },
                    offset: 0, // Destructure location from macro analysis
                    related_file: None,
                    severity: DiagnosticSeverity::Info,
                });
            }
        }
    }

    /// Detect circular reactive dependencies.
    fn detect_circular_dependencies(&mut self) {
        // Build a graph of reactive value dependencies
        let mut visited: FxHashSet<ReactiveValueId> = FxHashSet::default();
        let mut rec_stack: FxHashSet<ReactiveValueId> = FxHashSet::default();
        let mut path: Vec<CompactString> = Vec::new();

        for flow in &self.flows {
            if self.dfs_cycle_detect(&flow.source, &mut visited, &mut rec_stack, &mut path) {
                // Found a cycle
                let file_id = flow.source.file_id;
                self.issues.push(CrossFileReactivityIssue {
                    file_id,
                    kind: CrossFileReactivityIssueKind::CircularReactiveDependency {
                        cycle: path.clone(),
                    },
                    offset: flow.source.offset,
                    related_file: Some(flow.target.file_id),
                    severity: DiagnosticSeverity::Warning,
                });
                break;
            }
        }
    }

    /// DFS for cycle detection.
    fn dfs_cycle_detect(
        &self,
        current: &ReactiveValueId,
        visited: &mut FxHashSet<ReactiveValueId>,
        rec_stack: &mut FxHashSet<ReactiveValueId>,
        path: &mut Vec<CompactString>,
    ) -> bool {
        if rec_stack.contains(current) {
            return true;
        }
        if visited.contains(current) {
            return false;
        }

        visited.insert(current.clone());
        rec_stack.insert(current.clone());
        path.push(current.name.clone());

        // Find outgoing edges
        for flow in &self.flows {
            if flow.source == *current
                && self.dfs_cycle_detect(&flow.target, visited, rec_stack, path)
            {
                return true;
            }
        }

        path.pop();
        rec_stack.remove(current);
        false
    }

    /// Generate diagnostics from detected issues.
    fn generate_diagnostics(&self) -> Vec<CrossFileDiagnostic> {
        let mut diagnostics = Vec::new();

        for issue in &self.issues {
            let diag = match &issue.kind {
                CrossFileReactivityIssueKind::ComposableReturnDestructured {
                    composable_name,
                    destructured_props,
                } => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                        source_name: composable_name.clone(),
                        destructured_keys: destructured_props.clone(),
                        suggestion: CompactString::new("toRefs"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!(
                        "Destructuring {} return loses reactivity for: {}",
                        composable_name,
                        destructured_props.join(", ")
                    ),
                )
                .with_suggestion(format!(
                    "const result = {}(); then access result.prop or use toRefs(result)",
                    composable_name
                )),

                CrossFileReactivityIssueKind::InjectValueDestructured {
                    key,
                    destructured_props,
                } => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                        source_name: key.clone(),
                        destructured_keys: destructured_props.clone(),
                        suggestion: CompactString::new("toRefs"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!(
                        "Destructuring inject('{}') loses reactivity for: {}",
                        key,
                        destructured_props.join(", ")
                    ),
                )
                .with_suggestion(format!(
                    "const injected = inject('{}'); access injected.prop directly",
                    key
                )),

                CrossFileReactivityIssueKind::StoreDestructured {
                    store_name,
                    destructured_props,
                } => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                        source_name: store_name.clone(),
                        destructured_keys: destructured_props.clone(),
                        suggestion: CompactString::new("storeToRefs"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!(
                        "Destructuring {} loses reactivity for state/getters",
                        store_name
                    ),
                )
                .with_suggestion(format!(
                    "const {{ ... }} = storeToRefs({})",
                    store_name
                )),

                CrossFileReactivityIssueKind::PropsDestructured { destructured_props } => {
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::DestructuringBreaksReactivity {
                            source_name: CompactString::new("props"),
                            destructured_keys: destructured_props.clone(),
                            suggestion: CompactString::new("toRefs"),
                        },
                        issue.severity,
                        issue.file_id,
                        issue.offset,
                        format!(
                            "Destructuring props: {} (Vue compiler handles this, but explicit toRefs is clearer)",
                            destructured_props.join(", ")
                        ),
                    )
                    .with_suggestion("const { ... } = toRefs(props) for explicit reactivity")
                }

                CrossFileReactivityIssueKind::NonReactiveProvide { key } => {
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::ReactivityOutsideSetup {
                            api_name: CompactString::new("provide"),
                            context_description: CompactString::new("non-reactive value"),
                        },
                        issue.severity,
                        issue.file_id,
                        issue.offset,
                        format!(
                            "provide('{}') value is not reactive - consumers won't see updates",
                            key
                        ),
                    )
                    .with_suggestion("provide('key', ref(value)) or provide('key', reactive({...}))")
                }

                CrossFileReactivityIssueKind::CircularReactiveDependency { cycle } => {
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::CircularReactiveDependency {
                            cycle: cycle.clone(),
                        },
                        issue.severity,
                        issue.file_id,
                        issue.offset,
                        format!("Circular reactive dependency: {} → ...", cycle.join(" → ")),
                    )
                    .with_suggestion("Break the cycle by using computed() or reorganizing data flow")
                }

                _ => CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::HydrationMismatchRisk {
                        reason: CompactString::new("cross-file reactivity issue"),
                    },
                    issue.severity,
                    issue.file_id,
                    issue.offset,
                    format!("{:?}", issue.kind),
                ),
            };

            diagnostics.push(diag);
        }

        diagnostics
    }

    /// Generate a markdown report of cross-file reactivity flows.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Cross-File Reactivity Report\n\n");

        // Summary
        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "- **Tracked Reactive Values**: {}\n",
            self.reactive_values.len()
        ));
        md.push_str(&format!("- **Cross-File Flows**: {}\n", self.flows.len()));
        md.push_str(&format!("- **Issues Detected**: {}\n\n", self.issues.len()));

        // Flows
        if !self.flows.is_empty() {
            md.push_str("## Reactivity Flows\n\n");
            md.push_str("```\n");

            for flow in &self.flows {
                let status = if flow.preserved { "✓" } else { "✗" };
                let flow_type = match flow.flow_kind {
                    ReactivityFlowKind::ComposableExport => "composable",
                    ReactivityFlowKind::ProvideInject => "provide/inject",
                    ReactivityFlowKind::PropsFlow => "props",
                    ReactivityFlowKind::StoreFlow => "store",
                    ReactivityFlowKind::ModuleImport => "import",
                };

                md.push_str(&format!(
                    "{} [{}] {} → {}\n",
                    status, flow_type, flow.source.name, flow.target.name
                ));

                if let Some(ref reason) = flow.loss_reason {
                    md.push_str(&format!("   └─ Loss: {:?}\n", reason));
                }
            }

            md.push_str("```\n\n");
        }

        // Issues
        if !self.issues.is_empty() {
            md.push_str("## Issues\n\n");

            for issue in &self.issues {
                let icon = match issue.severity {
                    DiagnosticSeverity::Error => "❌",
                    DiagnosticSeverity::Warning => "⚠️",
                    DiagnosticSeverity::Info => "ℹ️",
                    DiagnosticSeverity::Hint => "💡",
                };

                md.push_str(&format!("### {} {:?}\n\n", icon, issue.kind));
                md.push_str(&format!("- **File**: {:?}\n", issue.file_id));
                md.push_str(&format!("- **Offset**: {}\n", issue.offset));
                if let Some(related) = issue.related_file {
                    md.push_str(&format!("- **Related File**: {:?}\n", related));
                }
                md.push('\n');
            }
        }

        md
    }
}

/// Public API: Analyze cross-file reactivity.
pub fn analyze_cross_file_reactivity(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<CrossFileReactivityIssue>, Vec<CrossFileDiagnostic>) {
    let analyzer = CrossFileReactivityAnalyzer::new(registry, graph);
    analyzer.analyze()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reactive_value_id() {
        let id = ReactiveValueId {
            file_id: FileId::new(1),
            name: CompactString::new("count"),
            offset: 42,
        };

        assert_eq!(id.name.as_str(), "count");
        assert_eq!(id.offset, 42);
    }

    #[test]
    fn test_reactivity_flow_kind() {
        let flow = ReactivityFlow {
            source: ReactiveValueId {
                file_id: FileId::new(1),
                name: CompactString::new("theme"),
                offset: 0,
            },
            target: ReactiveValueId {
                file_id: FileId::new(2),
                name: CompactString::new("theme"),
                offset: 0,
            },
            flow_kind: ReactivityFlowKind::ProvideInject,
            preserved: true,
            loss_reason: None,
        };

        assert_eq!(flow.flow_kind, ReactivityFlowKind::ProvideInject);
        assert!(flow.preserved);
    }

    #[test]
    fn test_reactivity_loss_reason() {
        let reason = ReactivityLossReason::Destructured {
            props: vec![CompactString::new("count"), CompactString::new("name")],
        };

        match reason {
            ReactivityLossReason::Destructured { props } => {
                assert_eq!(props.len(), 2);
            }
            _ => panic!("Wrong reason type"),
        }
    }
}
