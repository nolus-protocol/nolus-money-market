use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::node::{Node, NodeIndex, NodeRef};

pub use self::{
    human_readable::{HrtNode, HumanReadableTree},
    subtree::Subtree,
};

mod human_readable;
mod subtree;

pub trait FindBy {
    type Item;

    fn find_by<F>(&self, f: F) -> Option<NodeRef<'_, Self::Item>>
    where
        F: FnMut(&'_ Self::Item) -> bool;
}

type Nodes<T> = Vec<Node<T>>;

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
#[cfg_attr(not(debug_assertions), derive(Deserialize))]
#[repr(transparent)]
#[serde(transparent, rename_all = "snake_case")]
pub struct Tree<T> {
    nodes: Nodes<T>,
}

impl<T> Tree<T> {
    const ROOT_PARENT: NodeIndex = 0;

    const ROOT_INDEX: NodeIndex = 0;

    pub fn root(&self) -> NodeRef<'_, T> {
        debug_assert!(!self.nodes.is_empty());

        NodeRef::with_index(self, Self::ROOT_INDEX)
    }

    pub fn into_human_readable(self) -> HumanReadableTree<T> {
        HumanReadableTree::from_tree(self)
    }

    pub fn as_subtree(&self) -> Subtree<'_, T> {
        Subtree::from_tree(self)
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn iter(&self) -> TreeIter<'_, T> {
        TreeIter {
            tree: self,
            range: self.vector_range(),
        }
    }

    pub fn map<F, R>(self, mut f: F) -> Tree<R>
    where
        F: FnMut(T) -> R,
    {
        Tree {
            nodes: self
                .nodes
                .into_iter()
                .map(|node| node.map(&mut f))
                .collect(),
        }
    }

    pub(crate) fn node_index_len(&self) -> NodeIndex {
        self.nodes
            .len()
            .try_into()
            .expect("Tree has more elements than allowed!")
    }

    pub(crate) fn find_by_within<F>(
        &self,
        range: Range<NodeIndex>,
        mut f: F,
    ) -> Option<NodeRef<'_, T>>
    where
        F: FnMut(&'_ T) -> bool,
    {
        // `Iterator::enumerate` is not used to avoid problems with Rust
        // emitting floating-point instructions on WASM on some cases where
        // `usize` is used.
        //
        // Using zero as a literal instead of using `ROOT_INDEX` as it
        // represents the beginning of the vector and not necessarily the
        // tree's root.
        let mut index: NodeIndex = range.start;

        self.nodes[usize::from(range.start)..][..usize::from(range.end)]
            .iter()
            .find_map(|raw_node| {
                let result = f(raw_node.value()).then(|| NodeRef::with_index(self, index));

                index += 1;

                result
            })
    }

    pub(crate) fn is_root(&self, index: NodeIndex) -> bool {
        index == Self::ROOT_INDEX
    }

    pub(crate) fn node(&self, index: NodeIndex) -> &Node<T> {
        &self.nodes[usize::from(index)]
    }

    fn vector_range(&self) -> Range<NodeIndex> {
        0..self.node_index_len()
    }
}

#[cfg(debug_assertions)]
impl<'de, T> Deserialize<'de> for Tree<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let nodes: Nodes<T> = Nodes::deserialize(deserializer)?;

        assert!(!nodes.is_empty());
        assert_eq!(
            nodes[usize::from(Self::ROOT_INDEX)].parent_index(),
            Self::ROOT_PARENT
        );

        for (index, node) in nodes.iter().enumerate().skip(1) {
            assert!(usize::from(node.parent_index()) < index);
        }

        Ok(Self { nodes })
    }
}

/// Iterator over (sub)tree's nodes that returns node references in the starting
/// from the root going down to the bottom left and from there returning to the
/// parent node's sibling's child nodes.
///
/// Visualized, the order is represented by the numbers in the place of the
/// nodes, it looks like this:
/// ```text
///     1
///    /|\
///   2 5 7
///  /| | |\
/// 3 4 6 8 9
/// ```
pub struct TreeIter<'r, T> {
    tree: &'r Tree<T>,
    range: Range<NodeIndex>,
}

impl<'r, T> Iterator for TreeIter<'r, T> {
    type Item = NodeRef<'r, T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.range
            .next()
            .map(|this| NodeRef::with_index(self.tree, this))
    }
}

impl<T> DoubleEndedIterator for TreeIter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range
            .next_back()
            .map(|this| NodeRef::with_index(self.tree, this))
    }
}

impl<T> FindBy for Tree<T> {
    type Item = T;

    fn find_by<F>(&self, f: F) -> Option<NodeRef<'_, T>>
    where
        F: FnMut(&'_ T) -> bool,
    {
        self.find_by_within(self.vector_range(), f)
    }
}
