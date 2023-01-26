use std::{collections::BTreeMap, mem::replace};

use serde::{de::Error, Deserialize, Deserializer, Serialize};

use crate::{
    node::{Node, NodeIndex},
    tree::{Nodes, Tree},
};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct HumanReadableTree<T> {
    root: HRTNode<T>,
}

impl<T> HumanReadableTree<T> {
    pub fn into_tree(self) -> Tree<T> {
        Tree {
            nodes: self
                .root
                .flatten(Tree::<T>::ROOT_PARENT, Tree::<T>::ROOT_INDEX),
        }
    }

    pub fn from_tree(mut tree: Tree<T>) -> Self {
        Self {
            root: {
                let mut child_nodes: BTreeMap<NodeIndex, Vec<HRTNode<T>>> = BTreeMap::new();

                while let Some((start_index, node)) = Self::find_deepest(&mut tree) {
                    let parent_index: NodeIndex = node.parent_index();

                    let end_index: usize =
                        Self::find_last_child(&mut tree, start_index.into(), parent_index);

                    let children: Vec<HRTNode<T>> =
                        Self::drain_nodes(&mut tree, &mut child_nodes, start_index, end_index);

                    let result: Option<Vec<HRTNode<T>>> =
                        child_nodes.insert(parent_index, children);

                    debug_assert!(result.is_none());
                }

                let value: T = tree.nodes.remove(Tree::<T>::ROOT_INDEX.into()).into_value();

                let result: HRTNode<T> =
                    if let Some(children) = child_nodes.remove(&Tree::<T>::ROOT_PARENT) {
                        HRTNode::Branch { value, children }
                    } else {
                        HRTNode::Leaf { value }
                    };

                debug_assert!(child_nodes.is_empty());

                result
            },
        }
    }

    fn find_deepest(tree: &mut Tree<T>) -> Option<(NodeIndex, &Node<T>)> {
        tree.nodes[1..]
            .iter()
            .map({
                let mut index: NodeIndex = 0;

                move |node| {
                    let new_index: NodeIndex = index + 1;

                    (replace(&mut index, new_index), node)
                }
            })
            .max_by_key(|(_, node): &(NodeIndex, &Node<T>)| node.parent_index())
    }

    fn find_last_child(tree: &mut Tree<T>, start_index: usize, parent_index: NodeIndex) -> usize {
        start_index
            + tree.nodes[start_index..]
                .iter()
                .enumerate()
                .rfind(|(_, node): &(usize, &Node<T>)| node.parent_index() == parent_index)
                .expect("Subtree should contain at least the first found element!")
                .0
    }

    fn drain_nodes(
        tree: &mut Tree<T>,
        child_nodes: &mut BTreeMap<NodeIndex, Vec<HRTNode<T>>>,
        start_index: NodeIndex,
        end_index: usize,
    ) -> Vec<HRTNode<T>> {
        tree.nodes
            .drain(usize::from(start_index)..=end_index)
            .map({
                let mut index: NodeIndex = start_index;

                move |node: Node<T>| {
                    let value: T = node.into_value();

                    let children: Option<Vec<HRTNode<T>>> = child_nodes.remove(&index);

                    index += 1;

                    if let Some(children) = children {
                        HRTNode::Branch { value, children }
                    } else {
                        HRTNode::Leaf { value }
                    }
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case", deny_unknown_fields)]
enum HRTNode<T> {
    Leaf {
        value: T,
    },
    Branch {
        value: T,
        #[serde(deserialize_with = "node_children")]
        children: Vec<HRTNode<T>>,
    },
}

impl<T> HRTNode<T> {
    fn flatten(self, parent: NodeIndex, this: NodeIndex) -> Nodes<T> {
        match self {
            HRTNode::Leaf { value } => {
                vec![Node::new(parent, value)]
            }
            HRTNode::Branch { value, children } => {
                children
                    .into_iter()
                    .fold(vec![Node::new(parent, value)], |mut nodes, node| {
                        if let Self::Leaf { value } = node {
                            nodes.push(Node::new(this, value));
                        } else {
                            nodes.append(
                                &mut node.flatten(
                                    this,
                                    this + NodeIndex::try_from(nodes.len())
                                        .expect("Tree contains too many elements!"),
                                ),
                            );
                        }

                        nodes
                    })
            }
        }
    }
}

fn node_children<'de, D, T>(deserializer: D) -> Result<Vec<HRTNode<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let nodes: Vec<HRTNode<T>> = Deserialize::deserialize(deserializer)?;

    if nodes.is_empty() {
        return Err(Error::custom(
            r#"When "children" field is present, it has to contain at least one child."#,
        ));
    }

    Ok(nodes)
}
