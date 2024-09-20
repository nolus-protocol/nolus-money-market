use std::borrow::Cow;

pub(crate) trait SubtypeLifetime<'r>: 'r {
    type T<'new>: 'new
    where
        'r: 'new;

    fn subtype<'new>(self) -> Self::T<'new>
    where
        'r: 'new;
}

impl<'old, T> SubtypeLifetime<'old> for &'old T
where
    T: ?Sized,
{
    type T<'new> = &'new T
where
    'old: 'new;

    #[inline]
    fn subtype<'new>(self) -> Self::T<'new>
    where
        'old: 'new,
    {
        self
    }
}

impl<'old, T> SubtypeLifetime<'old> for &'old mut T
where
    T: ?Sized,
{
    type T<'new> = &'new mut T
where
    'old: 'new;

    #[inline]
    fn subtype<'new>(self) -> Self::T<'new>
    where
        'old: 'new,
    {
        self
    }
}

impl<'old, T> SubtypeLifetime<'old> for Cow<'old, T>
where
    T: ToOwned + ?Sized,
{
    type T<'new> = Cow<'new, T>
where
    'old: 'new;

    #[inline]
    fn subtype<'new>(self) -> Self::T<'new>
    where
        'old: 'new,
    {
        self
    }
}
