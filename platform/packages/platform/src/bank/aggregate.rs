use std::result::Result as StdResult;

use crate::batch::Batch;

pub trait Aggregate {
    fn aggregate(self, other: Self) -> Self
    where
        Self: Sized;
}

impl Aggregate for () {
    fn aggregate(self, _: Self) -> Self {}
}

impl Aggregate for Batch {
    fn aggregate(self, other: Self) -> Self {
        self.merge(other)
    }
}

impl<T> Aggregate for Vec<T> {
    fn aggregate(mut self, mut other: Self) -> Self {
        self.append(&mut other);

        self
    }
}

/// Temporary replacement for functionality similar to
/// [`Iterator::try_reduce`] until the feature is stabilized.
pub trait ReduceResults
where
    Self: Iterator<Item = StdResult<Self::InnerItem, Self::Error>>,
{
    type InnerItem;
    type Error;

    fn reduce_results<F>(&mut self, f: F) -> Option<StdResult<Self::InnerItem, Self::Error>>
    where
        F: FnMut(Self::InnerItem, Self::InnerItem) -> Self::InnerItem;
}

impl<I, T, E> ReduceResults for I
where
    I: Iterator<Item = StdResult<T, E>>,
{
    type InnerItem = T;
    type Error = E;

    fn reduce_results<F>(&mut self, mut f: F) -> Option<StdResult<T, E>>
    where
        F: FnMut(T, T) -> T,
    {
        self.next().map(|first: StdResult<T, E>| {
            first.and_then(|first: T| {
                self.try_fold(first, |acc: T, element: StdResult<T, E>| {
                    element.map(|element: T| f(acc, element))
                })
            })
        })
    }
}
