use super::try_extend::TryExtend;

pub trait TryFromIterator<T>: Sized {
    fn try_from_iter<I, E>(iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>;
}

impl<T> TryFromIterator<T> for String
where
    T: AsRef<str>,
{
    #[inline]
    fn try_from_iter<I, E>(iter: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<T, E>>,
    {
        String::new().try_extend(iter)
    }
}
