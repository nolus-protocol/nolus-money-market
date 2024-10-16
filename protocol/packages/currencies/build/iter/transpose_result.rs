use std::iter;

use crate::either::Either;

pub(crate) trait TransposeResult {
    type Ok;

    type Err;

    fn transpose(self) -> impl Iterator<Item = Result<Self::Ok, Self::Err>>;
}

impl<T, E> TransposeResult for Result<T, E>
where
    T: IntoIterator,
{
    type Ok = T::Item;

    type Err = E;

    fn transpose(self) -> impl Iterator<Item = Result<T::Item, E>> {
        match self {
            Ok(value) => Either::Left(value.into_iter().map(Ok)),
            Err(error) => Either::Right(iter::once(Err(error))),
        }
    }
}