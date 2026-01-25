//! Render tree management.

use rustc_hash::FxHashMap;

use super::node::{NodeId, RenderNode};
use crate::layout::{FlexStyle, LayoutEngine};

/// A tree of render nodes.
pub struct RenderTree {
    /// All nodes by ID
    nodes: FxHashMap<NodeId, RenderNode>,
    /// Root node ID
    root: Option<NodeId>,
    /// Layout engine
    layout: LayoutEngine,
    /// Next available node ID
    next_id: NodeId,
    /// Mapping from our IDs to layout IDs
    layout_ids: FxHashMap<NodeId, u64>,
}

impl RenderTree {
    /// Create a new render tree.
    pub fn new() -> Self {
        Self {
            nodes: FxHashMap::default(),
            root: None,
            layout: LayoutEngine::new(),
            next_id: 0,
            layout_ids: FxHashMap::default(),
        }
    }

    /// Allocate a new node ID.
    pub fn next_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Insert a node into the tree.
    pub fn insert(&mut self, node: RenderNode) -> NodeId {
        let id = node.id;
        let layout_id = self.layout.new_node(&node.style);
        self.layout_ids.insert(id, layout_id);
        self.nodes.insert(id, node);
        id
    }

    /// Insert a node as root.
    pub fn insert_root(&mut self, node: RenderNode) -> NodeId {
        let id = self.insert(node);
        self.set_root(id);
        id
    }

    /// Set the root node.
    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
        if let Some(&layout_id) = self.layout_ids.get(&id) {
            self.layout.set_root(layout_id);
        }
    }

    /// Get the root node ID.
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Get a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&RenderNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable node by ID.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut RenderNode> {
        self.nodes.get_mut(&id)
    }

    /// Add a child to a parent node.
    pub fn add_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        // Update render tree
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.add_child(child_id);
        }

        // Update layout tree
        if let (Some(&parent_layout), Some(&child_layout)) = (
            self.layout_ids.get(&parent_id),
            self.layout_ids.get(&child_id),
        ) {
            self.layout.add_child(parent_layout, child_layout);
        }
    }

    /// Remove a child from a parent node.
    pub fn remove_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        // Update render tree
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.remove_child(child_id);
        }

        // Update layout tree
        if let (Some(&parent_layout), Some(&child_layout)) = (
            self.layout_ids.get(&parent_id),
            self.layout_ids.get(&child_id),
        ) {
            self.layout.remove_child(parent_layout, child_layout);
        }
    }

    /// Remove a node and all its descendants.
    pub fn remove(&mut self, id: NodeId) {
        // Collect all descendant IDs first
        let descendants = self.collect_descendants(id);

        // Remove all nodes
        for node_id in descendants {
            if let Some(layout_id) = self.layout_ids.remove(&node_id) {
                self.layout.remove(layout_id);
            }
            self.nodes.remove(&node_id);
        }
    }

    /// Collect a node and all its descendants.
    fn collect_descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut result = vec![id];
        let mut stack = vec![id];

        while let Some(current) = stack.pop() {
            if let Some(node) = self.nodes.get(&current) {
                for &child_id in &node.children {
                    result.push(child_id);
                    stack.push(child_id);
                }
            }
        }

        result
    }

    /// Update a node's style.
    pub fn set_style(&mut self, id: NodeId, style: FlexStyle) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.style = style.clone();
            node.mark_dirty();
        }
        if let Some(&layout_id) = self.layout_ids.get(&id) {
            self.layout.set_style(layout_id, &style);
        }
    }

    /// Compute layout for all nodes.
    pub fn compute_layout(&mut self, width: u16, height: u16) {
        self.layout.compute(width as f32, height as f32);

        // Update layout in all nodes
        for (&id, &layout_id) in &self.layout_ids {
            if let Some(rect) = self.layout.layout(layout_id) {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.layout = Some(rect);
                }
            }
        }
    }

    /// Get all dirty nodes.
    pub fn dirty_nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes
            .iter()
            .filter(|(_, node)| node.dirty)
            .map(|(&id, _)| id)
    }

    /// Mark all nodes as clean.
    pub fn mark_all_clean(&mut self) {
        for node in self.nodes.values_mut() {
            node.mark_clean();
        }
    }

    /// Get node count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Clear the entire tree.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.layout.clear();
        self.layout_ids.clear();
        self.root = None;
        self.next_id = 0;
    }

    /// Iterate over all nodes.
    pub fn iter(&self) -> impl Iterator<Item = (&NodeId, &RenderNode)> {
        self.nodes.iter()
    }

    /// Iterate over all nodes mutably.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&NodeId, &mut RenderNode)> {
        self.nodes.iter_mut()
    }

    /// Walk the tree depth-first, pre-order.
    pub fn walk_preorder(&self, start: NodeId) -> TreeWalker<'_> {
        TreeWalker::new(self, start)
    }
}

impl Default for RenderTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator for tree traversal.
pub struct TreeWalker<'a> {
    tree: &'a RenderTree,
    stack: Vec<NodeId>,
}

impl<'a> TreeWalker<'a> {
    fn new(tree: &'a RenderTree, start: NodeId) -> Self {
        Self {
            tree,
            stack: vec![start],
        }
    }
}

impl<'a> Iterator for TreeWalker<'a> {
    type Item = &'a RenderNode;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.stack.pop() {
            if let Some(node) = self.tree.get(id) {
                // Push children in reverse order so they're processed left-to-right
                for &child_id in node.children.iter().rev() {
                    self.stack.push(child_id);
                }
                return Some(node);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::super::node::NodeKind;
    use super::*;

    #[test]
    fn test_tree_new() {
        let tree = RenderTree::new();
        assert_eq!(tree.node_count(), 0);
        assert!(tree.root().is_none());
    }

    #[test]
    fn test_insert_root() {
        let mut tree = RenderTree::new();
        let id = tree.next_id();
        let node = RenderNode::new(id, NodeKind::Box);
        let root_id = tree.insert_root(node);
        assert_eq!(tree.root(), Some(root_id));
        assert_eq!(tree.node_count(), 1);
    }

    #[test]
    fn test_add_child() {
        let mut tree = RenderTree::new();

        let parent_id = tree.next_id();
        let parent = RenderNode::new(parent_id, NodeKind::Box);
        tree.insert_root(parent);

        let child_id = tree.next_id();
        let child = RenderNode::new(child_id, NodeKind::Box);
        tree.insert(child);

        tree.add_child(parent_id, child_id);

        let parent = tree.get(parent_id).unwrap();
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0], child_id);
    }

    #[test]
    fn test_tree_walk() {
        let mut tree = RenderTree::new();

        let root_id = tree.next_id();
        tree.insert_root(RenderNode::new(root_id, NodeKind::Box));

        let child1_id = tree.next_id();
        tree.insert(RenderNode::new(child1_id, NodeKind::Box));
        tree.add_child(root_id, child1_id);

        let child2_id = tree.next_id();
        tree.insert(RenderNode::new(child2_id, NodeKind::Box));
        tree.add_child(root_id, child2_id);

        let ids: Vec<_> = tree.walk_preorder(root_id).map(|n| n.id).collect();
        assert_eq!(ids, vec![root_id, child1_id, child2_id]);
    }
}
