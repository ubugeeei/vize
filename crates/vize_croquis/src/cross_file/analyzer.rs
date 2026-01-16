//! Main cross-file analyzer.
//!
//! Orchestrates all cross-file analysis passes and manages the module registry
//! and dependency graph.

use crate::cross_file::analyzers;
use crate::cross_file::diagnostics::CrossFileDiagnostic;
use crate::cross_file::graph::{DependencyEdge, DependencyGraph, ModuleNode};
use crate::cross_file::registry::{FileId, ModuleRegistry};
use crate::{Analyzer, AnalyzerOptions, Croquis};
use std::path::Path;

/// Options for cross-file analysis (opt-in features).
#[derive(Debug, Clone, Default)]
pub struct CrossFileOptions {
    /// Analyze fallthrough attributes.
    pub fallthrough_attrs: bool,
    /// Analyze component emits.
    pub component_emits: bool,
    /// Analyze event bubbling.
    pub event_bubbling: bool,
    /// Analyze provide/inject.
    pub provide_inject: bool,
    /// Analyze unique element IDs.
    pub unique_ids: bool,
    /// Analyze server/client boundaries.
    pub server_client_boundary: bool,
    /// Analyze error and suspense boundaries.
    pub error_suspense_boundary: bool,
    /// Analyze reactivity loss.
    pub reactivity_tracking: bool,
    /// Detect circular dependencies.
    pub circular_dependencies: bool,
    /// Maximum depth for dependency chain warnings.
    pub max_import_depth: Option<usize>,

    // === Static validation (strict mode) ===
    /// Check for unregistered components in templates.
    pub component_resolution: bool,
    /// Validate props passed to child components.
    pub props_validation: bool,
}

impl CrossFileOptions {
    /// Create options with all features enabled.
    pub fn all() -> Self {
        Self {
            fallthrough_attrs: true,
            component_emits: true,
            event_bubbling: true,
            provide_inject: true,
            unique_ids: true,
            server_client_boundary: true,
            error_suspense_boundary: true,
            reactivity_tracking: true,
            circular_dependencies: true,
            max_import_depth: Some(10),
            component_resolution: true,
            props_validation: true,
        }
    }

    /// Create options for strict static validation (compile errors for invalid Vue).
    pub fn strict() -> Self {
        Self {
            component_resolution: true,
            props_validation: true,
            circular_dependencies: true,
            ..Default::default()
        }
    }

    /// Create minimal options (fastest).
    pub fn minimal() -> Self {
        Self::default()
    }

    /// Enable fallthrough attribute analysis.
    pub fn with_fallthrough_attrs(mut self, enabled: bool) -> Self {
        self.fallthrough_attrs = enabled;
        self
    }

    /// Enable component emit analysis.
    pub fn with_component_emits(mut self, enabled: bool) -> Self {
        self.component_emits = enabled;
        self
    }

    /// Enable event bubbling analysis.
    pub fn with_event_bubbling(mut self, enabled: bool) -> Self {
        self.event_bubbling = enabled;
        self
    }

    /// Enable provide/inject analysis.
    pub fn with_provide_inject(mut self, enabled: bool) -> Self {
        self.provide_inject = enabled;
        self
    }

    /// Enable unique ID analysis.
    pub fn with_unique_ids(mut self, enabled: bool) -> Self {
        self.unique_ids = enabled;
        self
    }

    /// Enable server/client boundary analysis.
    pub fn with_server_client_boundary(mut self, enabled: bool) -> Self {
        self.server_client_boundary = enabled;
        self
    }

    /// Enable error/suspense boundary analysis.
    pub fn with_error_suspense_boundary(mut self, enabled: bool) -> Self {
        self.error_suspense_boundary = enabled;
        self
    }

    /// Enable reactivity tracking.
    pub fn with_reactivity_tracking(mut self, enabled: bool) -> Self {
        self.reactivity_tracking = enabled;
        self
    }

    /// Enable circular dependency detection.
    pub fn with_circular_dependencies(mut self, enabled: bool) -> Self {
        self.circular_dependencies = enabled;
        self
    }

    /// Set maximum import depth for warnings.
    pub fn with_max_import_depth(mut self, depth: Option<usize>) -> Self {
        self.max_import_depth = depth;
        self
    }

    /// Enable component resolution checking.
    pub fn with_component_resolution(mut self, enabled: bool) -> Self {
        self.component_resolution = enabled;
        self
    }

    /// Enable props validation.
    pub fn with_props_validation(mut self, enabled: bool) -> Self {
        self.props_validation = enabled;
        self
    }

    /// Check if any analysis is enabled.
    pub fn any_enabled(&self) -> bool {
        self.fallthrough_attrs
            || self.component_emits
            || self.event_bubbling
            || self.provide_inject
            || self.unique_ids
            || self.server_client_boundary
            || self.error_suspense_boundary
            || self.reactivity_tracking
            || self.circular_dependencies
            || self.component_resolution
            || self.props_validation
    }
}

/// Result of cross-file analysis.
#[derive(Debug, Default)]
pub struct CrossFileResult {
    /// All diagnostics from cross-file analysis.
    pub diagnostics: Vec<CrossFileDiagnostic>,

    /// Fallthrough attribute information per component.
    pub fallthrough_info: Vec<analyzers::FallthroughInfo>,

    /// Emit flow information.
    pub emit_flows: Vec<analyzers::EmitFlow>,

    /// Event bubbling information.
    pub event_bubbles: Vec<analyzers::EventBubble>,

    /// Provide/inject matches.
    pub provide_inject_matches: Vec<analyzers::ProvideInjectMatch>,

    /// Unique ID issues.
    pub unique_id_issues: Vec<analyzers::UniqueIdIssue>,

    /// Boundary information.
    pub boundaries: Vec<analyzers::BoundaryInfo>,

    /// Reactivity issues.
    pub reactivity_issues: Vec<analyzers::ReactivityIssue>,

    /// Circular dependencies (as paths of file IDs).
    pub circular_deps: Vec<Vec<FileId>>,

    /// Component resolution issues.
    pub component_resolution_issues: Vec<analyzers::ComponentResolutionIssue>,

    /// Props validation issues.
    pub props_validation_issues: Vec<analyzers::PropsValidationIssue>,

    /// Statistics.
    pub stats: CrossFileStats,
}

/// Statistics from cross-file analysis.
#[derive(Debug, Default, Clone)]
pub struct CrossFileStats {
    /// Number of files analyzed.
    pub files_analyzed: usize,
    /// Number of Vue components.
    pub vue_components: usize,
    /// Number of edges in dependency graph.
    pub dependency_edges: usize,
    /// Number of diagnostics by severity.
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    /// Analysis time in milliseconds.
    pub analysis_time_ms: f64,
}

/// Cross-file analyzer for Vue projects.
pub struct CrossFileAnalyzer {
    /// Analysis options.
    options: CrossFileOptions,
    /// Module registry.
    registry: ModuleRegistry,
    /// Dependency graph.
    graph: DependencyGraph,
    /// Single-file analyzer options.
    single_file_options: AnalyzerOptions,
}

impl CrossFileAnalyzer {
    /// Create a new cross-file analyzer.
    pub fn new(options: CrossFileOptions) -> Self {
        Self {
            options,
            registry: ModuleRegistry::new(),
            graph: DependencyGraph::new(),
            single_file_options: AnalyzerOptions::full(),
        }
    }

    /// Create with a project root directory.
    pub fn with_project_root(options: CrossFileOptions, root: impl AsRef<Path>) -> Self {
        Self {
            options,
            registry: ModuleRegistry::with_project_root(root.as_ref()),
            graph: DependencyGraph::new(),
            single_file_options: AnalyzerOptions::full(),
        }
    }

    /// Set single-file analyzer options.
    pub fn set_single_file_options(&mut self, options: AnalyzerOptions) {
        self.single_file_options = options;
    }

    /// Add a file to be analyzed.
    pub fn add_file(&mut self, path: impl AsRef<Path>, source: &str) -> FileId {
        let path = path.as_ref();

        // Analyze the file with single-file analyzer
        let analysis = self.analyze_single_file(source, path);

        // Register in module registry (takes ownership of analysis)
        let (file_id, is_new) = self.registry.register(path, source, analysis);

        if is_new {
            // Add to dependency graph
            let mut node = ModuleNode::new(file_id, path.to_string_lossy().as_ref());

            // Extract component name
            if let Some(entry) = self.registry.get(file_id) {
                node.component_name = entry.component_name.clone();
            }

            // Mark entry points
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if filename == "App.vue"
                || filename == "main.ts"
                || filename == "main.js"
                || filename == "index.vue"
            {
                node.is_entry = true;
            }

            self.graph.add_node(node);
        }

        // Update dependencies based on imports (get from registry)
        if let Some(entry) = self.registry.get(file_id) {
            // Collect data we need before calling update_dependencies
            let imports_data: Vec<_> = entry
                .analysis
                .scopes
                .iter()
                .filter(|s| s.kind == crate::scope::ScopeKind::ExternalModule)
                .filter_map(|s| {
                    if let crate::scope::ScopeData::ExternalModule(data) = s.data() {
                        Some((data.source.clone(), data.is_type_only))
                    } else {
                        None
                    }
                })
                .collect();

            let used_components: Vec<_> = entry.analysis.used_components.iter().cloned().collect();

            // Now update dependencies
            for (source, is_type_only) in imports_data {
                if let Some(target_id) = self.resolve_import(&source) {
                    // TODO: Distinguish type-only imports when tracking is needed
                    let edge_type = if is_type_only {
                        DependencyEdge::TypeImport
                    } else {
                        DependencyEdge::Import
                    };
                    self.graph.add_edge(file_id, target_id, edge_type);
                }
            }

            for component in used_components {
                if let Some(target_id) = self.graph.find_by_component(component.as_str()) {
                    self.graph
                        .add_edge(file_id, target_id, DependencyEdge::ComponentUsage);
                }
            }
        }

        file_id
    }

    /// Add multiple files.
    pub fn add_files(&mut self, files: &[(&Path, &str)]) {
        for (path, source) in files {
            self.add_file(path, source);
        }
    }

    /// Run cross-file analysis.
    pub fn analyze(&mut self) -> CrossFileResult {
        let start_time = std::time::Instant::now();

        let mut result = CrossFileResult::default();

        // Detect circular dependencies first
        if self.options.circular_dependencies {
            self.graph.detect_circular_dependencies();
            result.circular_deps = self.graph.circular_dependencies().to_vec();
        }

        // Run enabled analyzers
        if self.options.fallthrough_attrs {
            let (info, diags) = analyzers::analyze_fallthrough(&self.registry, &self.graph);
            result.fallthrough_info = info;
            result.diagnostics.extend(diags);
        }

        if self.options.component_emits {
            let (flows, diags) = analyzers::analyze_emits(&self.registry, &self.graph);
            result.emit_flows = flows;
            result.diagnostics.extend(diags);
        }

        if self.options.event_bubbling {
            let (bubbles, diags) = analyzers::analyze_event_bubbling(&self.registry, &self.graph);
            result.event_bubbles = bubbles;
            result.diagnostics.extend(diags);
        }

        if self.options.provide_inject {
            let (matches, diags) = analyzers::analyze_provide_inject(&self.registry, &self.graph);
            result.provide_inject_matches = matches;
            result.diagnostics.extend(diags);
        }

        if self.options.unique_ids {
            let (issues, diags) = analyzers::analyze_element_ids(&self.registry);
            result.unique_id_issues = issues;
            result.diagnostics.extend(diags);
        }

        if self.options.server_client_boundary || self.options.error_suspense_boundary {
            let (boundaries, diags) = analyzers::analyze_boundaries(&self.registry, &self.graph);
            result.boundaries = boundaries;
            result.diagnostics.extend(diags);
        }

        if self.options.reactivity_tracking {
            let (issues, diags) = analyzers::analyze_reactivity(&self.registry, &self.graph);
            result.reactivity_issues = issues;
            result.diagnostics.extend(diags);
        }

        // Static validation analyzers
        if self.options.component_resolution {
            let (issues, diags) =
                analyzers::analyze_component_resolution(&self.registry, &self.graph);
            result.component_resolution_issues = issues;
            result.diagnostics.extend(diags);
        }

        if self.options.props_validation {
            let (issues, diags) = analyzers::analyze_props_validation(&self.registry, &self.graph);
            result.props_validation_issues = issues;
            result.diagnostics.extend(diags);
        }

        // Calculate statistics
        result.stats = CrossFileStats {
            files_analyzed: self.registry.len(),
            vue_components: self.registry.vue_components().count(),
            dependency_edges: self.count_edges(),
            error_count: result.diagnostics.iter().filter(|d| d.is_error()).count(),
            warning_count: result.diagnostics.iter().filter(|d| d.is_warning()).count(),
            info_count: result.diagnostics.len()
                - result.stats.error_count
                - result.stats.warning_count,
            analysis_time_ms: start_time.elapsed().as_secs_f64() * 1000.0,
        };

        result
    }

    /// Get the module registry.
    #[inline]
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    /// Get the dependency graph.
    #[inline]
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Get analysis for a specific file.
    pub fn get_analysis(&self, file_id: FileId) -> Option<&Croquis> {
        self.registry.get(file_id).map(|e| &e.analysis)
    }

    /// Get file path by ID.
    pub fn get_file_path(&self, file_id: FileId) -> Option<&Path> {
        self.registry.get(file_id).map(|e| e.path.as_path())
    }

    /// Clear all data and reset.
    pub fn clear(&mut self) {
        self.registry.clear();
        self.graph = DependencyGraph::new();
    }

    // === Private methods ===

    fn analyze_single_file(&self, source: &str, path: &Path) -> Croquis {
        let mut analyzer = Analyzer::with_options(self.single_file_options);

        // Detect if it's a Vue SFC
        let is_vue = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("vue"));

        if is_vue {
            // Parse SFC and analyze
            // For now, just analyze the script part
            // A full implementation would use vize_armature to parse the SFC
            analyzer.analyze_script_setup(source);
        } else {
            analyzer.analyze_script_plain(source);
        }

        analyzer.finish()
    }

    fn resolve_import(&self, specifier: &str) -> Option<FileId> {
        // Simple resolution - check if we have this file in the registry
        // A full implementation would use import_resolver

        // Handle relative imports
        if specifier.starts_with('.') {
            // Would need current file context to resolve
            return None;
        }

        // Check by filename
        for entry in self.registry.iter() {
            if entry.filename.as_str() == specifier
                || entry.filename.as_str() == format!("{}.vue", specifier)
            {
                return Some(entry.id);
            }
        }

        None
    }

    fn count_edges(&self) -> usize {
        self.graph.nodes().map(|n| n.imports.len()).sum()
    }
}

impl Default for CrossFileAnalyzer {
    fn default() -> Self {
        Self::new(CrossFileOptions::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_file_options() {
        let options = CrossFileOptions::default();
        assert!(!options.any_enabled());

        let options = CrossFileOptions::all();
        assert!(options.any_enabled());
        assert!(options.fallthrough_attrs);
        assert!(options.reactivity_tracking);
        assert!(options.component_resolution);
        assert!(options.props_validation);
    }

    #[test]
    fn test_strict_options() {
        let options = CrossFileOptions::strict();
        assert!(options.component_resolution);
        assert!(options.props_validation);
        assert!(options.circular_dependencies);
        // Other options should be disabled
        assert!(!options.fallthrough_attrs);
        assert!(!options.event_bubbling);
    }

    #[test]
    fn test_analyzer_basic() {
        let mut analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        let id = analyzer.add_file(
            Path::new("Test.vue"),
            "<script setup>\nconst count = ref(0)\n</script>",
        );

        assert_eq!(analyzer.registry().len(), 1);
        assert!(analyzer.get_analysis(id).is_some());
    }

    #[test]
    fn test_component_resolution_error() {
        let mut analyzer = CrossFileAnalyzer::new(CrossFileOptions::strict());

        // Add a file that uses an unregistered component
        analyzer.add_file(
            Path::new("Parent.vue"),
            r#"<script setup>
// No import of ChildComponent
</script>"#,
        );

        // When template analysis is added, this test will verify
        // that unregistered components produce errors
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut analyzer = CrossFileAnalyzer::new(CrossFileOptions::strict());

        // This test would require adding files with circular imports
        // For now, just verify the analysis runs without crashing
        let result = analyzer.analyze();
        assert!(result.circular_deps.is_empty());
    }

    // === Provide/Inject Tests ===
    // NOTE: CrossFileAnalyzer.analyze_single_file doesn't parse SFC tags,
    // so we use .ts extension to pass raw script content

    #[test]
    fn test_provide_inject_basic_match() {
        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_provide_inject(true));

        // Parent provides 'state' (using .ts extension to pass raw script)
        analyzer.add_file(
            Path::new("Parent.ts"),
            r#"import { provide, reactive } from 'vue'
const state = reactive({ count: 0 })
provide('state', state)"#,
        );

        // Child injects 'state'
        analyzer.add_file(
            Path::new("Child.ts"),
            r#"import { inject } from 'vue'
const state = inject('state')"#,
        );

        let result = analyzer.analyze();

        // Both files should be analyzed
        assert_eq!(result.stats.files_analyzed, 2);
    }

    #[test]
    fn test_provide_inject_with_type_assertion() {
        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_provide_inject(true));

        // Child injects 'state' with type assertion
        analyzer.add_file(
            Path::new("Child.ts"),
            r#"import { inject } from 'vue'
const state = inject('state') as { count: number; user: { name: string } }"#,
        );

        let _result = analyzer.analyze();

        // Should detect the inject even with type assertion
        let child_analysis = analyzer.get_analysis(analyzer.registry().iter().next().unwrap().id);
        assert!(child_analysis.is_some());

        let analysis = child_analysis.unwrap();
        assert_eq!(analysis.provide_inject.injects().len(), 1);
        assert_eq!(
            analysis.provide_inject.injects()[0].key,
            crate::provide::ProvideKey::String(vize_carton::CompactString::new("state"))
        );
    }

    #[test]
    fn test_provide_inject_with_satisfies() {
        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_provide_inject(true));

        // Child injects 'theme' with satisfies
        analyzer.add_file(
            Path::new("Child.ts"),
            r#"import { inject } from 'vue'
const theme = inject('theme') satisfies string | undefined"#,
        );

        let _result = analyzer.analyze();

        let child_analysis = analyzer.get_analysis(analyzer.registry().iter().next().unwrap().id);
        assert!(child_analysis.is_some());

        let analysis = child_analysis.unwrap();
        assert_eq!(analysis.provide_inject.injects().len(), 1);
    }

    #[test]
    fn test_provide_with_symbol_key() {
        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_provide_inject(true));

        // Using Symbol as provide key
        analyzer.add_file(
            Path::new("Parent.ts"),
            r#"import { provide } from 'vue'
const ThemeKey = Symbol('theme')
provide(ThemeKey, 'dark')"#,
        );

        let _result = analyzer.analyze();

        let parent_analysis = analyzer.get_analysis(analyzer.registry().iter().next().unwrap().id);
        assert!(parent_analysis.is_some());

        let analysis = parent_analysis.unwrap();
        assert_eq!(analysis.provide_inject.provides().len(), 1);
    }

    #[test]
    fn test_inject_with_default_value() {
        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_provide_inject(true));

        // Child injects with default value
        analyzer.add_file(
            Path::new("Child.ts"),
            r#"import { inject } from 'vue'
const theme = inject('theme', 'light')"#,
        );

        let _result = analyzer.analyze();

        let child_analysis = analyzer.get_analysis(analyzer.registry().iter().next().unwrap().id);
        assert!(child_analysis.is_some());

        let analysis = child_analysis.unwrap();
        let injects = analysis.provide_inject.injects();
        assert_eq!(injects.len(), 1);
        assert!(injects[0].default_value.is_some());
    }

    #[test]
    fn test_multiple_provides_and_injects() {
        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_provide_inject(true));

        // Component with multiple provides and injects
        analyzer.add_file(
            Path::new("Mixed.ts"),
            r#"import { provide, inject, ref } from 'vue'

// Inject from ancestor
const theme = inject('theme', 'light')
const user = inject('user')

// Provide for descendants
const count = ref(0)
provide('count', count)
provide('config', { debug: true })"#,
        );

        let _result = analyzer.analyze();

        let analysis = analyzer
            .get_analysis(analyzer.registry().iter().next().unwrap().id)
            .unwrap();

        assert_eq!(analysis.provide_inject.provides().len(), 2);
        assert_eq!(analysis.provide_inject.injects().len(), 2);
    }

    #[test]
    fn test_reactivity_wrappers_detected() {
        let mut analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        analyzer.add_file(
            Path::new("Test.ts"),
            r#"import { ref, computed, reactive, shallowRef, toRef, toRefs } from 'vue'

const count = ref(0)
const doubled = computed(() => count.value * 2)
const state = reactive({ name: 'test' })
const shallow = shallowRef({ deep: 'value' })
const props = defineProps<{ item: { name: string } }>()
const nameRef = toRef(props, 'item')"#,
        );

        let analysis = analyzer
            .get_analysis(analyzer.registry().iter().next().unwrap().id)
            .unwrap();

        // Check reactivity tracking
        assert!(analysis.reactivity.is_reactive("count"));
        assert!(analysis.reactivity.is_reactive("doubled"));
        assert!(analysis.reactivity.is_reactive("state"));
        assert!(analysis.reactivity.is_reactive("shallow"));
        assert!(analysis.reactivity.is_reactive("nameRef"));
    }

    #[test]
    fn test_define_props_with_type() {
        let mut analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        analyzer.add_file(
            Path::new("Test.ts"),
            r#"const props = defineProps<{
    msg: string
    count?: number
    user: { name: string; age: number }
}>()"#,
        );

        let analysis = analyzer
            .get_analysis(analyzer.registry().iter().next().unwrap().id)
            .unwrap();

        assert_eq!(analysis.macros.props().len(), 3);
        assert!(analysis
            .macros
            .props()
            .iter()
            .any(|p| p.name.as_str() == "msg" && p.required));
        assert!(analysis
            .macros
            .props()
            .iter()
            .any(|p| p.name.as_str() == "count" && !p.required));
        assert!(analysis
            .macros
            .props()
            .iter()
            .any(|p| p.name.as_str() == "user" && p.required));
    }

    #[test]
    fn test_define_emits() {
        let mut analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        analyzer.add_file(
            Path::new("Test.ts"),
            r#"const emit = defineEmits<{
    (e: 'update', value: string): void
    (e: 'delete', id: number): void
}>()"#,
        );

        let analysis = analyzer
            .get_analysis(analyzer.registry().iter().next().unwrap().id)
            .unwrap();

        assert_eq!(analysis.macros.emits().len(), 2);
    }

    #[test]
    fn test_invalid_exports_in_script_setup() {
        let _analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        // Use Analyzer directly for script setup context
        let mut single_analyzer = crate::Analyzer::with_options(AnalyzerOptions::full());
        single_analyzer.analyze_script_setup(
            r#"export const foo = 'bar'
export function hello() {}
export default {}"#,
        );
        let analysis = single_analyzer.finish();

        // Should detect invalid exports in script setup
        assert!(analysis.invalid_exports.len() >= 2);
    }

    #[test]
    fn test_type_exports_allowed() {
        let _analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        // Use Analyzer directly for script setup context
        let mut single_analyzer = crate::Analyzer::with_options(AnalyzerOptions::full());
        single_analyzer.analyze_script_setup(
            r#"export type Props = { msg: string }
export interface Emits {
    (e: 'update', value: string): void
}"#,
        );
        let analysis = single_analyzer.finish();

        // Type exports should be allowed and tracked
        assert_eq!(analysis.type_exports.len(), 2);
        // No invalid exports for type declarations
        assert_eq!(analysis.invalid_exports.len(), 0);
    }

    #[test]
    fn test_scope_tracking_lifecycle_hooks() {
        let _analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        // Use Analyzer directly for script setup context
        let mut single_analyzer = crate::Analyzer::with_options(AnalyzerOptions::full());
        single_analyzer.analyze_script_setup(
            r#"import { onMounted, onUnmounted, ref } from 'vue'

const count = ref(0)

onMounted(() => {
    console.log('mounted')
    count.value++
})

onUnmounted(() => {
    console.log('unmounted')
})"#,
        );
        let analysis = single_analyzer.finish();

        // Should have scopes for lifecycle hooks (client-only scopes)
        let client_only_scopes: Vec<_> = analysis
            .scopes
            .iter()
            .filter(|s| s.kind == crate::scope::ScopeKind::ClientOnly)
            .collect();

        assert_eq!(
            client_only_scopes.len(),
            2,
            "Should have 2 client-only scopes for onMounted and onUnmounted"
        );
    }

    #[test]
    fn test_nested_callback_scopes() {
        let _analyzer = CrossFileAnalyzer::new(CrossFileOptions::minimal());

        // Use Analyzer directly for script setup context
        let mut single_analyzer = crate::Analyzer::with_options(AnalyzerOptions::full());
        single_analyzer.analyze_script_setup(
            r#"import { computed } from 'vue'

const items = computed(() => {
    return list.map(item => {
        return item.value.filter(v => v > 0)
    })
})"#,
        );
        let analysis = single_analyzer.finish();

        // Should have multiple closure scopes for nested callbacks
        let closure_scopes: Vec<_> = analysis
            .scopes
            .iter()
            .filter(|s| s.kind == crate::scope::ScopeKind::Closure)
            .collect();

        assert!(
            closure_scopes.len() >= 3,
            "Should have at least 3 closure scopes (computed, map, filter)"
        );
    }

    #[test]
    fn test_inject_object_destructure_pattern() {
        use crate::provide::InjectPattern;

        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_reactivity_tracking(true));

        // Destructuring inject() loses reactivity
        analyzer.add_file(
            Path::new("Child.ts"),
            r#"import { inject } from 'vue'
const { count, name } = inject('state') as { count: number; name: string }"#,
        );

        let _result = analyzer.analyze();

        let analysis = analyzer
            .get_analysis(analyzer.registry().iter().next().unwrap().id)
            .unwrap();

        // Should detect the inject with ObjectDestructure pattern
        let injects = analysis.provide_inject.injects();
        assert_eq!(injects.len(), 1, "Should have 1 inject");
        match &injects[0].pattern {
            InjectPattern::ObjectDestructure(props) => {
                assert!(props.contains(&vize_carton::CompactString::new("count")));
                assert!(props.contains(&vize_carton::CompactString::new("name")));
            }
            _ => panic!(
                "Expected ObjectDestructure pattern, got {:?}",
                injects[0].pattern
            ),
        }
    }

    #[test]
    fn test_inject_simple_pattern() {
        use crate::provide::InjectPattern;

        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_provide_inject(true));

        // Simple inject without destructuring
        analyzer.add_file(
            Path::new("Child.ts"),
            r#"import { inject } from 'vue'
const state = inject('state')"#,
        );

        let _result = analyzer.analyze();

        let analysis = analyzer
            .get_analysis(analyzer.registry().iter().next().unwrap().id)
            .unwrap();

        let injects = analysis.provide_inject.injects();
        assert_eq!(injects.len(), 1);
        assert!(matches!(injects[0].pattern, InjectPattern::Simple));
    }

    #[test]
    fn test_inject_destructure_with_type_assertion() {
        use crate::provide::InjectPattern;

        let mut analyzer =
            CrossFileAnalyzer::new(CrossFileOptions::default().with_reactivity_tracking(true));

        // Destructuring with TSAsExpression
        analyzer.add_file(
            Path::new("Child.ts"),
            r#"import { inject } from 'vue'
const { foo } = inject('data') as { foo: string }"#,
        );

        let _result = analyzer.analyze();

        let analysis = analyzer
            .get_analysis(analyzer.registry().iter().next().unwrap().id)
            .unwrap();

        let injects = analysis.provide_inject.injects();
        assert_eq!(injects.len(), 1);
        match &injects[0].pattern {
            InjectPattern::ObjectDestructure(props) => {
                assert!(props.contains(&vize_carton::CompactString::new("foo")));
            }
            _ => panic!("Expected ObjectDestructure pattern"),
        }
    }
}
