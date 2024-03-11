#[derive(Clone)]
pub(crate) enum EitherIter<LeftIter, RightIter>
where
    LeftIter: Iterator,
    RightIter: Iterator<Item = LeftIter::Item>,
{
    Left(LeftIter),
    Right(RightIter),
}

impl<LeftIter, RightIter> Iterator for EitherIter<LeftIter, RightIter>
where
    LeftIter: Iterator,
    RightIter: Iterator<Item = LeftIter::Item>,
{
    type Item = LeftIter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            EitherIter::Left(iter) => iter.next(),
            EitherIter::Right(iter) => iter.next(),
        }
    }
}
