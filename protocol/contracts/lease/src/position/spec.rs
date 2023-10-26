use std::ops::Add;

use currency::Currency;
use finance::{
    coin::Coin,
    liability::Liability,
    percent::Percent,
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

    pub fn check_asset<Trasactional>(
        &self,
        amount: Coin<Trasactional>,
        trasactional_in_lpn: Price<Trasactional, Lpn>,
    ) -> ContractResult<()>
    where
        Trasactional: Currency,
    {
        let asset_amount = price::total(amount, trasactional_in_lpn);

        if asset_amount < self.min_asset {
            Err(ContractError::InsufficientAssetAmount(
                self.min_asset.into(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn check_asset_amount_on_liq<Trasactional>(
        &self,
        asset_amount: Coin<Trasactional>,
        trasactional_in_lpn: Price<Trasactional, Lpn>,
    ) -> ContractResult<()>
    where
        Trasactional: Currency,
    {
        let asset_amount = price::total(asset_amount, trasactional_in_lpn);
       
        if asset_amount < self.min_asset {
            Err(ContractError::PositionCloseAmountTooBig(
                self.min_asset.into(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn calc_borrow_amount(
        &self,
        downpayment: Coin<Lpn>,
        may_max_ltd: Option<Percent>,
    ) -> ContractResult<Coin<Lpn>> {
        let price: Price<Lpn, Lpn> = price::total_of(1.into()).is(1.into());

        let _ = self
            .check_trasaction_amount(downpayment, price)
            .map_err(|err| match err {
                ContractError::PositionCloseAmountTooSmall(min_amount) => {
                    ContractError::InsufficientTrasactionAmount(min_amount)
                }
                _ => err,
            });
        let borrow = self.liability.init_borrow_amount(downpayment, may_max_ltd);
        self.check_trasaction_amount(borrow, price)
            .map_err(|err| match err {
                ContractError::PositionCloseAmountTooSmall(min_amount) => {
                    ContractError::InsufficientTrasactionAmount(min_amount)
                }
                _ => err,
            })
            .and_then(|_| self.check_asset(downpayment.add(borrow), price))
            .map(|_| borrow)
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
