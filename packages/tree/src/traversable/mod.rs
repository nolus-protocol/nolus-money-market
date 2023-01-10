use alloc::vec::Vec;

use crate::node::Node;

pub use self::iter::NodeIter;

mod interface;
mod iter;

#[derive(Debug, Clone)]
pub struct Tranversable<'r, T> {
    parent_stack: Vec<&'r Node<T>>,
    current: &'r Node<T>,
}
