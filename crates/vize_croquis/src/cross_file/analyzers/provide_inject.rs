//! Provide/Inject analysis.
//!
//! Matches provide() calls with inject() consumers across the component tree:
//! - Unmatched inject (no provider found in ancestors)
//! - Unused provide (no inject found in descendants)
//! - Type mismatches between provide and inject

use crate::cross_file::diagnostics::{
    CrossFileDiagnostic, CrossFileDiagnosticKind, DiagnosticSeverity,
};
use crate::cross_file::graph::{DependencyEdge, DependencyGraph};
use crate::cross_file::registry::{FileId, ModuleRegistry};
use crate::provide::{InjectEntry, InjectPattern, ProvideEntry, ProvideKey};
use vize_carton::{CompactString, FxHashMap, FxHashSet};

/// Information about a provide/inject match.
#[derive(Debug, Clone)]
pub struct ProvideInjectMatch {
    /// Component providing the value.
    pub provider: FileId,
    /// Component injecting the value.
    pub consumer: FileId,
    /// The provide/inject key.
    pub key: CompactString,
    /// Path from provider to consumer.
    pub path: Vec<FileId>,
    /// Whether types match (if available).
    pub type_match: Option<bool>,
    /// Provider offset in source.
    pub provide_offset: u32,
    /// Consumer offset in source.
    pub inject_offset: u32,
}

/// Analyze provide/inject relationships across the component tree.
pub fn analyze_provide_inject(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
) -> (Vec<ProvideInjectMatch>, Vec<CrossFileDiagnostic>) {
    let mut matches = Vec::new();
    let mut diagnostics = Vec::new();

    // Collect all provides and injects
    let mut provides: FxHashMap<FileId, Vec<ProvideEntry>> = FxHashMap::default();
    let mut injects: FxHashMap<FileId, Vec<InjectEntry>> = FxHashMap::default();

    for entry in registry.vue_components() {
        // Extract provide/inject from analysis
        // In a full implementation, this would come from script_parser
        let (p, i) = extract_provide_inject(&entry.analysis);
        if !p.is_empty() {
            provides.insert(entry.id, p);
        }
        if !i.is_empty() {
            injects.insert(entry.id, i);
        }
    }

    // Track which provides are used
    let mut used_provides: FxHashSet<(FileId, CompactString)> = FxHashSet::default();

    // For each inject, try to find a matching provide in ancestors
    for (&consumer_id, consumer_injects) in &injects {
        for inject in consumer_injects {
            let key_str = match &inject.key {
                ProvideKey::String(s) => s.clone(),
                ProvideKey::Symbol(s) => s.clone(),
            };

            // Check for destructured inject - this causes reactivity loss
            match &inject.pattern {
                InjectPattern::ObjectDestructure(props) => {
                    diagnostics.push(
                        CrossFileDiagnostic::new(
                            CrossFileDiagnosticKind::HydrationMismatchRisk {
                                reason: CompactString::new(format!(
                                    "Destructuring inject('{}') loses reactivity",
                                    key_str
                                )),
                            },
                            DiagnosticSeverity::Error,
                            consumer_id,
                            inject.start,
                            format!(
                                "Destructuring inject('{}') into {{ {} }} breaks reactivity connection",
                                key_str,
                                props.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(", ")
                            ),
                        )
                        .with_suggestion(format!(
                            "Store inject result first: `const {} = inject('{}')`, then access properties",
                            inject.local_name,
                            key_str
                        )),
                    );
                }
                InjectPattern::ArrayDestructure(items) => {
                    diagnostics.push(
                        CrossFileDiagnostic::new(
                            CrossFileDiagnosticKind::HydrationMismatchRisk {
                                reason: CompactString::new(format!(
                                    "Array destructuring inject('{}') loses reactivity",
                                    key_str
                                )),
                            },
                            DiagnosticSeverity::Error,
                            consumer_id,
                            inject.start,
                            format!(
                                "Array destructuring inject('{}') into [{}] breaks reactivity connection",
                                key_str,
                                items.iter().map(|p| p.as_str()).collect::<Vec<_>>().join(", ")
                            ),
                        )
                        .with_suggestion(format!(
                            "Store inject result first: `const {} = inject('{}')`, then access indices",
                            inject.local_name,
                            key_str
                        )),
                    );
                }
                InjectPattern::Simple => {
                    // No reactivity loss issue
                }
            }

            // Search ancestors for a matching provide
            let provider_match = find_provider(consumer_id, &key_str, &provides, graph);

            match provider_match {
                Some((provider_id, provide_entry, path)) => {
                    // Found a match
                    used_provides.insert((provider_id, key_str.clone()));

                    matches.push(ProvideInjectMatch {
                        provider: provider_id,
                        consumer: consumer_id,
                        key: key_str.clone(),
                        path,
                        type_match: None, // Would need type analysis
                        provide_offset: provide_entry.start,
                        inject_offset: inject.start,
                    });
                }
                None => {
                    // No provider found
                    if inject.default_value.is_none() {
                        diagnostics.push(
                            CrossFileDiagnostic::new(
                                CrossFileDiagnosticKind::UnmatchedInject {
                                    key: key_str.clone(),
                                },
                                DiagnosticSeverity::Error,
                                consumer_id,
                                inject.start,
                                format!(
                                    "inject('{}') has no matching provide() in ancestor components",
                                    key_str
                                ),
                            )
                            .with_suggestion(
                                "Add provide() in a parent component or provide a default value",
                            ),
                        );
                    } else {
                        // Has default, just info
                        diagnostics.push(CrossFileDiagnostic::new(
                            CrossFileDiagnosticKind::UnmatchedInject {
                                key: key_str.clone(),
                            },
                            DiagnosticSeverity::Info,
                            consumer_id,
                            inject.start,
                            format!(
                                "inject('{}') uses default value (no ancestor provides it)",
                                key_str
                            ),
                        ));
                    }
                }
            }
        }
    }

    // Check for unused provides
    for (&provider_id, provider_provides) in &provides {
        for provide in provider_provides {
            let key_str = match &provide.key {
                ProvideKey::String(s) => s.clone(),
                ProvideKey::Symbol(s) => s.clone(),
            };

            if !used_provides.contains(&(provider_id, key_str.clone())) {
                // Check if any descendant injects this key
                let has_descendant_inject =
                    has_inject_in_descendants(provider_id, &key_str, &injects, graph);

                if !has_descendant_inject {
                    diagnostics.push(
                        CrossFileDiagnostic::new(
                            CrossFileDiagnosticKind::UnusedProvide {
                                key: key_str.clone(),
                            },
                            DiagnosticSeverity::Warning,
                            provider_id,
                            provide.start,
                            format!(
                                "provide('{}') is not used by any descendant component",
                                key_str
                            ),
                        )
                        .with_suggestion(
                            "Remove if not needed, or add inject() in a child component",
                        ),
                    );
                }
            }
        }
    }

    (matches, diagnostics)
}

/// Extract provide/inject calls from a component's analysis.
/// Uses the ProvideInjectTracker for precise static analysis - no heuristics.
#[inline]
fn extract_provide_inject(analysis: &crate::Croquis) -> (Vec<ProvideEntry>, Vec<InjectEntry>) {
    // Use the actual provide/inject tracker data - precise static analysis
    let provides = analysis.provide_inject.provides().to_vec();
    let injects = analysis.provide_inject.injects().to_vec();
    (provides, injects)
}

/// Find a provider for a given key in ancestor components.
fn find_provider(
    consumer: FileId,
    key: &str,
    provides: &FxHashMap<FileId, Vec<ProvideEntry>>,
    graph: &DependencyGraph,
) -> Option<(FileId, ProvideEntry, Vec<FileId>)> {
    let mut visited = FxHashSet::default();
    let mut queue = vec![(consumer, vec![consumer])];

    while let Some((current, path)) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Check if current component provides this key
        if current != consumer {
            if let Some(component_provides) = provides.get(&current) {
                for provide in component_provides {
                    let provide_key = match &provide.key {
                        ProvideKey::String(s) => s.as_str(),
                        ProvideKey::Symbol(s) => s.as_str(),
                    };
                    if provide_key == key {
                        return Some((current, provide.clone(), path));
                    }
                }
            }
        }

        // Add parents (components that use this one) to queue
        for (parent_id, edge_type) in graph.dependents(current) {
            if edge_type == DependencyEdge::ComponentUsage && !visited.contains(&parent_id) {
                let mut new_path = path.clone();
                new_path.push(parent_id);
                queue.push((parent_id, new_path));
            }
        }
    }

    None
}

/// Check if any descendant component injects a given key.
fn has_inject_in_descendants(
    provider: FileId,
    key: &str,
    injects: &FxHashMap<FileId, Vec<InjectEntry>>,
    graph: &DependencyGraph,
) -> bool {
    let mut visited = FxHashSet::default();
    let mut queue = vec![provider];

    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        // Check descendants (components used by this one)
        for (child_id, edge_type) in graph.dependencies(current) {
            if edge_type == DependencyEdge::ComponentUsage {
                // Check if child injects this key
                if let Some(child_injects) = injects.get(&child_id) {
                    for inject in child_injects {
                        let inject_key = match &inject.key {
                            ProvideKey::String(s) => s.as_str(),
                            ProvideKey::Symbol(s) => s.as_str(),
                        };
                        if inject_key == key {
                            return true;
                        }
                    }
                }

                if !visited.contains(&child_id) {
                    queue.push(child_id);
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provide_key_match() {
        let key1 = ProvideKey::String(CompactString::new("theme"));
        let key2 = ProvideKey::String(CompactString::new("theme"));

        let s1 = match &key1 {
            ProvideKey::String(s) => s.as_str(),
            ProvideKey::Symbol(s) => s.as_str(),
        };
        let s2 = match &key2 {
            ProvideKey::String(s) => s.as_str(),
            ProvideKey::Symbol(s) => s.as_str(),
        };

        assert_eq!(s1, s2);
    }
}
