use serde::{Deserialize, Serialize};

use crate::node::{Node, NodeRef};

pub use self::human_readable::HumanReadableTree;

mod human_readable;
mod unchecked;

type Nodes<T> = Vec<Node<T>>;

#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
#[repr(transparent)]
#[serde(rename_all = "snake_case", try_from = "unchecked::Unchecked<T>")]
pub struct Tree<T> {
    nodes: Nodes<T>,
}

impl<T> Tree<T> {
    const ROOT_INDEX: u16 = 0;
    const ROOT_PARENT: u16 = Self::ROOT_INDEX;

    pub fn root(&self) -> Option<NodeRef<T>> {
        (!self.nodes.is_empty()).then(|| NodeRef::with_index(self, Self::ROOT_INDEX))
    }

    pub fn find_by<F>(&self, mut f: F) -> Option<NodeRef<T>>
    where
        F: FnMut(&T) -> bool,
    {
        let mut index = 0;

        self.nodes.iter().find_map(|raw_node| {
            let result = f(raw_node.value()).then(|| NodeRef::with_index(self, index));

            index += 1;

            result
        })
    }

    pub(crate) fn node(&self, index: u16) -> &Node<T> {
        &self.nodes[usize::from(index)]
    }
}
