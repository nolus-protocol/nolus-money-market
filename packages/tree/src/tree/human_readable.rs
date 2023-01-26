use serde::{de::Error, Deserialize, Deserializer, Serialize};

use crate::{
    node::{Node, NodeIndex},
    tree::{Nodes, Tree},
};

#[derive(Serialize, Deserialize)]
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
                .flatten(Tree::<T>::ROOT_INDEX, Tree::<T>::ROOT_INDEX),
        }
    }
}

#[derive(Serialize, Deserialize)]
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
