use alloc::vec::Vec;
use core::{mem::replace, ops::Deref};

use crate::node::Node;

use super::{iter::NodeIter, Tranversable};

impl<'r, T> Tranversable<'r, T> {
    pub(crate) const fn new(node: &'r Node<T>) -> Self {
        Self {
            parent_stack: Vec::new(),
            current: node,
        }
    }
}

impl<'r, T> Tranversable<'r, T> {
    pub const fn copy_detached(&self) -> Self {
        Self {
            parent_stack: Vec::new(),
            current: self.current,
        }
    }

    #[inline]
    pub const fn as_node(&self) -> &'r Node<T> {
        self.current
    }

    #[inline]
    pub fn has_parent(&self) -> bool {
        !self.parent_stack.is_empty()
    }

    pub fn try_move_to_parent(&mut self) -> bool {
        self.parent_stack
            .pop()
            .map(|parent| self.current = parent)
            .is_some()
    }

    pub fn try_move_to_child_by_index(&mut self, index: usize) -> bool {
        self.try_move_to_child(move |children| children.get(index))
    }

    pub fn try_move_to_child_by<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&'r T) -> bool,
    {
        self.try_move_to_child(move |children| children.iter().find(|node| f(node.value())))
    }

    pub fn try_move_to_child_by_rev<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&'r T) -> bool,
    {
        self.try_move_to_child(move |children| children.iter().rev().find(|node| f(node.value())))
    }

    #[inline]
    fn try_move_to_child<F>(&mut self, mut f: F) -> bool
    where
        F: FnMut(&'r [Node<T>]) -> Option<&'r Node<T>>,
    {
        f(self.current.children())
            .map(|child| self.parent_stack.push(replace(&mut self.current, child)))
            .is_some()
    }
}

impl<'r, K, V> Tranversable<'r, (K, V)> {
    pub fn try_move_to_first_child_by_key<Q>(&mut self, key: &Q) -> bool
    where
        K: AsRef<Q>,
        Q: PartialEq + Eq + ?Sized,
    {
        self.try_move_to_child_by(move |value| value.0.as_ref() == key)
    }

    pub fn try_move_to_nth_child_by_key<Q>(&mut self, n: usize, key: &Q) -> bool
    where
        K: AsRef<Q>,
        Q: PartialEq + Eq + ?Sized,
    {
        self.try_move_to_child(move |children| {
            children
                .iter()
                .filter(move |node| node.value().0.as_ref() == key)
                .nth(n)
        })
    }

    pub fn try_move_to_last_child_by_key<Q>(&mut self, key: &Q) -> bool
    where
        K: AsRef<Q>,
        Q: PartialEq + Eq + ?Sized,
    {
        self.try_move_to_child_by_rev(move |value| value.0.as_ref() == key)
    }
}

impl<'r, T> Deref for Tranversable<'r, T> {
    type Target = Node<T>;

    fn deref(&self) -> &Self::Target {
        self.current
    }
}

impl<'r, T> IntoIterator for Tranversable<'r, T> {
    type Item = &'r Node<T>;
    type IntoIter = NodeIter<'r, T>;

    fn into_iter(self) -> Self::IntoIter {
        NodeIter::new(self.current)
    }
}

impl<'r, T> IntoIterator for &'_ Tranversable<'r, T> {
    type Item = &'r Node<T>;
    type IntoIter = NodeIter<'r, T>;

    fn into_iter(self) -> Self::IntoIter {
        NodeIter::new(self.current)
    }
}

impl<'r, T> IntoIterator for &'_ mut Tranversable<'r, T> {
    type Item = &'r Node<T>;
    type IntoIter = NodeIter<'r, T>;

    fn into_iter(self) -> Self::IntoIter {
        NodeIter::new(self.current)
    }
}

#[cfg(test)]
mod tests {
    use crate::node::Node;

    use super::Tranversable;

    #[test]
    fn construct() {
        let node = Node::new("ROOT");

        let as_traversable = node.as_traversable();

        let constructed_traversable = Tranversable::new(&node);

        assert!(core::ptr::eq(as_traversable.current, &node));
        assert!(core::ptr::eq(&*as_traversable, &node));
        assert!(core::ptr::eq(constructed_traversable.current, &node));
        assert!(core::ptr::eq(&*constructed_traversable, &node));
    }

    #[test]
    fn parent() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        assert!(core::ptr::eq(traversable.as_node(), &node));
        assert!(!traversable.has_parent());
        assert!(!traversable.try_move_to_parent());

        assert!(traversable.try_move_to_child_by_index(0));
        assert!(!core::ptr::eq(traversable.as_node(), &node));

        assert!(traversable.has_parent());
        assert!(traversable.try_move_to_parent());
        assert!(core::ptr::eq(traversable.as_node(), &node));
    }

    #[test]
    fn copy_detached() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        assert!(traversable.try_move_to_child_by_index(0));
        assert!(core::ptr::eq(traversable.as_node(), &node.children()[0]));

        assert!(traversable.has_parent());
        traversable = traversable.copy_detached();
        assert!(!traversable.has_parent());
    }

    #[test]
    fn child_by_index() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        for (index, child) in node.children().iter().enumerate() {
            assert!(traversable.try_move_to_child_by_index(index));
            assert!(core::ptr::eq(traversable.as_node(), child));

            assert!(traversable.try_move_to_parent());
        }

        assert!(core::ptr::eq(traversable.as_node(), &node));
        assert!(!traversable.try_move_to_child_by_index(node.children().len()));
        assert!(core::ptr::eq(traversable.as_node(), &node));
    }

    #[test]
    fn child_by_functor() {
        const ROOT: &str = "root";
        const VALUES: [&str; 4] = ["1", "2", "3", "4"];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        assert!(traversable.try_move_to_child_by({
            let mut second = false;

            move |_| core::mem::replace(&mut second, true)
        }));
        assert!(core::ptr::eq(traversable.as_node(), &node.children()[1]));
    }

    #[test]
    fn child_by_functor_reverse() {
        const ROOT: &str = "root";
        const VALUES: [&str; 4] = ["1", "2", "3", "4"];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        assert!(traversable.try_move_to_child_by_rev({
            let mut second = false;

            move |_| core::mem::replace(&mut second, true)
        }));
        assert!(core::ptr::eq(traversable.as_node(), &node.children()[2]));
    }

    #[test]
    fn first_child_by_key() {
        const ROOT: (&str, u32) = ("root", 0);
        const VALUES: [(&str, u32); 5] = [("1", 1), ("2", 2), ("2", 3), ("2", 4), ("3", 5)];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        assert!(traversable.try_move_to_first_child_by_key("2"));
        assert!(core::ptr::eq(traversable.as_node(), &node.children()[1]));
    }

    #[test]
    fn nth_child_by_key() {
        const ROOT: (&str, u32) = ("root", 0);
        const VALUES: [(&str, u32); 5] = [("1", 1), ("2", 2), ("2", 3), ("2", 4), ("3", 5)];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        assert!(traversable.try_move_to_nth_child_by_key(1, "2"));
        assert!(core::ptr::eq(traversable.as_node(), &node.children()[2]));
    }

    #[test]
    fn last_child_by_key() {
        const ROOT: (&str, u32) = ("root", 0);
        const VALUES: [(&str, u32); 5] = [("1", 1), ("2", 2), ("2", 3), ("2", 4), ("3", 5)];

        let node = Node::with_children_values(ROOT, VALUES);

        let mut traversable = node.as_traversable();

        assert!(traversable.try_move_to_last_child_by_key("2"));
        assert!(core::ptr::eq(traversable.as_node(), &node.children()[3]));
    }
}
