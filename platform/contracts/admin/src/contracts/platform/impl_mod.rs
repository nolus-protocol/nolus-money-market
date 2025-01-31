use crate::validate::Validate;

use super::{
    super::{MapAsRef, TryForEach, Zip},
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

impl MapAsRef for HigherOrderType {
    fn map_as_ref<T>(this: &Self::Of<T>) -> Self::Of<&T> {
        Contracts {
            timealarms: &this.timealarms,
            treasury: &this.treasury,
        }
    }
}

impl TryForEach for HigherOrderType {
    fn try_for_each<Unit, F, Err>(this: Self::Of<Unit>, f: F) -> Result<(), Err>
    where
        F: FnMut(Unit) -> Result<(), Err>,
    {
        [this.timealarms, this.treasury].into_iter().try_for_each(f)
    }
}

impl Zip for HigherOrderType {
    fn zip<LeftUnit, RightUnit>(
        left: Contracts<LeftUnit>,
        right: Contracts<RightUnit>,
    ) -> Contracts<(LeftUnit, RightUnit)> {
        Contracts {
            timealarms: (left.timealarms, right.timealarms),
            treasury: (left.treasury, right.treasury),
        }
    }
}
