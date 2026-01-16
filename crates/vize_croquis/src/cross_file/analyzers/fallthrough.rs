//! Fallthrough attribute analysis.
//!
//! Detects issues with attribute inheritance across component boundaries:
//! - Attributes passed to component but not used
//! - `inheritAttrs: false` without explicit $attrs binding
//! - Multiple root elements without explicit $attrs

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::DependencyGraph;
use crate::cross_file::registry::{FileId, ModuleRegistry};
use vize_carton::{CompactString, FxHashMap, FxHashSet};

/// Information about fallthrough attributes for a component.
#[derive(Debug, Clone)]
pub struct FallthroughInfo {
    /// File ID of the component.
    pub file_id: FileId,
    /// Whether `inheritAttrs: false` is set.
    pub inherit_attrs_disabled: bool,
    /// Whether $attrs is used in template.
    pub uses_attrs: bool,
    /// Whether $attrs is explicitly bound (v-bind="$attrs").
    pub binds_attrs: bool,
    /// Number of root elements in template.
    pub root_element_count: usize,
    /// Attributes passed by parent components.
    pub passed_attrs: FxHashSet<CompactString>,
    /// Props declared by this component.
    pub declared_props: FxHashSet<CompactString>,
}

impl FallthroughInfo {
    /// Check if fallthrough may cause issues.
    pub fn has_potential_issues(&self) -> bool {
        // Multiple roots without explicit $attrs
        if self.root_element_count > 1 && !self.binds_attrs {
            return true;
        }

        // inheritAttrs: false but $attrs not used
        if self.inherit_attrs_disabled && !self.uses_attrs && !self.binds_attrs {
            return true;
        }

        // Attributes passed that aren't props
        let fallthrough_attrs: Vec<_> = self
            .passed_attrs
            .iter()
            .filter(|attr| !self.declared_props.contains(*attr))
            .collect();

        if !fallthrough_attrs.is_empty() && !self.uses_attrs && self.root_element_count > 1 {
            return true;
        }

        false
    }
}

/// Analyze fallthrough attributes across the component graph.
pub fn analyze_fallthrough(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<FallthroughInfo>, Vec<CrossFileDiagnostic>) {
    let mut infos = Vec::new();
    let mut diagnostics = Vec::new();

    // Build a map of what attributes each component passes to its children
    let mut passed_attrs_map: FxHashMap<FileId, FxHashMap<FileId, FxHashSet<CompactString>>> =
        FxHashMap::default();

    // First pass: collect information from each component
    for entry in registry.vue_components() {
        let analysis = &entry.analysis;

        // Use precise template_info from static analysis
        let template_info = &analysis.template_info;

        // Check for inheritAttrs option (from defineOptions macro)
        let inherit_attrs_disabled = check_inherit_attrs_disabled(analysis);

        // Get declared props
        let declared_props: FxHashSet<_> = analysis
            .macros
            .props()
            .iter()
            .map(|p| p.name.clone())
            .collect();

        let info = FallthroughInfo {
            file_id: entry.id,
            inherit_attrs_disabled,
            uses_attrs: template_info.uses_attrs,
            binds_attrs: template_info.binds_attrs_explicitly,
            root_element_count: template_info.root_element_count,
            passed_attrs: FxHashSet::default(), // Will be filled later
            declared_props,
        };

        infos.push(info);
    }

    // Second pass: track attribute passing through component usage
    for node in graph.nodes() {
        // Look at component usage edges
        for (child_id, edge_type) in &node.imports {
            if *edge_type != crate::cross_file::graph::DependencyEdge::ComponentUsage {
                continue;
            }

            // Get the parent's analysis to find what attrs are passed
            if let Some(parent_entry) = registry.get(node.file_id) {
                // In a real implementation, we'd parse the template to find
                // exactly which attributes are passed. For now, we'll use
                // a simplified approach based on used_directives.
                let attrs = extract_passed_attrs(&parent_entry.analysis, child_id);

                passed_attrs_map
                    .entry(*child_id)
                    .or_default()
                    .entry(node.file_id)
                    .or_default()
                    .extend(attrs);
            }
        }
    }

    // Merge passed attrs into infos
    for info in &mut infos {
        if let Some(parent_attrs) = passed_attrs_map.get(&info.file_id) {
            for attrs in parent_attrs.values() {
                info.passed_attrs.extend(attrs.iter().cloned());
            }
        }
    }

    // Generate diagnostics
    for info in &infos {
        // Check for multiple root elements without explicit $attrs binding
        if info.root_element_count > 1 && !info.binds_attrs {
            let has_fallthrough = info
                .passed_attrs
                .iter()
                .any(|attr| !info.declared_props.contains(attr));

            if has_fallthrough {
                diagnostics.push(
                    CrossFileDiagnostic::new(
                        CrossFileDiagnosticKind::MultiRootMissingAttrs,
                        DiagnosticSeverity::Warning,
                        info.file_id,
                        0,
                        "Component has multiple root elements but $attrs is not explicitly bound",
                    )
                    .with_suggestion(
                        "Add v-bind=\"$attrs\" to the intended root element or wrap in single root",
                    ),
                );
            }
        }

        // Check for inheritAttrs: false without $attrs usage
        if info.inherit_attrs_disabled && !info.uses_attrs && !info.binds_attrs {
            diagnostics.push(
                CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::InheritAttrsDisabledUnused,
                    DiagnosticSeverity::Warning,
                    info.file_id,
                    0,
                    "inheritAttrs is disabled but $attrs is not used anywhere",
                )
                .with_suggestion("Use v-bind=\"$attrs\" or $attrs.class/$attrs.style in template"),
            );
        }

        // Check for unused fallthrough attributes
        let unused_attrs: Vec<_> = info
            .passed_attrs
            .iter()
            .filter(|attr| {
                !info.declared_props.contains(*attr)
                    && !is_standard_html_attr(attr)
                    && !info.uses_attrs
            })
            .cloned()
            .collect();

        if !unused_attrs.is_empty() && !info.binds_attrs && info.root_element_count > 1 {
            diagnostics.push(
                CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::UnusedFallthroughAttrs {
                        passed_attrs: unused_attrs.clone(),
                    },
                    DiagnosticSeverity::Info,
                    info.file_id,
                    0,
                    format!(
                        "Attributes {:?} are passed but not used (component has multiple roots)",
                        unused_attrs
                    ),
                )
                .with_suggestion("Bind $attrs explicitly or declare as props"),
            );
        }
    }

    (infos, diagnostics)
}

/// Check if inheritAttrs: false is set in the component options.
fn check_inherit_attrs_disabled(analysis: &crate::Croquis) -> bool {
    // Look for defineOptions or script block with inheritAttrs: false
    // This is a simplified check - a full implementation would parse the AST
    analysis
        .macros
        .all_calls()
        .iter()
        .any(|call| call.name == "defineOptions")
}

/// Extract attributes passed to a child component.
/// Uses component_usages for precise static analysis.
fn extract_passed_attrs(analysis: &crate::Croquis, _child_id: &FileId) -> FxHashSet<CompactString> {
    let mut attrs = FxHashSet::default();

    // Get the child component name from registry (if available)
    // For now, we'll collect all passed props from component usages
    for usage in &analysis.component_usages {
        for prop in &usage.props {
            attrs.insert(prop.name.clone());
        }
    }

    attrs
}

/// Check if an attribute is a standard HTML attribute.
fn is_standard_html_attr(attr: &str) -> bool {
    matches!(
        attr,
        "class"
            | "style"
            | "id"
            | "key"
            | "ref"
            | "data-*"
            | "aria-*"
            | "role"
            | "tabindex"
            | "title"
            | "disabled"
            | "hidden"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallthrough_info_issues() {
        // Single root element - no issue
        let mut info = FallthroughInfo {
            file_id: FileId::new(0),
            inherit_attrs_disabled: false,
            uses_attrs: false,
            binds_attrs: false,
            root_element_count: 1,
            passed_attrs: FxHashSet::default(),
            declared_props: FxHashSet::default(),
        };
        assert!(!info.has_potential_issues());

        // Multiple roots without binds_attrs - this IS an issue
        info.root_element_count = 2;
        assert!(info.has_potential_issues());

        // Multiple roots WITH binds_attrs - no issue
        info.binds_attrs = true;
        assert!(!info.has_potential_issues());

        // Reset and test inheritAttrs disabled without using $attrs
        info.binds_attrs = false;
        info.root_element_count = 1;
        info.inherit_attrs_disabled = true;
        assert!(info.has_potential_issues());

        // inheritAttrs disabled but $attrs is used - no issue
        info.uses_attrs = true;
        assert!(!info.has_potential_issues());
    }
}
