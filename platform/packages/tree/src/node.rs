use std::mem::replace;

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

    pub(crate) fn parent_index(&self) -> NodeIndex {
        self.parent
    }

    pub(crate) fn value(&self) -> &T {
        &self.value
    }

    pub(crate) fn into_value(self) -> T {
        self.value
    }

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
    pub fn shorten_lifetime(&self) -> NodeRef<'_, T> {
        NodeRef { ..*self }
    }

    pub fn value(&self) -> &'r T {
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

    pub fn parents_iter(&self) -> ParentsIter<'r, T> {
        ParentsIter {
            inner: ParentsIterInner::Unresolved { node: *self },
        }
    }

    /// This exists as a functional approach to converting node reference into
    /// a subtree.
    pub fn to_subtree(&self) -> Subtree<'r, T> {
        Subtree::from_node(*self)
    }

    pub(crate) fn tree(&self) -> &'r Tree<T> {
        self.tree
    }

    pub(crate) fn this_index(&self) -> NodeIndex {
        self.this
    }

    pub(crate) const fn with_index(tree: &'r Tree<T>, this: NodeIndex) -> Self {
        Self { tree, this }
    }
}

impl<T> Clone for NodeRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for NodeRef<'_, T> {}

pub struct ParentsIter<'r, T> {
    inner: ParentsIterInner<'r, T>,
}

impl<'r, T> Iterator for ParentsIter<'r, T> {
    type Item = NodeRef<'r, T>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            ParentsIterInner::Unresolved { node } => {
                let parent = node.parent()?;

                *node = parent;

                Some(parent)
            }
            ParentsIterInner::Resolved { nodes } => nodes.next(),
        }
    }
}

impl<'r, T> DoubleEndedIterator for ParentsIter<'r, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            &mut ParentsIterInner::Unresolved { node } => {
                let mut node: Option<NodeRef<'r, T>> = node.parent();

                {
                    let mut nodes: Vec<NodeRef<'r, T>> = vec![];

                    if let Some(node) = &mut node {
                        while let Some(parent) = node.parent() {
                            nodes.push(replace(node, parent));
                        }
                    }

                    self.inner = ParentsIterInner::Resolved {
                        nodes: nodes.into_iter(),
                    };
                }

                node
            }
            ParentsIterInner::Resolved { nodes } => nodes.next_back(),
        }
    }
}

enum ParentsIterInner<'r, T> {
    Unresolved {
        node: NodeRef<'r, T>,
    },
    Resolved {
        nodes: std::vec::IntoIter<NodeRef<'r, T>>,
    },
}
