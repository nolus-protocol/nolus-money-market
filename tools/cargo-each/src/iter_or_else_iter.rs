pub(crate) struct IterOrElseIter<Iter, AltIter>
where
    Iter: Iterator,
    AltIter: Iterator<Item = Iter::Item>,
{
    iter: Iter,
    alt_iter: Option<AltIter>,
}

impl<Iter, AltIter> IterOrElseIter<Iter, AltIter>
where
    Iter: Iterator,
    AltIter: Iterator<Item = Iter::Item>,
{
    pub fn new(iter: Iter, alt_iter: AltIter) -> Self {
        Self {
            iter,
            alt_iter: Some(alt_iter),
        }
    }
}

impl<Iter, AltIter> Iterator for IterOrElseIter<Iter, AltIter>
where
    Iter: Iterator,
    AltIter: Iterator<Item = Iter::Item>,
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
