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
}
