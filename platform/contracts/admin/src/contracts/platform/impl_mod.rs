use crate::validate::Validate;

use super::{
    super::higher_order_type::{Map, MapAsRef, TryForEach, Zip},
    Contracts, HigherOrderType,
};

impl<T> Validate for Contracts<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        HigherOrderType::try_for_each(HigherOrderType::map_as_ref(self), |contract| {
            contract.validate(ctx)
        })
    }
}

impl TryForEach for HigherOrderType {
    #[inline]
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        [this.timealarms, this.treasury].into_iter().try_for_each(f)
    }
}

impl Map for HigherOrderType {
    #[inline]
    fn map<Unit, F, MappedUnit>(this: Self::Of<Unit>, mut f: F) -> Self::Of<MappedUnit>
    where
        F: FnMut(Unit) -> MappedUnit,
    {
        Contracts {
            timealarms: f(this.timealarms),
            treasury: f(this.treasury),
        }
    }
}

impl MapAsRef for HigherOrderType {
    #[inline]
    fn map_as_ref<T>(this: &Self::Of<T>) -> Self::Of<&T> {
        Contracts {
            timealarms: &this.timealarms,
            treasury: &this.treasury,
        }
    }
}

impl Zip for HigherOrderType {
    #[inline]
    fn zip<LeftUnit, RightUnit>(
        left: Self::Of<LeftUnit>,
        right: Self::Of<RightUnit>,
    ) -> Self::Of<(LeftUnit, RightUnit)> {
        Contracts {
            timealarms: (left.timealarms, right.timealarms),
            treasury: (left.treasury, right.treasury),
        }
    }
}
