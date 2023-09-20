use currency::Currency;
pub use dto::PositionDTO;
use finance::{
    coin::Coin,
    liability::{Liability, Status},
    price::{self, Price},
};

use crate::{
    api::{LeaseCoin, PositionSpec},
    error::{ContractError, ContractResult},
};

mod dto;

#[cfg_attr(test, derive(Debug))]
pub struct Position<Asset, Lpn> {
    amount: Coin<Asset>,
    liability: Liability,
    min_asset: Coin<Lpn>,
    min_sell_asset: Coin<Lpn>,
}

impl<Asset, Lpn> Position<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    fn new_internal(
        amount: Coin<Asset>,
        liability: Liability,
        min_asset: Coin<Lpn>,
        min_sell_asset: Coin<Lpn>,
    ) -> Self {
        let obj = Self {
            amount,
            liability,
            min_asset,
            min_sell_asset,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    pub fn try_from(amount: &LeaseCoin, spec: PositionSpec) -> ContractResult<Self> {
        Ok(Self::new_internal(
            amount.try_into()?,
            spec.liability,
            spec.min_asset.try_into()?,
            spec.min_sell_asset.try_into()?,
        ))
    }

    #[cfg(test)]
    pub fn new(
        amount: Coin<Asset>,
        liability: Liability,
        min_asset: Coin<Lpn>,
        min_sell_asset: Coin<Lpn>,
    ) -> Self {
        Self::new_internal(amount, liability, min_asset, min_sell_asset)
    }

    pub fn amount(&self) -> Coin<Asset> {
        self.amount
    }

    // `self.liability` is semi-hidden, semi-public - it's exposed just for computing the recalculation period
    // let's return `recalc_liability_at` as a data of `Status`
    // for more insights refer to the TODO next to `Liability::check`
    pub fn liability(&self) -> Liability {
        self.liability
    }

    pub fn close(&mut self, amount: Coin<Asset>) {
        debug_assert!(amount <= self.amount);

        self.amount -= amount
    }

    pub fn check_liability(
        &self,
        total_due: Coin<Lpn>,
        overdue: Coin<Lpn>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Status<Asset> {
        debug_assert!(overdue <= total_due);

        let total_due = price::total(total_due, lpn_in_assets);
        let overdue = price::total(overdue, lpn_in_assets);
        let min_asset = price::total(self.min_asset, lpn_in_assets);
        let min_sell_asset = price::total(self.min_sell_asset, lpn_in_assets);

        // TODO the functionality consuming both `min*_asset` amounts should be moved
        // to this type
        // finance::Liability's responsability is only to provide `amount_to_liquidate`
        self.liability
            .check(self.amount, total_due, overdue, min_asset, min_sell_asset)
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(!self.amount.is_zero(), "The amount should be positive").and(Self::check(
            !self.min_asset.is_zero(),
            "Min asset amount should be positive",
        ))
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }
}
