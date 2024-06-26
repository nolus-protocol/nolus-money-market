use crate::validate::Validate;

use super::{
    super::{AsRef, ForEachPair, HigherOrderType, TryForEach},
    PlatformContracts,
};

impl<T> AsRef for PlatformContracts<T> {
    type Item = T;

    type HigherOrderType = super::HigherOrderType;

    fn as_ref(&self) -> <Self::HigherOrderType as HigherOrderType>::Of<&Self::Item> {
        PlatformContracts {
            timealarms: &self.timealarms,
            treasury: &self.treasury,
        }
    }
}

impl<T> TryForEach for PlatformContracts<T> {
    type Item = T;

    fn try_for_each<U, F, E>(self, accumulator: U, mut functor: F) -> Result<U, E>
    where
        F: FnMut(Self::Item, U) -> Result<U, E>,
    {
        functor(self.timealarms, accumulator)
            .and_then(|accumulator| functor(self.treasury, accumulator))
    }
}

impl<T> ForEachPair for PlatformContracts<T> {
    type Item = T;

    type HigherOrderType = super::HigherOrderType;

    fn for_each_pair<U, V, F>(
        self,
        counter_part: PlatformContracts<U>,
        mut accumulator: V,
        mut functor: F,
    ) -> V
    where
        F: FnMut(Self::Item, U, V) -> V,
    {
        accumulator = functor(self.timealarms, counter_part.timealarms, accumulator);

        functor(self.treasury, counter_part.treasury, accumulator)
    }
}

impl<T> Validate for PlatformContracts<T>
where
    T: Validate,
{
    type Context<'r> = T::Context<'r>;

    type Error = T::Error;

    fn validate(&self, ctx: Self::Context<'_>) -> Result<(), Self::Error> {
        self.as_ref()
            .try_for_each((), |contract, ()| contract.validate(ctx))
    }
}
