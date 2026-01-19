//! Server/Client boundary and Error/Suspense boundary analysis.
//!
//! Detects issues related to:
//! - Browser APIs used in SSR context
//! - Async components without Suspense
//! - Missing error boundaries
//! - Hydration mismatch risks

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::{DependencyEdge, DependencyGraph};
use crate::cross_file::registry::{FileId, ModuleRegistry};
use vize_carton::{CompactString, FxHashSet};

/// Kind of boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryKind {
    /// Server/Client boundary (SSR).
    ServerClient,
    /// Error boundary (onErrorCaptured).
    Error,
    /// Suspense boundary (async components).
    Suspense,
}

/// Information about a boundary.
#[derive(Debug, Clone)]
pub struct BoundaryInfo {
    /// Component that defines the boundary.
    pub file_id: FileId,
    /// Kind of boundary.
    pub kind: BoundaryKind,
    /// Offset in source.
    pub offset: u32,
    /// Components protected by this boundary.
    pub protects: Vec<FileId>,
}

/// Analyze boundaries across the component tree.
pub fn analyze_boundaries(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<BoundaryInfo>, Vec<CrossFileDiagnostic>) {
    let mut boundaries = Vec::new();
    let mut diagnostics = Vec::new();

    // Collect components with boundaries
    let mut error_boundaries: FxHashSet<FileId> = FxHashSet::default();
    let mut suspense_boundaries: FxHashSet<FileId> = FxHashSet::default();
    let mut client_only_apis: Vec<(FileId, CompactString, u32)> = Vec::new();
    let mut async_components: FxHashSet<FileId> = FxHashSet::default();
    let mut components_with_errors: Vec<(FileId, u32)> = Vec::new();

    for entry in registry.vue_components() {
        let analysis = &entry.analysis;

        // Check for error boundary (onErrorCaptured)
        if has_error_captured(analysis) {
            error_boundaries.insert(entry.id);
            boundaries.push(BoundaryInfo {
                file_id: entry.id,
                kind: BoundaryKind::Error,
                offset: 0,
                protects: Vec::new(),
            });
        }

        // Check for Suspense usage
        if uses_suspense(analysis) {
            suspense_boundaries.insert(entry.id);
            boundaries.push(BoundaryInfo {
                file_id: entry.id,
                kind: BoundaryKind::Suspense,
                offset: 0,
                protects: Vec::new(),
            });
        }

        // Check for async setup
        if analysis.macros.is_async() {
            async_components.insert(entry.id);
        }

        // Check for browser-only APIs used outside client-only hooks
        let browser_usages = find_browser_api_usage(analysis);
        for (api, offset, context) in browser_usages {
            if !is_in_client_only_context(analysis, offset) {
                client_only_apis.push((entry.id, api.clone(), offset));

                diagnostics.push(
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::BrowserApiInSsr {
                            api,
                            context: CompactString::new(context),
                        },
                        DiagnosticSeverity::Warning,
                        entry.id,
                        offset,
                        "Browser API used in potentially SSR context",
                    )
                    .with_suggestion("Wrap in onMounted() or use import.meta.client check"),
                );
            }
        }

        // Check for potential errors without boundaries
        let error_sources = find_error_sources(analysis);
        for offset in error_sources {
            components_with_errors.push((entry.id, offset));
        }
    }

    // Check async components for Suspense boundaries
    for async_id in &async_components {
        let has_suspense = has_ancestor_with_boundary(*async_id, &suspense_boundaries, graph);

        if !has_suspense {
            let component_name = registry
                .get(*async_id)
                .and_then(|e| e.component_name.clone())
                .unwrap_or_else(|| CompactString::new("Component"));

            diagnostics.push(
                CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::AsyncWithoutSuspense { component_name },
                    DiagnosticSeverity::Warning,
                    *async_id,
                    0,
                    "Async component without Suspense boundary",
                )
                .with_suggestion(
                    "Wrap in <Suspense> or use defineAsyncComponent with loading state",
                ),
            );
        }
    }

    // Check error sources for error boundaries
    for (file_id, offset) in &components_with_errors {
        let has_boundary = has_ancestor_with_boundary(*file_id, &error_boundaries, graph);

        if !has_boundary {
            diagnostics.push(
                CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::UncaughtErrorBoundary,
                    DiagnosticSeverity::Info,
                    *file_id,
                    *offset,
                    "Potential error without error boundary",
                )
                .with_suggestion("Add onErrorCaptured in a parent component"),
            );
        }
    }

    // Update boundary protections
    for boundary in &mut boundaries {
        boundary.protects = find_protected_components(boundary.file_id, graph);
    }

    (boundaries, diagnostics)
}

/// Check if a component has onErrorCaptured.
fn has_error_captured(analysis: &crate::Croquis) -> bool {
    // Check for onErrorCaptured in bindings or scope
    analysis.bindings.contains("onErrorCaptured")
        || analysis.scopes.is_defined("onErrorCaptured")
        || analysis
            .template_expressions
            .iter()
            .any(|e| e.content.contains("onErrorCaptured"))
}

/// Check if a component uses Suspense.
fn uses_suspense(analysis: &crate::Croquis) -> bool {
    analysis.used_components.contains("Suspense")
        || analysis
            .used_components
            .iter()
            .any(|c| c.as_str() == "Suspense")
}

/// Find browser-only API usage in a component.
fn find_browser_api_usage(analysis: &crate::Croquis) -> Vec<(CompactString, u32, &'static str)> {
    let mut usages = Vec::new();

    let browser_apis = [
        ("window", "Browser global"),
        ("document", "DOM API"),
        ("navigator", "Browser API"),
        ("localStorage", "Web Storage"),
        ("sessionStorage", "Web Storage"),
        ("location", "Browser location"),
        ("history", "Browser history"),
        ("fetch", "Fetch API"), // Note: fetch is available in Node 18+, but behavior differs
        ("XMLHttpRequest", "XHR"),
        ("WebSocket", "WebSocket"),
        ("IntersectionObserver", "Intersection Observer"),
        ("ResizeObserver", "Resize Observer"),
        ("MutationObserver", "Mutation Observer"),
        ("requestAnimationFrame", "Animation API"),
        ("cancelAnimationFrame", "Animation API"),
        ("getComputedStyle", "CSSOM"),
        ("matchMedia", "Media Query"),
        ("alert", "Browser dialog"),
        ("confirm", "Browser dialog"),
        ("prompt", "Browser dialog"),
    ];

    // Check template expressions
    for expr in &analysis.template_expressions {
        for (api, context) in &browser_apis {
            if expr.content.contains(api) {
                usages.push((CompactString::new(*api), expr.start, *context));
            }
        }
    }

    // Note: We intentionally don't check global scopes here because they define
    // browser APIs as globals (window, document, etc.) which would cause false positives.
    // Instead, we only check template expressions for actual usage of these APIs.

    usages
}

/// Check if an offset is inside a client-only context.
fn is_in_client_only_context(analysis: &crate::Croquis, offset: u32) -> bool {
    // Find the scope at this offset
    for scope in analysis.scopes.iter() {
        if scope.span.start <= offset && offset <= scope.span.end {
            // Check if this scope or any parent is client-only
            if scope.kind == crate::scope::ScopeKind::ClientOnly {
                return true;
            }

            // Check parents
            for &parent_id in &scope.parents {
                if let Some(parent) = analysis.scopes.get_scope(parent_id) {
                    if parent.kind == crate::scope::ScopeKind::ClientOnly {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Find potential error sources in a component.
fn find_error_sources(analysis: &crate::Croquis) -> Vec<u32> {
    let mut sources = Vec::new();

    // Look for common error patterns
    let error_patterns = [
        "throw",
        "Error(",
        "reject(",
        "JSON.parse",
        "fetch(",
        "axios",
        "await ",
    ];

    for expr in &analysis.template_expressions {
        for pattern in &error_patterns {
            if expr.content.contains(pattern) {
                sources.push(expr.start);
                break;
            }
        }
    }

    sources
}

/// Check if a component has an ancestor with a boundary.
fn has_ancestor_with_boundary(
    file_id: FileId,
    boundaries: &FxHashSet<FileId>,
    graph: &DependencyGraph,
) -> bool {
    let mut visited = FxHashSet::default();
    let mut queue = vec![file_id];

    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Check if current is a boundary
        if current != file_id && boundaries.contains(&current) {
            return true;
        }

        // Add parents to queue
        for (parent_id, edge_type) in graph.dependents(current) {
            if edge_type == DependencyEdge::ComponentUsage && !visited.contains(&parent_id) {
                queue.push(parent_id);
            }
        }
    }

    false
}

/// Find all components protected by a boundary.
fn find_protected_components(boundary_id: FileId, graph: &DependencyGraph) -> Vec<FileId> {
    let mut protected = Vec::new();
    let mut visited = FxHashSet::default();
    let mut queue = vec![boundary_id];

    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Add children (components used by this one)
        for (child_id, edge_type) in graph.dependencies(current) {
            if edge_type == DependencyEdge::ComponentUsage {
                protected.push(child_id);
                if !visited.contains(&child_id) {
                    queue.push(child_id);
                }
            }
        }
    }

    protected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boundary_kind() {
        let kind = BoundaryKind::Error;
        assert_eq!(kind, BoundaryKind::Error);
    }
}
