pub(crate) trait TryExtend<T>: Sized {
    fn try_extend<I, E>(self, iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>;
}

impl<T> TryExtend<T> for String
where
    T: AsRef<str>,
{
    fn try_extend<I, E>(self, iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>,
    {
        iter.into_iter().try_fold(self, |mut accumulator, element| {
            element.map(|element| {
                accumulator.push_str(element.as_ref());

                accumulator
            })
        })
    }
}