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
        let mut index = 0;

        self.nodes.iter().find_map(|raw_node| {
            let result = f(raw_node.value()).then(|| Node::with_index(self, index));

            index += 1;

            result
        })
    }

    pub(crate) fn get(&self, index: u16) -> &RawNode<T> {
        &self.nodes[usize::from(index)]
    }
}
