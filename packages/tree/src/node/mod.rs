use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::traversable::Tranversable;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Node<T> {
    value: T,
    children: Vec<Self>,
}

impl<T> Node<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value,
            children: Vec::new(),
        }
    }

    pub fn with_children_iter<I>(value: T, children: I) -> Self
    where
        I: IntoIterator<Item = Self>,
    {
        Self {
            value,
            children: Vec::from_iter(children),
        }
    }

    #[inline]
    pub fn with_children_values<I>(value: T, children: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Self::with_children_iter(value, children.into_iter().map(Self::new))
    }

    #[inline]
    pub const fn as_traversable(&self) -> Tranversable<T> {
        Tranversable::new(self)
    }

    #[inline]
    pub const fn value(&self) -> &T {
        &self.value
    }

    #[inline]
    pub fn value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    #[inline]
    pub fn children(&self) -> &[Self] {
        &self.children
    }

    #[inline]
    pub fn children_mut(&mut self) -> &mut [Self] {
        &mut self.children
    }
}

impl<T> From<T> for Node<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Extend<T> for Node<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.children.extend(iter.into_iter().map(Self::new))
    }
}

impl<T> Extend<Self> for Node<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Self>,
    {
        self.children.extend(iter)
    }
}

#[cfg(test)]
mod tests {

    use alloc::vec::Vec;

    use super::Node;

    #[test]
    fn constructor() {
        const VALUE: &str = "123";

        let node = Node::new(VALUE);

        assert_eq!(node.value, VALUE);
        assert_eq!(node.value(), &VALUE);
        assert!(node.children.is_empty());
    }

    #[test]
    fn with_iter() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let iter = VALUES.into_iter().map(Node::new);

        let node = Node::with_children_iter(ROOT, iter.clone());

        assert_eq!(node.value, ROOT);
        assert_eq!(node.value(), &ROOT);
        assert!(!node.children.is_empty());
        assert_eq!(node.children, Vec::from_iter(iter));
        assert_eq!(node.children.as_slice(), node.children());
    }

    #[test]
    fn with_values() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let node = Node::with_children_values(ROOT, VALUES);

        assert_eq!(node.value, ROOT);
        assert_eq!(node.value(), &ROOT);
        assert!(!node.children.is_empty());
        assert_eq!(
            node.children,
            Vec::from_iter(VALUES.into_iter().map(Node::new))
        );
        assert!(core::ptr::eq(node.children.as_slice(), node.children()));
    }

    #[test]
    fn getters() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let mut node = Node::with_children_values(ROOT, VALUES);

        assert!(core::ptr::eq(node.value(), &node.value));
        assert!(core::ptr::eq(node.value_mut(), &node.value));
        assert!(core::ptr::eq(node.children(), node.children.as_slice()));
        assert!(core::ptr::eq(node.children_mut(), node.children.as_slice()));
    }

    #[test]
    fn extend_with_value() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let mut node = Node::new(ROOT);

        assert!(node.children().is_empty());

        node.extend(VALUES);

        assert_eq!(
            node.children,
            Vec::from_iter(VALUES.into_iter().map(Node::new))
        );
    }

    #[test]
    fn extend_with_nodes() {
        const ROOT: &str = "root";
        const VALUES: [&str; 3] = ["1", "2", "3"];

        let mut node = Node::new(ROOT);

        assert!(node.children().is_empty());

        let iter = VALUES.into_iter().map(Node::new);

        node.extend(iter.clone());

        assert_eq!(node.children, Vec::from_iter(iter));
    }
}
