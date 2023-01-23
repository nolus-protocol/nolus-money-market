use serde::{Deserialize, Serialize};

use crate::tree::Tree;

pub(crate) type NodeIndex = u16;

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(crate) struct Node<T> {
    parent: u16,
    value: T,
}

impl<T> Node<T> {
    pub(crate) fn new(parent: NodeIndex, value: T) -> Self {
        Self { parent, value }
    }

    #[inline]
    pub(crate) fn parent(&self) -> NodeIndex {
        self.parent
    }

    #[inline]
    pub(crate) fn value(&self) -> &T {
        &self.value
    }
}

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
                this: this.parent,
            })
        }
    }

    #[inline]
    pub(crate) const fn with_index(tree: &'r Tree<T>, this: NodeIndex) -> Self {
        Self { tree, this }
    }
}
