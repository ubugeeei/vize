//! Differential rendering algorithm.

#![allow(dead_code)]

use super::node::{NodeId, RenderNode};
use super::tree::RenderTree;
use smallvec::SmallVec;

/// Types of changes detected during diffing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOp {
    /// Node was added
    Insert(NodeId),
    /// Node was removed
    Remove(NodeId),
    /// Node content changed
    Update(NodeId),
    /// Node was moved to new position
    Move(NodeId, NodeId), // (node_id, new_parent_id)
    /// Node style changed (needs re-layout)
    Restyle(NodeId),
}

/// Diff result containing all operations needed.
#[derive(Debug, Default)]
pub struct DiffResult {
    /// Operations to apply
    pub ops: SmallVec<[DiffOp; 8]>,
    /// Whether layout needs to be recalculated
    pub needs_layout: bool,
}

impl DiffResult {
    /// Create a new empty diff result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an operation.
    pub fn push(&mut self, op: DiffOp) {
        match &op {
            DiffOp::Insert(_) | DiffOp::Remove(_) | DiffOp::Restyle(_) => {
                self.needs_layout = true;
            }
            _ => {}
        }
        self.ops.push(op);
    }

    /// Check if there are any changes.
    pub fn has_changes(&self) -> bool {
        !self.ops.is_empty()
    }

    /// Get number of operations.
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// Diff engine for comparing render trees.
pub struct DiffEngine;

impl DiffEngine {
    /// Compare two trees and return operations to transform old to new.
    pub fn diff(old: &RenderTree, new: &RenderTree) -> DiffResult {
        let mut result = DiffResult::new();

        // Get roots
        let old_root = old.root();
        let new_root = new.root();

        match (old_root, new_root) {
            (None, None) => {
                // Both empty, no changes
            }
            (Some(_), None) => {
                // New tree is empty, remove everything
                if let Some(root_id) = old_root {
                    Self::collect_removes(old, root_id, &mut result);
                }
            }
            (None, Some(root_id)) => {
                // Old tree was empty, insert everything
                Self::collect_inserts(new, root_id, &mut result);
            }
            (Some(old_root_id), Some(new_root_id)) => {
                // Both have roots, diff them
                Self::diff_nodes(old, new, old_root_id, new_root_id, &mut result);
            }
        }

        result
    }

    /// Recursively diff two nodes.
    fn diff_nodes(
        old: &RenderTree,
        new: &RenderTree,
        old_id: NodeId,
        new_id: NodeId,
        result: &mut DiffResult,
    ) {
        let old_node = old.get(old_id);
        let new_node = new.get(new_id);

        match (old_node, new_node) {
            (Some(old_n), Some(new_n)) => {
                // Check if node content changed
                if Self::content_changed(old_n, new_n) {
                    result.push(DiffOp::Update(new_id));
                }

                // Check if style changed
                if Self::style_changed(old_n, new_n) {
                    result.push(DiffOp::Restyle(new_id));
                }

                // Diff children
                Self::diff_children(old, new, old_n, new_n, result);
            }
            (None, Some(_)) => {
                // Node was added
                Self::collect_inserts(new, new_id, result);
            }
            (Some(_), None) => {
                // Node was removed
                Self::collect_removes(old, old_id, result);
            }
            (None, None) => {
                // Both don't exist, nothing to do
            }
        }
    }

    /// Diff children of two nodes.
    fn diff_children(
        old: &RenderTree,
        new: &RenderTree,
        old_node: &RenderNode,
        new_node: &RenderNode,
        result: &mut DiffResult,
    ) {
        let old_children = &old_node.children;
        let new_children = &new_node.children;

        // Simple diff algorithm: match by index
        // A more sophisticated algorithm would use keys
        let max_len = old_children.len().max(new_children.len());

        for i in 0..max_len {
            let old_child = old_children.get(i).copied();
            let new_child = new_children.get(i).copied();

            match (old_child, new_child) {
                (Some(old_id), Some(new_id)) => {
                    Self::diff_nodes(old, new, old_id, new_id, result);
                }
                (None, Some(new_id)) => {
                    Self::collect_inserts(new, new_id, result);
                }
                (Some(old_id), None) => {
                    Self::collect_removes(old, old_id, result);
                }
                (None, None) => unreachable!(),
            }
        }
    }

    /// Check if node content changed.
    fn content_changed(old: &RenderNode, new: &RenderNode) -> bool {
        use super::node::NodeKind;

        match (&old.kind, &new.kind) {
            (NodeKind::Box, NodeKind::Box) => false,
            (NodeKind::Text(old_text), NodeKind::Text(new_text)) => {
                old_text.text != new_text.text || old_text.wrap != new_text.wrap
            }
            (NodeKind::Input(old_input), NodeKind::Input(new_input)) => {
                old_input.value != new_input.value
                    || old_input.cursor != new_input.cursor
                    || old_input.focused != new_input.focused
            }
            (NodeKind::Raw(old_raw), NodeKind::Raw(new_raw)) => old_raw.lines != new_raw.lines,
            // Different types always count as changed
            _ => true,
        }
    }

    /// Check if node style changed.
    fn style_changed(old: &RenderNode, new: &RenderNode) -> bool {
        // Compare appearance
        old.appearance.fg != new.appearance.fg
            || old.appearance.bg != new.appearance.bg
            || old.appearance.bold != new.appearance.bold
            || old.appearance.italic != new.appearance.italic
            || old.appearance.border != new.appearance.border
        // Note: FlexStyle changes would need deep comparison
    }

    /// Collect insert operations for a subtree.
    fn collect_inserts(tree: &RenderTree, id: NodeId, result: &mut DiffResult) {
        result.push(DiffOp::Insert(id));
        if let Some(node) = tree.get(id) {
            for &child_id in &node.children {
                Self::collect_inserts(tree, child_id, result);
            }
        }
    }

    /// Collect remove operations for a subtree.
    fn collect_removes(tree: &RenderTree, id: NodeId, result: &mut DiffResult) {
        if let Some(node) = tree.get(id) {
            for &child_id in &node.children {
                Self::collect_removes(tree, child_id, result);
            }
        }
        result.push(DiffOp::Remove(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::node::NodeKind;

    #[test]
    fn test_diff_empty_trees() {
        let old = RenderTree::new();
        let new = RenderTree::new();
        let result = DiffEngine::diff(&old, &new);
        assert!(result.is_empty());
    }

    #[test]
    fn test_diff_insert_root() {
        let old = RenderTree::new();
        let mut new = RenderTree::new();
        let id = new.next_id();
        new.insert_root(RenderNode::new(id, NodeKind::Box));

        let result = DiffEngine::diff(&old, &new);
        assert_eq!(result.len(), 1);
        assert!(matches!(result.ops[0], DiffOp::Insert(_)));
        assert!(result.needs_layout);
    }

    #[test]
    fn test_diff_remove_root() {
        let mut old = RenderTree::new();
        let id = old.next_id();
        old.insert_root(RenderNode::new(id, NodeKind::Box));
        let new = RenderTree::new();

        let result = DiffEngine::diff(&old, &new);
        assert_eq!(result.len(), 1);
        assert!(matches!(result.ops[0], DiffOp::Remove(_)));
    }

    #[test]
    fn test_diff_update_text() {
        let mut old = RenderTree::new();
        let id1 = old.next_id();
        old.insert_root(RenderNode::text_node(id1, "Hello"));

        let mut new = RenderTree::new();
        let id2 = new.next_id();
        new.insert_root(RenderNode::text_node(id2, "World"));

        let result = DiffEngine::diff(&old, &new);
        assert!(result.has_changes());
        assert!(result.ops.iter().any(|op| matches!(op, DiffOp::Update(_))));
    }
}
