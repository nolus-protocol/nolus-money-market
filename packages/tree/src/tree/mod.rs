use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::node::{Node, Raw as RawNode};

mod unchecked;

type NodesField<T> = Vec<RawNode<T>>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", try_from = "unchecked::Unchecked<T>")]
pub struct Tree<T> {
    nodes: NodesField<T>,
}

impl<T> Tree<T> {
    pub fn root(&self) -> Option<Node<T>> {
        (!self.nodes.is_empty()).then(|| Node::root(self))
    }

    pub fn find_by<F>(&self, mut f: F) -> Option<Node<T>>
    where
        F: FnMut(&T) -> bool,
    {
        self.nodes.iter().enumerate().find_map(|(index, raw_node)| {
            f(raw_node.value()).then(|| Node::with_index(self, index))
        })
    }

    pub(crate) fn get(&self, index: usize) -> &RawNode<T> {
        &self.nodes[index]
    }
}
