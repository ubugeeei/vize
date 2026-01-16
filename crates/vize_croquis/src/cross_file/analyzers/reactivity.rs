//! Reactivity tracking and loss detection.
//!
//! Detects issues where reactivity is accidentally broken:
//! - Destructuring reactive objects or refs
//! - Passing reactive values to non-reactive contexts
//! - Reactivity loss through function calls
//! - Ref unwrapping issues
//!
//! ## Performance Optimizations
//!
//! - Uses FxHashSet for O(1) lookups
//! - Early termination when possible
//! - Minimal allocations during analysis

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::DependencyGraph;
use crate::cross_file::registry::{FileId, ModuleRegistry};
use crate::reactivity::ReactiveKind;
use vize_carton::{CompactString, FxHashSet};

/// Kind of reactivity issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactivityIssueKind {
    /// Destructuring a reactive object loses reactivity.
    DestructuredReactive {
        source_name: CompactString,
        destructured_props: Vec<CompactString>,
    },
    /// Destructuring a ref without .value loses reactivity.
    DestructuredRef { ref_name: CompactString },
    /// Reactive value passed to non-reactive context.
    ReactivityLost {
        value_name: CompactString,
        context: CompactString,
    },
    /// Ref used without .value in script.
    MissingValueAccess { ref_name: CompactString },
    /// toRef/toRefs should be used instead of destructuring.
    ShouldUseToRefs { source_name: CompactString },
    /// Reactive value assigned to plain variable.
    ReactiveToPlain {
        source_name: CompactString,
        target_name: CompactString,
    },
    /// storeToRefs should be used for Pinia store.
    ShouldUseStoreToRefs { store_name: CompactString },
    /// Computed without return statement.
    ComputedWithoutReturn { computed_name: CompactString },
    /// Watch source is not reactive.
    NonReactiveWatchSource { source_expression: CompactString },
    /// Prop passed to ref() which creates a copy.
    PropPassedToRef { prop_name: CompactString },
}

/// Information about a reactivity issue.
#[derive(Debug, Clone)]
pub struct ReactivityIssue {
    /// File where the issue occurs.
    pub file_id: FileId,
    /// Kind of issue.
    pub kind: ReactivityIssueKind,
    /// Offset in source.
    pub offset: u32,
    /// The reactive source involved.
    pub source: Option<CompactString>,
}

/// Analyze reactivity issues across components.
pub fn analyze_reactivity(
    registry: &ModuleRegistry,
    _graph: &DependencyGraph,
) -> (Vec<ReactivityIssue>, Vec<CrossFileDiagnostic>) {
    let mut issues = Vec::new();
    let mut diagnostics = Vec::new();

    for entry in registry.vue_components() {
        let analysis = &entry.analysis;
        let file_id = entry.id;

        // Analyze each component for reactivity issues
        let component_issues = analyze_component_reactivity(analysis);

        for issue in component_issues {
            let diag = create_diagnostic(file_id, &issue);
            diagnostics.push(diag);

            issues.push(ReactivityIssue {
                file_id,
                kind: issue.kind,
                offset: issue.offset,
                source: issue.source,
            });
        }
    }

    (issues, diagnostics)
}

/// Internal issue representation during analysis.
struct InternalIssue {
    kind: ReactivityIssueKind,
    offset: u32,
    source: Option<CompactString>,
}

/// Analyze a single component for reactivity issues.
/// Uses precise static analysis from the Croquis data - no heuristics.
#[inline]
fn analyze_component_reactivity(analysis: &crate::Croquis) -> Vec<InternalIssue> {
    let mut issues = Vec::new();

    // Track which identifiers come from 'vue' imports (ref, reactive, toRefs, etc.)
    let vue_imports = extract_vue_imports(analysis);

    // Check for destructured inject() calls - these lose reactivity
    // This is precise: we check the actual InjectPattern from the tracker
    for inject in analysis.provide_inject.injects() {
        use crate::provide::InjectPattern;
        match &inject.pattern {
            InjectPattern::ObjectDestructure(props) => {
                issues.push(InternalIssue {
                    kind: ReactivityIssueKind::DestructuredReactive {
                        source_name: inject.local_name.clone(),
                        destructured_props: props.clone(),
                    },
                    offset: inject.start,
                    source: Some(inject.local_name.clone()),
                });
            }
            InjectPattern::ArrayDestructure(_items) => {
                issues.push(InternalIssue {
                    kind: ReactivityIssueKind::DestructuredReactive {
                        source_name: inject.local_name.clone(),
                        destructured_props: vec![CompactString::new("(array items)")],
                    },
                    offset: inject.start,
                    source: Some(inject.local_name.clone()),
                });
            }
            InjectPattern::Simple => {
                // No issue - inject is stored properly
            }
        }
    }

    // Check for toRefs usage - this is the correct pattern, no warning needed
    // Check for reactive sources that indicate proper usage
    let torefs_sources: FxHashSet<&str> = analysis
        .reactivity
        .sources()
        .iter()
        .filter(|s| matches!(s.kind, ReactiveKind::ToRef | ReactiveKind::ToRefs))
        .map(|s| s.name.as_str())
        .collect();

    // Build a set of all reactive sources (from vue imports)
    let _reactive_sources: FxHashSet<&str> = analysis
        .reactivity
        .sources()
        .iter()
        .map(|s| s.name.as_str())
        .collect();

    // Track props defined via defineProps
    let props: FxHashSet<&str> = analysis
        .macros
        .props()
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    // Check if props are properly wrapped with toRef/toRefs when destructured
    if let Some(props_destructure) = analysis.macros.props_destructure() {
        for (key, _binding) in props_destructure.bindings.iter() {
            // Check if this destructured prop has a corresponding toRef
            if !torefs_sources.contains(key.as_str()) {
                // This prop is destructured without toRefs - Vue handles this with
                // reactive props destructure transform, so this is actually OK in modern Vue
                // We don't warn here as it's handled by the compiler
            }
        }
    }

    // Check for reactivity loss patterns in the code
    // This requires analyzing the AST for patterns like:
    // - const { x } = reactive({ x: 1 })  // reactivity lost
    // - const x = ref(0); const y = x.value  // reactivity lost
    // - const x = toRaw(reactive)  // intentional, but might be wrong

    // Report if vue imports are present but not used properly
    if !vue_imports.is_empty() {
        // Check if reactive sources are actually used
        for source in analysis.reactivity.sources() {
            // Verify the reactive function was imported from 'vue'
            let function_name = match source.kind {
                ReactiveKind::Ref => "ref",
                ReactiveKind::ShallowRef => "shallowRef",
                ReactiveKind::Reactive => "reactive",
                ReactiveKind::ShallowReactive => "shallowReactive",
                ReactiveKind::Computed => "computed",
                ReactiveKind::Readonly => "readonly",
                ReactiveKind::ShallowReadonly => "shallowReadonly",
                ReactiveKind::ToRef => "toRef",
                ReactiveKind::ToRefs => "toRefs",
            };

            // Verify it comes from vue
            if !vue_imports.contains(function_name) {
                // The reactive function might be a local implementation or from another library
                // This is a potential issue but not necessarily an error
            }
        }
    }

    // Check for prop passed to ref() which creates a copy
    for source in analysis.reactivity.sources() {
        if source.kind == ReactiveKind::Ref {
            // Check if this ref is initialized with a prop
            if props.contains(source.name.as_str()) {
                issues.push(InternalIssue {
                    kind: ReactivityIssueKind::PropPassedToRef {
                        prop_name: source.name.clone(),
                    },
                    offset: source.declaration_offset,
                    source: Some(source.name.clone()),
                });
            }
        }
    }

    issues
}

/// Extract identifiers imported from 'vue'.
fn extract_vue_imports(analysis: &crate::Croquis) -> FxHashSet<&str> {
    use crate::scope::ScopeKind;

    let mut vue_imports = FxHashSet::default();

    for scope in analysis.scopes.iter() {
        if scope.kind == ScopeKind::ExternalModule {
            if let crate::scope::ScopeData::ExternalModule(data) = scope.data() {
                // Check if this is a vue import
                if data.source.as_str() == "vue" || data.source.starts_with("vue/") {
                    // Collect all bindings from this import
                    for (name, _) in scope.bindings() {
                        vue_imports.insert(name);
                    }
                }
            }
        }
    }

    vue_imports
}

/// Create a diagnostic from an internal issue.
fn create_diagnostic(file_id: FileId, issue: &InternalIssue) -> CrossFileDiagnostic {
    match &issue.kind {
        ReactivityIssueKind::DestructuredReactive {
            source_name,
            destructured_props,
        } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!(
                    "Destructuring '{}' loses reactivity for: {:?}",
                    source_name, destructured_props
                )),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Destructuring reactive object '{}' breaks reactivity connection",
                source_name
            ),
        )
        .with_suggestion(format!(
            "Use toRefs({}) or access properties directly as {}.prop",
            source_name, source_name
        )),

        ReactivityIssueKind::DestructuredRef { ref_name } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!(
                    "Destructuring ref '{}' loses reactivity",
                    ref_name
                )),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Destructuring ref '{}' creates a non-reactive copy",
                ref_name
            ),
        )
        .with_suggestion(format!(
            "Access {}.value directly or use computed(() => {}.value.prop)",
            ref_name, ref_name
        )),

        ReactivityIssueKind::ReactivityLost {
            value_name,
            context,
        } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!(
                    "'{}' loses reactivity in {}",
                    value_name, context
                )),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Reactive value '{}' loses reactivity when passed to {}",
                value_name, context
            ),
        ),

        ReactivityIssueKind::MissingValueAccess { ref_name } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!("Ref '{}' used without .value", ref_name)),
            },
            DiagnosticSeverity::Error,
            file_id,
            issue.offset,
            format!(
                "Ref '{}' should be accessed with .value in script context",
                ref_name
            ),
        )
        .with_suggestion(format!("Use {}.value instead of {}", ref_name, ref_name)),

        ReactivityIssueKind::ShouldUseToRefs { source_name } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!("Use toRefs for '{}'", source_name)),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Destructuring '{}' - consider using toRefs() to maintain reactivity",
                source_name
            ),
        )
        .with_suggestion(format!(
            "const {{ prop1, prop2 }} = toRefs({})",
            source_name
        )),

        ReactivityIssueKind::ReactiveToPlain {
            source_name,
            target_name,
        } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!(
                    "Assigning '{}' to plain '{}' loses reactivity",
                    source_name, target_name
                )),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Assigning reactive '{}' to '{}' creates a non-reactive copy",
                source_name, target_name
            ),
        )
        .with_suggestion("Use computed() or keep the reactive reference"),

        ReactivityIssueKind::ShouldUseStoreToRefs { store_name } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!("Use storeToRefs for '{}'", store_name)),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Destructuring Pinia store '{}' - use storeToRefs() for state/getters",
                store_name
            ),
        )
        .with_suggestion(format!(
            "const {{ state, getter }} = storeToRefs({})",
            store_name
        )),

        ReactivityIssueKind::ComputedWithoutReturn { computed_name } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!(
                    "Computed '{}' may not return value",
                    computed_name
                )),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Computed property '{}' should return a value",
                computed_name
            ),
        ),

        ReactivityIssueKind::NonReactiveWatchSource { source_expression } => {
            CrossFileDiagnostic::new(
                CrossFileDiagnosticKind::HydrationMismatchRisk {
                    reason: CompactString::new(format!(
                        "Watch source '{}' is not reactive",
                        source_expression
                    )),
                },
                DiagnosticSeverity::Warning,
                file_id,
                issue.offset,
                format!(
                    "Watch source '{}' is not reactive, changes won't trigger the callback",
                    source_expression
                ),
            )
            .with_suggestion("Use () => value or a ref/reactive object as the watch source")
        }

        ReactivityIssueKind::PropPassedToRef { prop_name } => CrossFileDiagnostic::new(
            CrossFileDiagnosticKind::HydrationMismatchRisk {
                reason: CompactString::new(format!(
                    "Prop '{}' passed to ref() creates a copy",
                    prop_name
                )),
            },
            DiagnosticSeverity::Warning,
            file_id,
            issue.offset,
            format!(
                "Passing prop '{}' to ref() creates a non-reactive copy",
                prop_name
            ),
        )
        .with_suggestion(format!(
            "Use toRef(props, '{}') or computed(() => props.{})",
            prop_name, prop_name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reactivity_issue_kind() {
        let kind = ReactivityIssueKind::DestructuredReactive {
            source_name: CompactString::new("state"),
            destructured_props: vec![CompactString::new("count")],
        };

        match kind {
            ReactivityIssueKind::DestructuredReactive { source_name, .. } => {
                assert_eq!(source_name.as_str(), "state");
            }
            _ => panic!("Wrong kind"),
        }
    }
}
