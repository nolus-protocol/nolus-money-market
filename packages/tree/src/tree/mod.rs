use serde::{Deserialize, Serialize};

use crate::node::{Node, NodeIndex, NodeRef};

pub use self::human_readable::HumanReadableTree;

mod human_readable;

type Nodes<T> = Vec<Node<T>>;

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
#[repr(transparent)]
#[serde(rename_all = "snake_case")]
pub struct Tree<T> {
    nodes: Nodes<T>,
}

impl<T> Tree<T> {
    const ROOT_INDEX: NodeIndex = 0;

    pub fn root(&self) -> Option<NodeRef<T>> {
        (!self.nodes.is_empty()).then(|| NodeRef::with_index(self, Self::ROOT_INDEX))
    }

    pub fn find_by<F>(&self, mut f: F) -> Option<NodeRef<T>>
    where
        F: FnMut(&T) -> bool,
    {
        // `Iterator::enumerate` is not used to avoid problems with Rust emitting floating-point
        // instructions on WASM on some cases where `usize` is used.
        let mut index: NodeIndex = 0;

        self.nodes.iter().find_map(|raw_node| {
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
}
