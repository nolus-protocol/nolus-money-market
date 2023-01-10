#![no_std]

extern crate alloc;

pub use self::{
    node::Node,
    traversable::{NodeIter, Tranversable},
};

mod node;
mod traversable;

pub type Tree<T> = Node<T>;
