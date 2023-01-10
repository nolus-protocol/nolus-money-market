use alloc::{vec, vec::Vec};

use crate::node::Node;

use super::Tranversable;

pub struct NodeIter<'r, T> {
    parent_stack: Vec<(&'r Node<T>, Option<usize>)>,
}

impl<'r, T> NodeIter<'r, T> {
    pub(super) fn new(node: &'r Node<T>) -> Self {
        Self {
            parent_stack: vec![(node, None)],
        }
    }
}

impl<'r, T> NodeIter<'r, T> {
    pub fn as_traversable(&self) -> Option<Tranversable<'r, T>> {
        Some(Tranversable {
            parent_stack: Vec::from_iter(
                self.parent_stack
                    .iter()
                    .take(self.parent_stack.len().saturating_sub(1))
                    .map(|&(parent, _)| parent),
            ),
            current: self.parent_stack.last().map(|&(current, _)| current)?,
        })
    }
}

impl<'r, T> Iterator for NodeIter<'r, T> {
    type Item = &'r Node<T>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(loop {
            let &mut (parent, ref mut index) = self.parent_stack.last_mut()?;

            if let Some(index) = index {
                // Node has been processed.
                // Processing children.

                if let Some(child) = parent.children().get(*index) {
                    // Edge-case check.
                    if let Some(new_index) = index.checked_add(1) {
                        *index = new_index;
                    } else {
                        // All possible children were processed.
                        // Discard parent from stack.

                        let _ = self.parent_stack.pop();
                    }

                    self.parent_stack.push((child, None));
                } else {
                    // All children were processed.
                    // Discard parent from stack.

                    let _ = self.parent_stack.pop();
                }
            } else {
                // Node not yet processed.

                *index = Some(0);

                break parent;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use crate::node::Node;

    #[test]
    fn iter() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let node = Node::with_children_values(ROOT, VALUES);

        let collected_nodes = node.as_traversable().into_iter().collect::<Vec<_>>();

        assert!(core::ptr::eq(collected_nodes[0], &node));

        for index in 0..3 {
            assert!(core::ptr::eq(
                collected_nodes[index + 1],
                &node.children()[index]
            ));
        }
    }

    #[test]
    fn iter_to_traversable() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut iter = node.as_traversable().into_iter();

        // 1st -> root
        // 2nd -> 1st child
        // 3nd -> 2st child
        for _ in 0..3 {
            assert!(iter.next().is_some());
        }

        let mut traversable = iter.as_traversable().unwrap();

        assert!(core::ptr::eq(traversable.as_node(), &node.children()[1]));
        assert!(traversable.try_move_to_parent());
        assert!(core::ptr::eq(traversable.as_node(), &node));
    }
}
