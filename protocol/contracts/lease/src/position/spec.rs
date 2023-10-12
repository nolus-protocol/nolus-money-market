use currency::Currency;
use finance::{
    coin::Coin,
    liability::Liability,
    price::{self, Price},
};

use crate::{
    api::PositionSpecDTO,
    error::{ContractError, ContractResult},
};

#[cfg_attr(test, derive(Debug))]
pub struct Spec<Lpn> {
    liability: Liability,
    min_asset: Coin<Lpn>,
    min_trasaction_amount: Coin<Lpn>,
}

impl<Lpn> Spec<Lpn>
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

    pub fn check_trasaction_amount<Trasactional>(
        &self,
        amount: Coin<Trasactional>,
        trasactional_in_lpn: Price<Trasactional, Lpn>,
    ) -> ContractResult<()>
    where
        Trasactional: Currency,
    {
        let amount = price::total(amount, trasactional_in_lpn);

        if amount < self.min_trasaction_amount {
            Err(ContractError::PositionCloseAmountTooSmall(
                self.min_trasaction_amount.into(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_asset_amount<Trasactional>(
        &self,
        asset_amount: Coin<Trasactional>,
        close_amount: Coin<Trasactional>,
        trasactional_in_lpn: Price<Trasactional, Lpn>,
    ) -> ContractResult<()>
    where
        Trasactional: Currency,
    {
        let asset_amount = price::total(asset_amount, trasactional_in_lpn);
        let close_amount = price::total(close_amount, trasactional_in_lpn);

        if asset_amount.saturating_sub(close_amount) < self.min_asset {
            Err(ContractError::PositionCloseAmountTooBig(
                self.min_asset.into(),
            ))
        } else {
            Ok(())
        }
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

impl<Lpn> From<Spec<Lpn>> for PositionSpecDTO
where
    Lpn: Currency,
{
    fn from(spec: Spec<Lpn>) -> Self {
        PositionSpecDTO::new_internal(
            spec.liability,
            spec.min_asset.into(),
            spec.min_trasaction_amount.into(),
        )
    }
}