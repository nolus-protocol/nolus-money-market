pub(crate) struct IterOrElseIter<Iter, AltIter>
where
    Iter: IntoIterator,
    AltIter: IntoIterator<Item = Iter::Item>,
{
    iter: Iter::IntoIter,
    alt_iter: Option<AltIter::IntoIter>,
}

impl<Iter, AltIter> IterOrElseIter<Iter, AltIter>
where
    Iter: IntoIterator,
    AltIter: IntoIterator<Item = Iter::Item>,
{
    pub fn new(iter: Iter, alt_iter: AltIter) -> Self {
        Self {
            iter: iter.into_iter(),
            alt_iter: Some(alt_iter.into_iter()),
        }
    }
}

impl<Iter, AltIter> Iterator for IterOrElseIter<Iter, AltIter>
where
    Iter: IntoIterator,
    AltIter: IntoIterator<Item = Iter::Item>,
{
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        #[expect(if_let_rescope)]
        if let Some(value) = self.iter.next() {
            self.alt_iter = None;

            Some(value)
        } else {
            self.alt_iter.as_mut().and_then(Iterator::next)
        }
    }
}
