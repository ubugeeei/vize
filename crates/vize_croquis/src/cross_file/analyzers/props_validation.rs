//! Props validation analyzer.
//!
//! Validates that props passed to child components match their declarations.

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::DependencyGraph;
use crate::cross_file::registry::{FileId, ModuleRegistry};
use vize_carton::{CompactString, FxHashMap, FxHashSet};

/// Information about a props validation issue.
#[derive(Debug, Clone)]
pub struct PropsValidationIssue {
    /// The file where the parent component is.
    pub parent_file: FileId,
    /// The file where the child component is.
    pub child_file: FileId,
    /// The component name.
    pub component_name: CompactString,
    /// Kind of issue.
    pub kind: PropsValidationIssueKind,
    /// Source offset in parent file.
    pub offset: u32,
}

/// Kind of props validation issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsValidationIssueKind {
    /// Prop passed but not declared in child.
    UndeclaredProp { prop_name: CompactString },
    /// Required prop not passed.
    MissingRequiredProp { prop_name: CompactString },
    /// Type mismatch (if detectable statically).
    TypeMismatch {
        prop_name: CompactString,
        expected: CompactString,
        actual: CompactString,
    },
}

/// Information about a child component's props.
#[derive(Debug, Default)]
struct ComponentPropsInfo {
    /// Declared props with their required status.
    props: FxHashMap<CompactString, PropInfo>,
}

#[derive(Debug, Clone)]
struct PropInfo {
    required: bool,
    #[allow(dead_code)] // Will be used for type checking in future
    prop_type: Option<CompactString>,
}

/// Analyze props validation across component boundaries.
///
/// This analyzer checks:
/// 1. Props passed to children are declared in their defineProps
/// 2. Required props are always passed
pub fn analyze_props_validation(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<PropsValidationIssue>, Vec<CrossFileDiagnostic>) {
    let mut issues = Vec::new();
    let mut diagnostics = Vec::new();

    // Build a map of component name -> props info
    let mut component_props: FxHashMap<CompactString, (FileId, ComponentPropsInfo)> =
        FxHashMap::default();

    for entry in registry.iter() {
        if !entry.is_vue_sfc {
            continue;
        }

        let Some(ref component_name) = entry.component_name else {
            continue;
        };

        let mut props_info = ComponentPropsInfo::default();

        // Extract props from macros
        for prop in entry.analysis.macros.props() {
            props_info.props.insert(
                prop.name.clone(),
                PropInfo {
                    required: prop.required,
                    prop_type: prop.prop_type.clone(),
                },
            );
        }

        component_props.insert(component_name.clone(), (entry.id, props_info));
    }

    // Now check each component usage
    for (parent_id, child_id) in graph.component_usage() {
        let Some(parent_entry) = registry.get(parent_id) else {
            continue;
        };
        let Some(child_entry) = registry.get(child_id) else {
            continue;
        };
        let Some(ref child_component_name) = child_entry.component_name else {
            continue;
        };

        // Get the child's props info
        let Some((_, child_props_info)) = component_props.get(child_component_name) else {
            continue;
        };

        // Get props passed by parent
        // This requires parsing the template to find the actual props passed
        // For now, we focus on checking required props from the child's perspective
        let passed_props = extract_passed_props_for_component(
            &parent_entry.analysis,
            child_component_name.as_str(),
        );

        // Check for missing required props
        for (prop_name, prop_info) in &child_props_info.props {
            if prop_info.required && !passed_props.contains(prop_name.as_str()) {
                let issue = PropsValidationIssue {
                    parent_file: parent_id,
                    child_file: child_id,
                    component_name: child_component_name.clone(),
                    kind: PropsValidationIssueKind::MissingRequiredProp {
                        prop_name: prop_name.clone(),
                    },
                    offset: 0,
                };
                issues.push(issue);

                let diagnostic = CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::MissingRequiredProp {
                        prop_name: prop_name.clone(),
                        component_name: child_component_name.clone(),
                    },
                    DiagnosticSeverity::Error,
                    parent_id,
                    0,
                    format!(
                        "**Missing Required Prop**: `{}` must be passed to `<{}>`\n\n\
                        This prop is declared as required in the component's `defineProps`.",
                        prop_name, child_component_name
                    ),
                )
                .with_related(
                    child_id,
                    0,
                    format!("Prop `{}` is declared as required here", prop_name),
                );

                diagnostics.push(diagnostic);
            }
        }

        // Check for undeclared props (props passed but not in defineProps)
        for passed_prop in &passed_props {
            // Skip built-in attributes
            if is_builtin_attr(passed_prop) {
                continue;
            }

            // Skip event handlers (@xxx or v-on:xxx)
            if passed_prop.starts_with('@') || passed_prop.starts_with("on") {
                continue;
            }

            // Check if this prop is declared
            let is_declared = child_props_info.props.contains_key(*passed_prop);

            if !is_declared {
                let issue = PropsValidationIssue {
                    parent_file: parent_id,
                    child_file: child_id,
                    component_name: child_component_name.clone(),
                    kind: PropsValidationIssueKind::UndeclaredProp {
                        prop_name: CompactString::new(*passed_prop),
                    },
                    offset: 0,
                };
                issues.push(issue);

                let diagnostic = CrossFileDiagnostic::new(
                    CrossFileDiagnosticKind::UndeclaredProp {
                        prop_name: CompactString::new(*passed_prop),
                        component_name: child_component_name.clone(),
                    },
                    DiagnosticSeverity::Warning, // Warning since it might be intentional $attrs
                    parent_id,
                    0,
                    format!(
                        "**Undeclared Prop**: `{}` is passed to `<{}>` but not declared\n\n\
                        The prop is not defined in the component's `defineProps`.\n\
                        If intentional, it will fall through to the root element via `$attrs`.",
                        passed_prop, child_component_name
                    ),
                )
                .with_suggestion(format!(
                    "Add to defineProps:\n```typescript\ndefineProps<{{\n  {}: unknown\n}}>()\n```\n\n\
                    Or use `v-bind=\"$attrs\"` in the child component for fallthrough.",
                    passed_prop
                ));

                diagnostics.push(diagnostic);
            }
        }
    }

    (issues, diagnostics)
}

/// Extract props passed to a specific component from the analysis.
///
/// Uses component_usages to find props passed to the component.
fn extract_passed_props_for_component<'a>(
    analysis: &'a crate::Croquis,
    component_name: &str,
) -> FxHashSet<&'a str> {
    let mut props = FxHashSet::default();

    for usage in &analysis.component_usages {
        // Match component name (case-insensitive for kebab-case vs PascalCase)
        if usage.name.as_str().eq_ignore_ascii_case(component_name)
            || to_pascal_case(usage.name.as_str()).eq_ignore_ascii_case(component_name)
        {
            for prop in &usage.props {
                props.insert(prop.name.as_str());
            }
        }
    }

    props
}

/// Convert kebab-case to PascalCase.
#[inline]
fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

/// Check if an attribute name is a built-in HTML/Vue attribute.
#[inline]
fn is_builtin_attr(name: &str) -> bool {
    matches!(
        name,
        "key"
            | "ref"
            | "is"
            | "class"
            | "style"
            | "id"
            | "slot"
            | "slot-scope"
            | "v-slot"
            | "v-if"
            | "v-else"
            | "v-else-if"
            | "v-for"
            | "v-show"
            | "v-bind"
            | "v-on"
            | "v-model"
            | "v-html"
            | "v-text"
            | "v-pre"
            | "v-cloak"
            | "v-once"
            | "v-memo"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_attr() {
        assert!(is_builtin_attr("key"));
        assert!(is_builtin_attr("ref"));
        assert!(is_builtin_attr("v-model"));
        assert!(!is_builtin_attr("myProp"));
        assert!(!is_builtin_attr("customAttr"));
    }
}
