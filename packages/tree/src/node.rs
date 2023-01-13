use serde::{Deserialize, Serialize};

use crate::tree::Tree;

#[derive(Serialize, Deserialize)]
pub(crate) struct Raw<T> {
    parent: usize,
    value: T,
}

impl<T> Raw<T> {
    #[inline]
    pub(crate) fn parent(&self) -> usize {
        self.parent
    }

    #[inline]
    pub(crate) fn value(&self) -> &T {
        &self.value
    }
}

pub struct Node<'r, T> {
    tree: &'r Tree<T>,
    this: usize,
}

impl<'r, T> Node<'r, T> {
    #[inline]
    pub fn value(&self) -> &T {
        &self.tree.get(self.this).value
    }

    pub fn parent(&self) -> Option<Self> {
        let this = self.tree.get(self.this);

        (this.parent != self.this).then_some(Node {
            tree: self.tree,
            this: this.parent,
        })
    }

    #[inline]
    pub(crate) const fn root(tree: &'r Tree<T>) -> Self {
        Self { tree, this: 0 }
    }

    #[inline]
    pub(crate) const fn with_index(tree: &'r Tree<T>, this: usize) -> Self {
        Self { tree, this }
    }
}
