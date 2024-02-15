pub mod storage;

pub trait HigherOrderDyn: 'static {
    type Dyn<'r>: ?Sized + 'r
    where
        Self: 'r;
}

pub trait AsDyn<T>
where
    T: HigherOrderDyn + ?Sized,
{
    fn as_dyn(&self) -> &T::Dyn<'_>;
}

pub trait AsDynMut<T>: AsDyn<T>
where
    T: HigherOrderDyn + ?Sized,
{
    fn as_dyn_mut(&mut self) -> &mut T::Dyn<'_>;
}

impl<'r, T, U> AsDyn<U> for &'r T
where
    T: AsDyn<U> + ?Sized,
    U: HigherOrderDyn + ?Sized,
{
    fn as_dyn(&self) -> &U::Dyn<'_> {
        T::as_dyn(self)
    }
}

impl<'r, T, U> AsDyn<U> for &'r mut T
where
    T: AsDyn<U> + ?Sized,
    U: HigherOrderDyn + ?Sized,
{
    fn as_dyn(&self) -> &U::Dyn<'_> {
        T::as_dyn(self)
    }
}

impl<'r, T, U> AsDynMut<U> for &'r mut T
where
    T: AsDynMut<U> + ?Sized,
    U: HigherOrderDyn + ?Sized,
{
    fn as_dyn_mut(&mut self) -> &mut U::Dyn<'_> {
        T::as_dyn_mut(self)
    }
}
