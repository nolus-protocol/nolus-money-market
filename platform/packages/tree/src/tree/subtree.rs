use std::ops::Range;

use crate::node::{NodeIndex, NodeRef};

use super::{FindBy, Tree, TreeIter};

#[derive(Copy, Clone)]
#[cfg_attr(any(debug_assertions, test), derive(Debug))]
pub struct Subtree<'r, T> {
    tree: &'r Tree<T>,
    subtree_root_index: NodeIndex,
    length: NodeIndex,
}

impl<'r, T> Subtree<'r, T> {
    pub fn from_tree(tree: &'r Tree<T>) -> Self {
        Self {
            tree,
            subtree_root_index: Tree::<T>::ROOT_INDEX,
            length: tree.node_index_len(),
        }
    }

    pub fn from_node(node: NodeRef<'r, T>) -> Self {
        Self {
            tree: node.tree(),
            subtree_root_index: node.this_index(),
            length: {
                // Starting from one to account for subtree's root node.
                let mut length: NodeIndex = 1;

                // Starting from next node so this will only iterate though
                // child nodes.
                for _ in node.tree().nodes[usize::from(node.this_index() + 1)..]
                    .iter()
                    .take_while(|tree_node| tree_node.parent_index() >= node.this_index())
                {
                    length += 1;
                }

                length
            },
        }
    }

    pub fn into_subtree_root(self) -> NodeRef<'r, T> {
        NodeRef::with_index(self.tree, self.subtree_root_index)
    }

    pub fn iter(&self) -> TreeIter<'_, T> {
        TreeIter {
            tree: self.tree,
            range: self.subtree_range(),
        }
    }

    fn subtree_range(&self) -> Range<NodeIndex> {
        self.subtree_root_index..self.subtree_root_index + self.length
    }
}

impl<'r, T> From<NodeRef<'r, T>> for Subtree<'r, T> {
    fn from(value: NodeRef<'r, T>) -> Self {
        Self::from_node(value)
    }
}

impl<'r, T> From<Subtree<'r, T>> for NodeRef<'r, T> {
    fn from(value: Subtree<'r, T>) -> Self {
        value.into_subtree_root()
    }
}

impl<T> FindBy for Subtree<'_, T> {
    type Item = T;

    fn find_by<F>(&self, f: F) -> Option<NodeRef<'_, T>>
    where
        F: FnMut(&'_ T) -> bool,
    {
        self.tree.find_by_within(self.subtree_range(), f)
    }
}
