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

/// Tree representation of provide/inject relationships.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ProvideInjectTree {
    /// Root nodes (components that provide but don't inject from ancestors).
    pub roots: Vec<ProvideNode>,
}

/// A node in the provide/inject tree.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ProvideNode {
    /// File ID of this component.
    pub file_id: FileId,
    /// Component name (if available).
    pub component_name: Option<CompactString>,
    /// Keys provided by this component.
    pub provides: Vec<ProvideInfo>,
    /// Keys injected by this component.
    pub injects: Vec<InjectInfo>,
    /// Children components that inject from this provider.
    pub children: Vec<ProvideNode>,
}

/// Information about a provide call.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ProvideInfo {
    /// The provide key.
    pub key: CompactString,
    /// The provided type (if available).
    pub value_type: Option<CompactString>,
    /// Source offset.
    pub offset: u32,
    /// Number of consumers.
    pub consumer_count: usize,
}

/// Information about an inject call.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct InjectInfo {
    /// The inject key.
    pub key: CompactString,
    /// Whether a default value is provided.
    pub has_default: bool,
    /// The provider file (if found).
    pub provider: Option<FileId>,
    /// Source offset.
    pub offset: u32,
}

#[allow(unused)]
impl ProvideInjectTree {
    /// Render the tree as a markdown string for visualization.
    pub fn to_markdown(&self, registry: &ModuleRegistry) -> String {
        let mut output = String::with_capacity(4096);
        output.push_str("## Provide/Inject Tree\n\n");

        if self.roots.is_empty() {
            output.push_str("_No provide/inject relationships found._\n");
            return output;
        }

        for root in &self.roots {
            Self::render_node(&mut output, root, registry, 0);
        }

        output
    }

    fn render_node(
        output: &mut String,
        node: &ProvideNode,
        registry: &ModuleRegistry,
        depth: usize,
    ) {
        use std::fmt::Write;

        let indent = "  ".repeat(depth);
        let name = node
            .component_name
            .as_deref()
            .or_else(|| {
                registry
                    .get(node.file_id)
                    .and_then(|e| e.path.file_stem()?.to_str())
            })
            .unwrap_or("<unknown>");

        // Component name
        writeln!(output, "{}📦 **{}**", indent, name).ok();

        // Provides
        if !node.provides.is_empty() {
            for p in &node.provides {
                let type_str = p
                    .value_type
                    .as_deref()
                    .map(|t| format!(": `{}`", t))
                    .unwrap_or_default();
                let consumers = if p.consumer_count > 0 {
                    format!(" → {} consumer(s)", p.consumer_count)
                } else {
                    " ⚠️ _unused_".to_string()
                };
                writeln!(
                    output,
                    "{}  🔹 provide(`\"{}\"`){}{} ",
                    indent, p.key, type_str, consumers
                )
                .ok();
            }
        }

        // Injects
        if !node.injects.is_empty() {
            for i in &node.injects {
                let default_str = if i.has_default { " (has default)" } else { "" };
                let provider_str = if i.provider.is_some() {
                    " ✅"
                } else {
                    " ❌ _no provider_"
                };
                writeln!(
                    output,
                    "{}  🔸 inject(`\"{}\"`){}{} ",
                    indent, i.key, default_str, provider_str
                )
                .ok();
            }
        }

        // Children
        for child in &node.children {
            Self::render_node(output, child, registry, depth + 1);
        }
    }
}

/// Build the provide/inject tree from analysis results.
#[allow(unused)]
pub fn build_provide_inject_tree(
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
    matches: &[ProvideInjectMatch],
) -> ProvideInjectTree {
    // Collect all provides and injects
    let mut provides_map: FxHashMap<FileId, Vec<ProvideEntry>> = FxHashMap::default();
    let mut injects_map: FxHashMap<FileId, Vec<InjectEntry>> = FxHashMap::default();
    let mut consumer_counts: FxHashMap<(FileId, CompactString), usize> = FxHashMap::default();

    for entry in registry.vue_components() {
        let (p, i) = extract_provide_inject(&entry.analysis);
        if !p.is_empty() {
            provides_map.insert(entry.id, p);
        }
        if !i.is_empty() {
            injects_map.insert(entry.id, i);
        }
    }

    // Count consumers for each provide
    for m in matches {
        *consumer_counts
            .entry((m.provider, m.key.clone()))
            .or_insert(0) += 1;
    }

    // Build tree starting from components that provide but have no ancestor providers
    let mut roots = Vec::new();
    let mut visited = FxHashSet::default();

    for &file_id in provides_map.keys() {
        if visited.contains(&file_id) {
            continue;
        }

        // Check if this component has any ancestor that provides
        let has_ancestor_provider = has_ancestor_with_provide(file_id, graph, &provides_map);

        if !has_ancestor_provider {
            let node = build_node(
                file_id,
                registry,
                graph,
                &provides_map,
                &injects_map,
                &consumer_counts,
                matches,
                &mut visited,
            );
            roots.push(node);
        }
    }

    // Also add components that only inject (no provides) but have no ancestor
    for &file_id in injects_map.keys() {
        if visited.contains(&file_id) || provides_map.contains_key(&file_id) {
            continue;
        }

        let node = build_node(
            file_id,
            registry,
            graph,
            &provides_map,
            &injects_map,
            &consumer_counts,
            matches,
            &mut visited,
        );
        if !node.injects.is_empty() {
            roots.push(node);
        }
    }

    ProvideInjectTree { roots }
}

#[allow(unused)]
fn has_ancestor_with_provide(
    file_id: FileId,
    graph: &DependencyGraph,
    provides_map: &FxHashMap<FileId, Vec<ProvideEntry>>,
) -> bool {
    let mut visited = FxHashSet::default();
    let mut queue = vec![file_id];

    while let Some(current) = queue.pop() {
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        for (parent_id, edge_type) in graph.dependents(current) {
            if edge_type == DependencyEdge::ComponentUsage {
                if provides_map.contains_key(&parent_id) {
                    return true;
                }
                queue.push(parent_id);
            }
        }
    }

    false
}

#[allow(unused, clippy::too_many_arguments)]
fn build_node(
    file_id: FileId,
    registry: &ModuleRegistry,
    graph: &DependencyGraph,
    provides_map: &FxHashMap<FileId, Vec<ProvideEntry>>,
    injects_map: &FxHashMap<FileId, Vec<InjectEntry>>,
    consumer_counts: &FxHashMap<(FileId, CompactString), usize>,
    matches: &[ProvideInjectMatch],
    visited: &mut FxHashSet<FileId>,
) -> ProvideNode {
    visited.insert(file_id);

    let component_name = registry.get(file_id).and_then(|e| e.component_name.clone());

    // Build provides info
    let provides: Vec<ProvideInfo> = provides_map
        .get(&file_id)
        .map(|ps| {
            ps.iter()
                .map(|p| {
                    let key = match &p.key {
                        ProvideKey::String(s) => s.clone(),
                        ProvideKey::Symbol(s) => s.clone(),
                    };
                    let count = *consumer_counts.get(&(file_id, key.clone())).unwrap_or(&0);
                    ProvideInfo {
                        key,
                        value_type: p.value_type.clone(),
                        offset: p.start,
                        consumer_count: count,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Build injects info
    let injects = injects_map
        .get(&file_id)
        .map(|is| {
            is.iter()
                .map(|i| {
                    let key = match &i.key {
                        ProvideKey::String(s) => s.clone(),
                        ProvideKey::Symbol(s) => s.clone(),
                    };
                    let provider = matches
                        .iter()
                        .find(|m| m.consumer == file_id && m.key == key)
                        .map(|m| m.provider);
                    InjectInfo {
                        key,
                        has_default: i.default_value.is_some(),
                        provider,
                        offset: i.start,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Find children (components that inject from this provider)
    let mut children = Vec::new();
    for (child_id, edge_type) in graph.dependencies(file_id) {
        if edge_type == DependencyEdge::ComponentUsage && !visited.contains(&child_id) {
            // Check if child injects something we provide
            let child_injects_from_us = injects_map.get(&child_id).is_some_and(|child_injects| {
                child_injects.iter().any(|ci| {
                    let ci_key = match &ci.key {
                        ProvideKey::String(s) => s.as_str(),
                        ProvideKey::Symbol(s) => s.as_str(),
                    };
                    provides.iter().any(|p| p.key.as_str() == ci_key)
                })
            });

            if child_injects_from_us || provides_map.contains_key(&child_id) {
                let child_node = build_node(
                    child_id,
                    registry,
                    graph,
                    provides_map,
                    injects_map,
                    consumer_counts,
                    matches,
                    visited,
                );
                children.push(child_node);
            }
        }
    }

    ProvideNode {
        file_id,
        component_name,
        provides,
        injects,
        children,
    }
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
                                    "**Unmatched Inject**: `inject('{}')` has no matching `provide()` in any ancestor component\n\n\
                                    This will return `undefined` at runtime and may cause errors.\n\n\
                                    ### Checklist:\n\
                                    - [ ] Add `provide('{}', value)` in a parent/ancestor component\n\
                                    - [ ] Or provide a default value: `inject('{}', defaultValue)`",
                                    key_str, key_str, key_str
                                ),
                            )
                            .with_suggestion(format!(
                                "```typescript\n// In parent component:\nprovide('{}', yourValue)\n\n// Or with default:\nconst {} = inject('{}', defaultValue)\n```",
                                key_str, inject.local_name, key_str
                            )),
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
                                "**Info**: `inject('{}')` uses default value — no ancestor provides this key",
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
