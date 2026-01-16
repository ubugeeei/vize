//! Dependency graph for tracking import/export relationships.
//!
//! This module builds a directed graph of module dependencies based on
//! import statements, component registrations, and provide/inject relationships.
//!
//! ## Performance Optimizations
//!
//! - Uses `FxHashMap` for O(1) node lookup
//! - Uses `SmallVec` for edges (stack-allocated for small counts)
//! - Iterative algorithms avoid stack overflow on deep graphs
//! - Early termination in path finding algorithms

use super::registry::FileId;
use vize_carton::{CompactString, FxHashMap, FxHashSet, SmallVec};

/// Edge type in the dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DependencyEdge {
    /// Static ES module import: `import Foo from './Foo.vue'`
    Import = 0,
    /// Dynamic import: `const Foo = () => import('./Foo.vue')`
    DynamicImport = 1,
    /// Component used in template: `<Foo />`
    ComponentUsage = 2,
    /// Provide/Inject relationship (provider -> consumer)
    ProvideInject = 3,
    /// Event emission (child -> parent event handler)
    EventEmit = 4,
    /// Slot content (parent -> child slot)
    SlotContent = 5,
    /// Re-export: `export { Foo } from './Foo.vue'`
    ReExport = 6,
    /// Type-only import: `import type { Foo } from './types'`
    TypeImport = 7,
}

impl DependencyEdge {
    /// Get display name for this edge type.
    #[inline]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::DynamicImport => "dynamic-import",
            Self::ComponentUsage => "component",
            Self::ProvideInject => "provide-inject",
            Self::EventEmit => "emit",
            Self::SlotContent => "slot",
            Self::ReExport => "re-export",
            Self::TypeImport => "type-import",
        }
    }
}

/// A node in the dependency graph representing a module.
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// File ID reference.
    pub file_id: FileId,
    /// Module path (relative to project root).
    pub path: CompactString,
    /// Outgoing edges (this module depends on these).
    pub imports: SmallVec<[(FileId, DependencyEdge); 8]>,
    /// Incoming edges (these modules depend on this).
    pub importers: SmallVec<[(FileId, DependencyEdge); 8]>,
    /// Exported names from this module.
    pub exports: FxHashSet<CompactString>,
    /// Component name (for Vue SFCs).
    pub component_name: Option<CompactString>,
    /// Whether this module is an entry point (App.vue, main.ts).
    pub is_entry: bool,
}

impl ModuleNode {
    /// Create a new module node.
    pub fn new(file_id: FileId, path: impl Into<CompactString>) -> Self {
        Self {
            file_id,
            path: path.into(),
            imports: SmallVec::new(),
            importers: SmallVec::new(),
            exports: FxHashSet::default(),
            component_name: None,
            is_entry: false,
        }
    }
}

/// Dependency graph for a Vue project.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Map from file ID to module node.
    nodes: FxHashMap<FileId, ModuleNode>,
    /// Map from component name to file ID (for resolving template usage).
    component_index: FxHashMap<CompactString, FileId>,
    /// Entry points (App.vue, main.ts, etc.).
    entries: SmallVec<[FileId; 4]>,
    /// Detected circular dependencies.
    circular_deps: Vec<Vec<FileId>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a module node to the graph.
    pub fn add_node(&mut self, node: ModuleNode) {
        let file_id = node.file_id;

        // Index component name if present
        if let Some(ref name) = node.component_name {
            self.component_index.insert(name.clone(), file_id);
        }

        // Track entry points
        if node.is_entry {
            self.entries.push(file_id);
        }

        self.nodes.insert(file_id, node);
    }

    /// Add a dependency edge between two modules.
    pub fn add_edge(&mut self, from: FileId, to: FileId, edge_type: DependencyEdge) {
        // Add to importer's imports
        if let Some(from_node) = self.nodes.get_mut(&from) {
            if !from_node.imports.iter().any(|(id, _)| *id == to) {
                from_node.imports.push((to, edge_type));
            }
        }

        // Add to importee's importers
        if let Some(to_node) = self.nodes.get_mut(&to) {
            if !to_node.importers.iter().any(|(id, _)| *id == from) {
                to_node.importers.push((from, edge_type));
            }
        }
    }

    /// Get a node by file ID.
    #[inline]
    pub fn get_node(&self, id: FileId) -> Option<&ModuleNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable node by file ID.
    #[inline]
    pub fn get_node_mut(&mut self, id: FileId) -> Option<&mut ModuleNode> {
        self.nodes.get_mut(&id)
    }

    /// Find a file by component name.
    #[inline]
    pub fn find_by_component(&self, name: &str) -> Option<FileId> {
        self.component_index.get(name).copied()
    }

    /// Get all direct dependencies of a module.
    pub fn dependencies(&self, id: FileId) -> impl Iterator<Item = (FileId, DependencyEdge)> + '_ {
        self.nodes
            .get(&id)
            .into_iter()
            .flat_map(|n| n.imports.iter().copied())
    }

    /// Get all modules that depend on this module.
    pub fn dependents(&self, id: FileId) -> impl Iterator<Item = (FileId, DependencyEdge)> + '_ {
        self.nodes
            .get(&id)
            .into_iter()
            .flat_map(|n| n.importers.iter().copied())
    }

    /// Get transitive dependencies (all modules this depends on, recursively).
    pub fn transitive_dependencies(&self, id: FileId) -> FxHashSet<FileId> {
        let mut visited = FxHashSet::default();
        let mut stack = vec![id];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(node) = self.nodes.get(&current) {
                for (dep_id, _) in &node.imports {
                    if !visited.contains(dep_id) {
                        stack.push(*dep_id);
                    }
                }
            }
        }

        visited.remove(&id);
        visited
    }

    /// Get transitive dependents (all modules that depend on this, recursively).
    pub fn transitive_dependents(&self, id: FileId) -> FxHashSet<FileId> {
        let mut visited = FxHashSet::default();
        let mut stack = vec![id];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(node) = self.nodes.get(&current) {
                for (dep_id, _) in &node.importers {
                    if !visited.contains(dep_id) {
                        stack.push(*dep_id);
                    }
                }
            }
        }

        visited.remove(&id);
        visited
    }

    /// Detect circular dependencies using DFS.
    pub fn detect_circular_dependencies(&mut self) {
        self.circular_deps.clear();

        let mut visited = FxHashSet::default();
        let mut rec_stack = FxHashSet::default();
        let mut path = Vec::new();
        let mut cycles = Vec::new();

        // Collect all node IDs first to avoid borrow issues
        let node_ids: Vec<_> = self.nodes.keys().copied().collect();

        for start_id in node_ids {
            if !visited.contains(&start_id) {
                Self::dfs_cycle_static(
                    &self.nodes,
                    start_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        self.circular_deps = cycles;
    }

    fn dfs_cycle_static(
        nodes: &FxHashMap<FileId, ModuleNode>,
        id: FileId,
        visited: &mut FxHashSet<FileId>,
        rec_stack: &mut FxHashSet<FileId>,
        path: &mut Vec<FileId>,
        cycles: &mut Vec<Vec<FileId>>,
    ) {
        visited.insert(id);
        rec_stack.insert(id);
        path.push(id);

        if let Some(node) = nodes.get(&id) {
            for (dep_id, _) in &node.imports {
                if !visited.contains(dep_id) {
                    Self::dfs_cycle_static(nodes, *dep_id, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(dep_id) {
                    // Found a cycle - extract the cycle from path
                    if let Some(start) = path.iter().position(|p| p == dep_id) {
                        let cycle: Vec<_> = path[start..].to_vec();
                        cycles.push(cycle);
                    }
                }
            }
        }

        path.pop();
        rec_stack.remove(&id);
    }

    /// Get detected circular dependencies.
    #[inline]
    pub fn circular_dependencies(&self) -> &[Vec<FileId>] {
        &self.circular_deps
    }

    /// Check if there's a path from one module to another.
    pub fn has_path(&self, from: FileId, to: FileId) -> bool {
        if from == to {
            return true;
        }
        self.transitive_dependencies(from).contains(&to)
    }

    /// Get the shortest path between two modules.
    pub fn shortest_path(&self, from: FileId, to: FileId) -> Option<Vec<FileId>> {
        if from == to {
            return Some(vec![from]);
        }

        let mut visited = FxHashSet::default();
        let mut queue = std::collections::VecDeque::new();
        let mut parent: FxHashMap<FileId, FileId> = FxHashMap::default();

        visited.insert(from);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            if let Some(node) = self.nodes.get(&current) {
                for (dep_id, _) in &node.imports {
                    if !visited.contains(dep_id) {
                        visited.insert(*dep_id);
                        parent.insert(*dep_id, current);
                        queue.push_back(*dep_id);

                        if *dep_id == to {
                            // Reconstruct path
                            let mut path = vec![to];
                            let mut curr = to;
                            while let Some(&p) = parent.get(&curr) {
                                path.push(p);
                                curr = p;
                            }
                            path.reverse();
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// Get all entry points.
    #[inline]
    pub fn entries(&self) -> &[FileId] {
        &self.entries
    }

    /// Get the number of nodes in the graph.
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the graph is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &ModuleNode> {
        self.nodes.values()
    }

    /// Get component usage edges (which components use which).
    pub fn component_usage(&self) -> impl Iterator<Item = (FileId, FileId)> + '_ {
        self.nodes.values().flat_map(|node| {
            node.imports
                .iter()
                .filter(|(_, edge)| *edge == DependencyEdge::ComponentUsage)
                .map(move |(dep_id, _)| (node.file_id, *dep_id))
        })
    }

    /// Get all provide/inject relationships.
    pub fn provide_inject_edges(&self) -> impl Iterator<Item = (FileId, FileId)> + '_ {
        self.nodes.values().flat_map(|node| {
            node.imports
                .iter()
                .filter(|(_, edge)| *edge == DependencyEdge::ProvideInject)
                .map(move |(dep_id, _)| (node.file_id, *dep_id))
        })
    }

    /// Topological sort of the graph (for analysis ordering).
    pub fn topological_sort(&self) -> Option<Vec<FileId>> {
        let mut in_degree: FxHashMap<FileId, usize> = FxHashMap::default();
        let mut result = Vec::with_capacity(self.nodes.len());
        let mut queue = std::collections::VecDeque::new();

        // Initialize in-degrees
        for (&id, node) in &self.nodes {
            in_degree.entry(id).or_insert(0);
            for (dep_id, _) in &node.imports {
                *in_degree.entry(*dep_id).or_insert(0) += 1;
            }
        }

        // Find nodes with no incoming edges
        for (&id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(id);
            }
        }

        while let Some(id) = queue.pop_front() {
            result.push(id);

            if let Some(node) = self.nodes.get(&id) {
                for (dep_id, _) in &node.imports {
                    if let Some(degree) = in_degree.get_mut(dep_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(*dep_id);
                        }
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None // Graph has cycles
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_basic() {
        let mut graph = DependencyGraph::new();

        let id1 = FileId::new(0);
        let id2 = FileId::new(1);

        graph.add_node(ModuleNode::new(id1, "Parent.vue"));
        graph.add_node(ModuleNode::new(id2, "Child.vue"));

        graph.add_edge(id1, id2, DependencyEdge::ComponentUsage);

        assert!(graph.has_path(id1, id2));
        assert!(!graph.has_path(id2, id1));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        let id1 = FileId::new(0);
        let id2 = FileId::new(1);
        let id3 = FileId::new(2);

        graph.add_node(ModuleNode::new(id1, "A.vue"));
        graph.add_node(ModuleNode::new(id2, "B.vue"));
        graph.add_node(ModuleNode::new(id3, "C.vue"));

        graph.add_edge(id1, id2, DependencyEdge::Import);
        graph.add_edge(id2, id3, DependencyEdge::Import);
        graph.add_edge(id3, id1, DependencyEdge::Import); // Cycle!

        graph.detect_circular_dependencies();

        assert!(!graph.circular_dependencies().is_empty());
    }
}
