pub(crate) struct IterOrElseIter<Iter, AltIter>
where
    Iter: IntoIterator,
    AltIter: IntoIterator<Item = Iter::Item>,
{
    iter: Option<Iter::IntoIter>,
    alt_iter: Option<AltIter::IntoIter>,
}

impl<Iter, AltIter> IterOrElseIter<Iter, AltIter>
where
    Iter: IntoIterator,
    AltIter: IntoIterator<Item = Iter::Item>,
{
    pub fn new(iter: Iter, alt_iter: AltIter) -> Self {
        Self {
            iter: Some(iter.into_iter()),
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
        if let Some(ref mut iter) = self.iter {
            let item = iter.next();

            if item.is_some() {
                self.alt_iter = None;

                return item;
            } else {
                self.iter = None;
            }
        }

        #[expect(if_let_rescope)]
        if let Some(ref mut alt_iter) = self.alt_iter {
            alt_iter.next()
        } else {
            None
        }
    }
}
