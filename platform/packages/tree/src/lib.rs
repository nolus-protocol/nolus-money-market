use std::{
    collections::btree_map::{BTreeMap, Entry},
    num::NonZeroUsize,
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "schema")]
use sdk::schemars::{self, JsonSchema};

use self::error::Error;

pub mod error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum HumanReadableTree<T> {
    Leaf {
        value: T,
    },
    Branch {
        value: T,
        children: Box<[HumanReadableTree<T>]>,
    },
}

impl<T> HumanReadableTree<T> {
    fn new_branch(value: T, children: Box<[HumanReadableTree<T>]>) -> HumanReadableTree<T> {
        Self::Branch { value, children }
    }

    fn new_leaf(value: T) -> HumanReadableTree<T> {
        Self::Leaf { value }
    }

    fn prepare_children(
        child_nodes: &mut BTreeMap<usize, Vec<Self>>,
        indexes_mapping: &usize,
    ) -> Option<Box<[Self]>> {
        child_nodes.remove(indexes_mapping).map(|mut children| {
            children.reverse();

            children.into_boxed_slice()
        })
    }
}

impl<T> From<Tree<T>> for HumanReadableTree<T> {
    fn from(
        Tree {
            root,
            parent_indexes,
            branches_and_leafs,
        }: Tree<T>,
    ) -> Self {
        let mut child_nodes: BTreeMap<usize, Vec<HumanReadableTree<T>>> = BTreeMap::new();

        let mut indexes_mapping: Vec<usize> = (1..1 + branches_and_leafs.len()).collect();

        let mut parent_indexes = parent_indexes.into_vec();

        let mut branches_and_leafs = branches_and_leafs.into_vec();

        while let Some(index) = parent_indexes
            .iter()
            .copied()
            .enumerate()
            .max_by_key(|&(_, parent_index)| parent_index)
            .map(|(index, _)| index)
        {
            let node = {
                let value = branches_and_leafs.remove(index);

                if let Some(children) =
                    Self::prepare_children(&mut child_nodes, &indexes_mapping.remove(index))
                {
                    Self::new_branch(value, children)
                } else {
                    Self::new_leaf(value)
                }
            };

            match child_nodes.entry(parent_indexes.remove(index).into()) {
                Entry::Vacant(entry) => _ = entry.insert(vec![node]),
                Entry::Occupied(entry) => entry.into_mut().push(node),
            }
        }

        let tree = if let Some(children) = Self::prepare_children(&mut child_nodes, &0) {
            Self::new_branch(root, children)
        } else {
            Self::new_leaf(root)
        };

        debug_assert!(parent_indexes.is_empty());

        debug_assert!(branches_and_leafs.is_empty());

        debug_assert!(indexes_mapping.is_empty());

        debug_assert!(child_nodes.is_empty());

        tree
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tree<T>
where
    ParentIndex: Into<usize>,
{
    root: T,
    parent_indexes: Box<[ParentIndex]>,
    branches_and_leafs: Box<[T]>,
}

impl<T> Tree<T> {
    #[inline]
    pub fn root(&self) -> Node<'_, T> {
        Node {
            tree: self,
            index: None,
        }
    }

    #[inline]
    pub fn direct_children(&self) -> impl DoubleEndedIterator<Item = Node<'_, T>> {
        direct_children(self, 0, self.parent_indexes.len())
    }

    #[inline]
    pub fn depth_first_values_iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        Some(&self.root)
            .into_iter()
            .chain(self.branches_and_leafs.iter())
    }

    #[inline]
    pub fn depth_first_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Node<'_, T>> {
        Some(self.root())
            .into_iter()
            .chain((0..self.branches_and_leafs.len()).map(move |index| Node {
                tree: self,
                index: NonZeroUsize::new(index + 1),
            }))
    }

    #[cfg(debug_assertions)]
    pub fn check_tree(&self) {
        debug_assert_eq!(self.parent_indexes.len(), self.branches_and_leafs.len());

        let mut current_subtree_root = 0;

        for (index, parent_index) in self
            .parent_indexes
            .iter()
            .copied()
            .map(usize::from)
            .enumerate()
        {
            debug_assert!(
                parent_index <= index,
                "Nodes can only belong to nodes on the left-side of them! Expected: <={index}, got: {parent_index}"
            );

            if current_subtree_root < parent_index {
                debug_assert_eq!(
                    parent_index,
                    index,
                    "Nodes can only be defined directly under the node they belong to! Expected: {index}, got: {parent_index}"
                );
            }

            current_subtree_root = parent_index;
        }
    }

    fn fill_in_child_nodes(
        children: Box<[HumanReadableTree<T>]>,
    ) -> Result<(Box<[ParentIndex]>, Box<[T]>), Error> {
        let mut parent_indexes = Vec::new();

        let mut branches_and_leafs = Vec::new();

        let branches_and_leafs_index: ParentIndex = 0;

        let mut parent_indexes_stack = Vec::new();

        parent_indexes_stack.push((branches_and_leafs_index, children.len()));

        let mut nodes_stack = children.into_vec();

        nodes_stack.reverse();

        Self::process_children(
            &mut parent_indexes,
            &mut branches_and_leafs,
            branches_and_leafs_index,
            parent_indexes_stack,
            nodes_stack,
        )?;

        Ok((
            parent_indexes.into_boxed_slice(),
            branches_and_leafs.into_boxed_slice(),
        ))
    }

    fn process_children(
        parent_indexes: &mut Vec<ParentIndex>,
        branches_and_leafs: &mut Vec<T>,
        mut branches_and_leafs_index: ParentIndex,
        mut parent_indexes_stack: Vec<(ParentIndex, usize)>,
        mut nodes_stack: Vec<HumanReadableTree<T>>,
    ) -> Result<(), Error> {
        loop {
            let Some(&mut (parent_index, ref mut child_nodes_left)) =
                parent_indexes_stack.last_mut()
            else {
                break;
            };

            let Some(node) = nodes_stack.pop() else {
                unreachable!()
            };

            parent_indexes.push(parent_index);

            {
                let child_nodes_left_value = *child_nodes_left;

                if child_nodes_left_value == 1 {
                    parent_indexes_stack.pop();
                } else {
                    *child_nodes_left = child_nodes_left_value - 1;
                }
            }

            branches_and_leafs_index = branches_and_leafs_index
                .checked_add(1)
                .ok_or(Error::TreeTooBig)?;

            let value = match node {
                HumanReadableTree::Leaf { value } => value,
                HumanReadableTree::Branch { value, children } => {
                    if !children.is_empty() {
                        parent_indexes_stack.push((branches_and_leafs_index, children.len()));

                        nodes_stack.extend(children.into_vec().into_iter().rev());
                    }

                    value
                }
            };

            branches_and_leafs.push(value);
        }

        Ok(())
    }
}

impl<T> TryFrom<HumanReadableTree<T>> for Tree<T> {
    type Error = Error;

    fn try_from(value: HumanReadableTree<T>) -> Result<Self, Self::Error> {
        let tree = match value {
            HumanReadableTree::Leaf { value } => Self {
                root: value,
                parent_indexes: Box::default(),
                branches_and_leafs: Box::default(),
            },
            HumanReadableTree::Branch { value, children } => {
                let (parent_indexes, branches_and_leafs) = Self::fill_in_child_nodes(children)?;

                Self {
                    root: value,
                    parent_indexes: parent_indexes,
                    branches_and_leafs: branches_and_leafs,
                }
            }
        };

        #[cfg(debug_assertions)]
        tree.check_tree();

        Ok(tree)
    }
}

pub struct Node<'r, T> {
    tree: &'r Tree<T>,
    index: Option<NonZeroUsize>,
}

impl<'r, T> Node<'r, T> {
    #[inline]
    pub fn parent(&self) -> Option<Node<'r, T>> {
        self.index.map(|index| Self {
            index: NonZeroUsize::new(self.tree.parent_indexes[index.get() - 1].into()),
            ..*self
        })
    }

    #[inline]
    pub fn value(&self) -> &'r T {
        self.index.map_or_else(
            || &self.tree.root,
            |index| &self.tree.branches_and_leafs[index.get() - 1],
        )
    }

    #[inline]
    pub fn direct_children(&self) -> impl DoubleEndedIterator<Item = Node<'r, T>> + 'r {
        direct_children(
            self.tree,
            self.index.map_or(0, NonZeroUsize::get),
            self.subtree_end(),
        )
    }

    #[inline]
    pub fn depth_first_values_iter(&self) -> impl DoubleEndedIterator<Item = &'r T> {
        let (maybe_root, start_index) = self.index.map_or_else(
            || (Some(&self.tree.root), 0),
            |index| (None, index.get() - 1),
        );

        maybe_root
            .into_iter()
            .chain(&self.tree.branches_and_leafs[start_index..self.subtree_end()])
    }

    #[inline]
    pub fn depth_first_nodes_iter(&self) -> impl DoubleEndedIterator<Item = Node<'r, T>> + 'r {
        let (maybe_root, start_index) = self.index.map_or_else(
            || (Some(self.tree.root()), 0),
            |index| (None, index.get() - 1),
        );

        maybe_root
            .into_iter()
            .chain((start_index..self.subtree_end()).map({
                let tree = self.tree;

                move |index| Self {
                    tree,
                    index: NonZeroUsize::new(index + 1),
                }
            }))
    }

    fn subtree_end(&self) -> usize {
        self.index.map_or_else(
            || self.tree.parent_indexes.len(),
            |start_index| {
                let start_index = start_index.get() - 1;

                self.tree.parent_indexes[start_index..]
                    .iter()
                    .copied()
                    .map(usize::from)
                    .enumerate()
                    .skip_while(|&(_, parent_index)| parent_index <= start_index)
                    .find_map(|(node_index, parent_index)| {
                        (parent_index <= start_index).then_some(node_index)
                    })
                    .unwrap_or_else(|| self.tree.parent_indexes.len())
            },
        )
    }
}

fn direct_children<T>(
    tree: &Tree<T>,
    parent_index: usize,
    subtree_end: usize,
) -> impl DoubleEndedIterator<Item = Node<'_, T>> {
    tree.parent_indexes[parent_index..subtree_end]
        .iter()
        .copied()
        .enumerate()
        .filter_map(move |(index, node_parent_index)| {
            (usize::from(node_parent_index) == parent_index).then(|| Node {
                tree,
                index: NonZeroUsize::new(1 + parent_index + index),
            })
        })
}

type ParentIndex = u16;
