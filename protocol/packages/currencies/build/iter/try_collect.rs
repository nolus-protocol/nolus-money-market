use super::try_from_iterator::TryFromIterator;

pub trait TryCollect<T, E>: Iterator<Item = Result<T, E>> {
    fn try_collect<U>(self) -> Result<U, E>
    where
        U: TryFromIterator<T>;
}

impl<I, T, E> TryCollect<T, E> for I
where
    Self: Iterator<Item = Result<T, E>>,
{
    #[inline]
    fn try_collect<U>(self) -> Result<U, E>
    where
        U: TryFromIterator<T>,
    {
        U::try_from_iter(self)
    }
}
