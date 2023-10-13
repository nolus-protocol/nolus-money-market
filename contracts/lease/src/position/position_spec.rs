use currency::Currency;
use finance::{coin::Coin, liability::Liability};

use crate::{
    api::PositionSpecDTO,
    error::{ContractError, ContractResult},
};

#[cfg_attr(test, derive(Debug))]
pub struct PositionSpec<Lpn> {
    liability: Liability,
    min_asset: Coin<Lpn>,
    min_trasaction_amount: Coin<Lpn>,
}

impl<Lpn> PositionSpec<Lpn>
where
    Lpn: Currency,
{
    fn new_internal(
        liability: Liability,
        min_asset: Coin<Lpn>,
        min_trasaction_amount: Coin<Lpn>,
    ) -> Self {
        let obj = Self {
            liability,
            min_asset,
            min_trasaction_amount,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    pub fn try_from(spec: PositionSpecDTO) -> ContractResult<Self> {
        Ok(Self::new_internal(
            spec.liability,
            spec.min_asset.try_into()?,
            spec.min_sell_asset.try_into()?,
        ))
    }

    #[cfg(test)]
    pub fn new(
        liability: Liability,
        min_asset: Coin<Lpn>,
        min_trasaction_amount: Coin<Lpn>,
    ) -> Self {
        Self::new_internal(liability, min_asset, min_trasaction_amount)
    }

    pub fn liability(&self) -> Liability {
        self.liability
    }

    pub fn min_asset(&self) -> Coin<Lpn> {
        self.min_asset
    }

    pub fn min_trasaction_amount(&self) -> Coin<Lpn> {
        self.min_trasaction_amount
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(
            !self.min_asset.is_zero(),
            "Min asset amount should be positive",
        )
        .and(Self::check(
            !self.min_trasaction_amount.is_zero(),
            "Min trasaction amount should be positive",
        ))
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }
}
