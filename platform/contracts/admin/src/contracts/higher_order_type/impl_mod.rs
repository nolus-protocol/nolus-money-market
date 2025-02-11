use super::{Compose, HigherOrderTuple, Identity, Map, MapAsRef, Option, TryForEach, Zip};

impl TryForEach for Identity {
    #[inline]
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, mut f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        f(this)
    }
}

impl Map for Identity {
    #[inline]
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        f(this)
    }
}

impl MapAsRef for Identity {
    #[inline]
    fn map_as_ref<Unit>(this: &Self::Of<Unit>) -> Self::Of<&Unit> {
        this
    }
}

impl Zip for Identity {
    #[inline]
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)> {
        (left, right)
    }
}

impl TryForEach for Option {
    #[inline]
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        this.map_or(const { Ok(()) }, f)
    }
}

impl Map for Option {
    #[inline]
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        this.map(f)
    }
}

impl MapAsRef for Option {
    #[inline]
    fn map_as_ref<Unit>(this: &Self::Of<Unit>) -> Self::Of<&Unit> {
        this.as_ref()
    }
}

impl Zip for Option {
    #[inline]
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)> {
        left.zip(right)
    }
}

impl<Bound> Map for HigherOrderTuple<false, Bound> {
    #[inline]
    fn map<Unit, F, MappedUnit>((left, right): Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        (left, f(right))
    }
}

impl<Bound> Map for HigherOrderTuple<true, Bound> {
    #[inline]
    fn map<Unit, F, MappedUnit>((left, right): Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        (f(left), right)
    }
}

impl<Outer, Inner> TryForEach for Compose<Outer, Inner>
where
    Outer: TryForEach,
    Inner: TryForEach,
{
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, mut f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        Outer::try_for_each(this, |this| Inner::try_for_each(this, &mut f))
    }
}

impl<Outer, Inner> Map for Compose<Outer, Inner>
where
    Outer: Map,
    Inner: Map,
{
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        Outer::map(this, |inner| Inner::map(inner, &mut f))
    }
}

impl<Outer, Inner> Zip for Compose<Outer, Inner>
where
    Outer: Map + Zip,
    Inner: Zip,
{
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)> {
        Outer::map(Outer::zip(left, right), |(left, right)| {
            Inner::zip(left, right)
        })
    }
}
