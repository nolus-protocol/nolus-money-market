use currency::Currency;
use finance::{coin::Coin, liability::Liability};

use crate::{
    api::PositionSpecDTO,
    error::{ContractError, ContractResult},
};

#[cfg_attr(test, derive(Debug))]
pub struct Spec<Lpn> {
    liability: Liability,
    min_asset: Coin<Lpn>,
    min_sell_asset: Coin<Lpn>,
}

impl<Lpn> Spec<Lpn>
where
    Lpn: Currency,
{
    fn new_internal(liability: Liability, min_asset: Coin<Lpn>, min_sell_asset: Coin<Lpn>) -> Self {
        let obj = Self {
            liability,
            min_asset,
            min_sell_asset,
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
    pub fn new(liability: Liability, min_asset: Coin<Lpn>, min_sell_asset: Coin<Lpn>) -> Self {
        Self::new_internal(liability, min_asset, min_sell_asset)
    }

    pub fn liability(&self) -> Liability {
        self.liability
    }

    pub fn min_asset(&self) -> Coin<Lpn> {
        self.min_asset
    }

    pub fn min_sell_asset(&self) -> Coin<Lpn> {
        self.min_sell_asset
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(
            !self.min_asset.is_zero(),
            "Min asset amount should be positive",
        )
        .and(Self::check(
            !self.min_sell_asset.is_zero(),
            "Min sell asset amount should be positive",
        ))
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }
}
