use core::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use super::{NodesField, Tree};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct Unchecked<T> {
    nodes: NodesField<T>,
}

impl<T> TryFrom<Unchecked<T>> for Tree<T> {
    type Error = Error;

    fn try_from(value: Unchecked<T>) -> Result<Self, Self::Error> {
        if let Some(root) = value.nodes.first() {
            if root.parent() != 0 {
                return Err(Error::InvalidRoot);
            }
        }

        let mut index: u16 = 0;

        value
            .nodes
            .iter()
            .skip(1)
            .all(|raw_node| {
                index += 1;

                raw_node.parent() < index
            })
            .then_some(Tree { nodes: value.nodes })
            .ok_or(Error::MaybeCyclic)
    }
}

#[derive(Debug)]
pub(super) enum Error {
    InvalidRoot,
    MaybeCyclic,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("[Package = `tree`] Error: {}", match self {
            Error::InvalidRoot => "Deserialized tree has invalid root element!",
            Error::MaybeCyclic => "Deserialized tree contains forward indexes which could imply cyclic references!",
        }))
    }
}

impl std::error::Error for Error {}
