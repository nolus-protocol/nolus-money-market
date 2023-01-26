use serde::{Deserialize, Serialize};

use crate::tree::{Subtree, Tree};

pub(crate) type NodeIndex = u16;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Node<T> {
    parent: NodeIndex,
    value: T,
}

impl<T> Node<T> {
    pub(crate) fn new(parent: NodeIndex, value: T) -> Self {
        Self { parent, value }
    }

    #[inline]
    pub(crate) fn parent_index(&self) -> NodeIndex {
        self.parent
    }

    #[inline]
    pub(crate) fn value(&self) -> &T {
        &self.value
    }

    #[inline]
    pub(crate) fn into_value(self) -> T {
        self.value
    }

    #[inline]
    pub(crate) fn map<F, R>(self, f: F) -> Node<R>
    where
        F: FnOnce(T) -> R,
    {
        Node {
            parent: self.parent,
            value: f(self.value),
        }
    }
}

#[derive(Eq, PartialEq)]
#[cfg_attr(any(debug_assertions, test), derive(Debug))]
pub struct NodeRef<'r, T> {
    tree: &'r Tree<T>,
    this: NodeIndex,
}

impl<'r, T> NodeRef<'r, T> {
    #[inline]
    pub fn value(&self) -> &T {
        &self.tree.node(self.this).value
    }

    pub fn parent(&self) -> Option<Self> {
        if self.tree.is_root(self.this) {
            None
        } else {
            let this = self.tree.node(self.this);

            Some(NodeRef {
                tree: self.tree,
                this: this.parent_index(),
            })
        }
    }

    #[inline]
    pub fn parents_iter(&self) -> ParentsIter<'r, T> {
        ParentsIter { node: *self }
    }

    #[inline]
    /// This exists as a functional approach to converting node reference into
    /// a subtree.
    pub fn to_subtree(&self) -> Subtree<'r, T> {
        Subtree::from_node(*self)
    }

    #[inline]
    pub(crate) fn tree(&self) -> &'r Tree<T> {
        self.tree
    }

    #[inline]
    pub(crate) fn this_index(&self) -> NodeIndex {
        self.this
    }

    #[inline]
    pub(crate) const fn with_index(tree: &'r Tree<T>, this: NodeIndex) -> Self {
        Self { tree, this }
    }
}

impl<'r, T> Clone for NodeRef<'r, T> {
    fn clone(&self) -> Self {
        Self { ..*self }
    }
}

impl<'r, T> Copy for NodeRef<'r, T> {}

pub struct ParentsIter<'r, T> {
    node: NodeRef<'r, T>,
}

impl<'r, T> Iterator for ParentsIter<'r, T> {
    type Item = NodeRef<'r, T>;

    fn next(&mut self) -> Option<Self::Item> {
        let parent = self.node.parent()?;

        self.node = parent;

        Some(parent)
    }
}
