//! Component resolution analyzer.
//!
//! Detects unregistered components and unresolved imports.

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::DependencyGraph;
use crate::cross_file::registry::{FileId, ModuleRegistry};
use vize_carton::{CompactString, FxHashSet};

/// Information about a component resolution issue.
#[derive(Debug, Clone)]
pub struct ComponentResolutionIssue {
    /// The file where the issue was found.
    pub file_id: FileId,
    /// The component name or import specifier.
    pub name: CompactString,
    /// Kind of issue.
    pub kind: ComponentResolutionIssueKind,
    /// Source offset.
    pub offset: u32,
}

/// Kind of component resolution issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentResolutionIssueKind {
    /// Component used in template but not imported/registered.
    UnregisteredComponent,
    /// Import specifier could not be resolved.
    UnresolvedImport,
}

/// Analyze component resolution across all files.
///
/// This analyzer checks:
/// 1. All components used in templates are properly imported/registered
/// 2. All import specifiers can be resolved to actual files
pub fn analyze_component_resolution(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<ComponentResolutionIssue>, Vec<CrossFileDiagnostic>) {
    let mut issues = Vec::new();
    let mut diagnostics = Vec::new();

    // Build a set of all registered component names from the dependency graph
    let registered_components: FxHashSet<&str> = graph
        .nodes()
        .filter_map(|node| node.component_name.as_deref())
        .collect();

    // Check each file
    for entry in registry.iter() {
        let file_id = entry.id;
        let analysis = &entry.analysis;

        // Get all imported identifiers from this file
        let imported_identifiers: FxHashSet<&str> = analysis
            .scopes
            .iter()
            .flat_map(|scope| scope.bindings().map(|(name, _)| name))
            .collect();

        // Check used components
        for component_name in &analysis.used_components {
            // Skip built-in components
            if is_builtin_component(component_name.as_str()) {
                continue;
            }

            // Check if component is imported as a binding
            let is_imported = imported_identifiers.contains(component_name.as_str());

            // Check if component exists in the project (registered in graph)
            let exists_in_project = registered_components.contains(component_name.as_str());

            // Check if it's available as a global component name (via import)
            let is_available = is_imported
                || exists_in_project
                || analysis.bindings.contains(component_name.as_str());

            if !is_available {
                let issue = ComponentResolutionIssue {
                    file_id,
                    name: component_name.clone(),
                    kind: ComponentResolutionIssueKind::UnregisteredComponent,
                    offset: 0, // TODO: Get actual offset from template
                };
                issues.push(issue);

                let diagnostic = CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::UnregisteredComponent {
                        component_name: component_name.clone(),
                        template_offset: 0,
                    },
                    DiagnosticSeverity::Error,
                    file_id,
                    0,
                    format!(
                        "**Unregistered Component**: `<{}>` is used in template but not imported\n\n\
                        The component must be imported in `<script setup>` or registered globally.",
                        component_name
                    ),
                )
                .with_suggestion(format!(
                    "```typescript\nimport {} from './{}.vue'\n```",
                    component_name, component_name
                ));

                diagnostics.push(diagnostic);
            }
        }

        // Check for unresolved imports
        for scope in analysis.scopes.iter() {
            if scope.kind == crate::scope::ScopeKind::ExternalModule {
                if let crate::scope::ScopeData::ExternalModule(data) = scope.data() {
                    let source = &data.source;

                    // Skip node_modules imports (bare specifiers)
                    if !source.starts_with('.')
                        && !source.starts_with('/')
                        && !source.starts_with('@')
                    {
                        continue;
                    }

                    // Skip @-prefixed imports that are likely aliases
                    if source.starts_with('@') && !source.starts_with("@/") {
                        continue;
                    }

                    // Check if the import resolves to a known file
                    let resolved = resolve_import(source, registry, entry.path.parent());

                    if !resolved {
                        let issue = ComponentResolutionIssue {
                            file_id,
                            name: source.clone(),
                            kind: ComponentResolutionIssueKind::UnresolvedImport,
                            offset: scope.span.start,
                        };
                        issues.push(issue);

                        let diagnostic = CrossFileDiagnostic::new(
                            CrossFileDiagnosticKind::UnresolvedImport {
                                specifier: source.clone(),
                                import_offset: scope.span.start,
                            },
                            DiagnosticSeverity::Error,
                            file_id,
                            scope.span.start,
                            format!(
                                "**Unresolved Import**: Cannot find module `{}`\n\n\
                                - Check if the file exists at the specified path\n\
                                - Verify the import path is correct (relative paths start with `./` or `../`)\n\
                                - For alias imports like `@/`, ensure tsconfig paths are configured",
                                source
                            ),
                        );

                        diagnostics.push(diagnostic);
                    }
                }
            }
        }
    }

    (issues, diagnostics)
}

/// Check if a component name is a Vue built-in component.
#[inline]
fn is_builtin_component(name: &str) -> bool {
    matches!(
        name,
        "Transition"
            | "TransitionGroup"
            | "KeepAlive"
            | "Suspense"
            | "Teleport"
            | "component"
            | "slot"
            | "template"
            // Nuxt built-ins
            | "NuxtPage"
            | "NuxtLayout"
            | "NuxtLink"
            | "NuxtLoadingIndicator"
            | "NuxtErrorBoundary"
            | "NuxtWelcome"
            | "NuxtIsland"
            | "ClientOnly"
            | "DevOnly"
            | "ServerPlaceholder"
            // Vue Router
            | "RouterView"
            | "RouterLink"
            // Head management
            | "Head"
            | "Html"
            | "Body"
            | "Title"
            | "Meta"
            | "Style"
            | "Link"
            | "Base"
            | "NoScript"
            | "Script"
    )
}

/// Try to resolve an import specifier to a file in the registry.
fn resolve_import(
    specifier: &str,
    registry: &ModuleRegistry,
    from_dir: Option<&std::path::Path>,
) -> bool {
    // Handle @/ alias (common Vue project alias)
    if let Some(relative) = specifier.strip_prefix("@/") {
        // Check with common extensions
        for ext in &["", ".vue", ".ts", ".tsx", ".js", ".jsx"] {
            let path = format!("src/{}{}", relative, ext);
            if registry.get_by_path(&path).is_some() {
                return true;
            }
        }
        return false;
    }

    // Handle relative imports
    if specifier.starts_with('.') {
        if let Some(dir) = from_dir {
            // Try with common extensions
            for ext in &[
                "",
                ".vue",
                ".ts",
                ".tsx",
                ".js",
                ".jsx",
                "/index.ts",
                "/index.vue",
            ] {
                let resolved = dir.join(format!("{}{}", specifier, ext));
                if registry.get_by_path(&resolved).is_some() {
                    return true;
                }
            }
        }
        return false;
    }

    // For absolute or other paths, check directly
    registry.get_by_path(specifier).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_component() {
        assert!(is_builtin_component("Transition"));
        assert!(is_builtin_component("KeepAlive"));
        assert!(is_builtin_component("RouterView"));
        assert!(is_builtin_component("NuxtPage"));
        assert!(!is_builtin_component("MyComponent"));
        assert!(!is_builtin_component("UserCard"));
    }
}
