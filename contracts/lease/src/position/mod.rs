use currency::Currency;
pub use dto::PositionDTO;
use finance::{
    coin::Coin,
    liability::{Liability, Status},
    price::{self, Price},
    zero::Zero,
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
    fn new(
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

    #[cfg(test)]
    pub fn new_unchecked(
        amount: Coin<Asset>,
        liability: Liability,
        min_asset: Coin<Lpn>,
        min_sell_asset: Coin<Lpn>,
    ) -> Self {
        Self::new(amount, liability, min_asset, min_sell_asset)
    }

    pub fn try_from(amount: &LeaseCoin, spec: PositionSpec) -> ContractResult<Self> {
        let amount = amount.try_into()?;
        let min_asset = spec.min_asset.try_into()?;
        let min_sell_asset = spec.min_sell_asset.try_into()?;

        Ok(Self::new(amount, spec.liability, min_asset, min_sell_asset))
    }

    pub fn amount(&self) -> Coin<Asset> {
        self.amount
    }

    pub fn liability(&self) -> Liability {
        self.liability
    }

    pub fn decrease_amount_with(&mut self, removal_amount: Coin<Asset>) {
        self.amount -= removal_amount
    }

    fn invariant_held(&self) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Position<Asset, Lpn>>(
            self.min_asset <= Coin::ZERO,
            "Min asset amount should be positive",
        )
    }

    pub fn check_liability(
        &self,
        asset: Coin<Asset>,
        total_due: Coin<Lpn>,
        overdue: Coin<Lpn>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Status<Asset> {
        debug_assert!(asset != Coin::ZERO);
        debug_assert!(overdue <= total_due);

        let total_due = price::total(total_due, lpn_in_assets);
        let overdue = price::total(overdue, lpn_in_assets);
        let min_asset = price::total(self.min_asset, lpn_in_assets);
        let min_sell_asset = price::total(self.min_sell_asset, lpn_in_assets);

        self.liability
            .check(asset, total_due, overdue, min_asset, min_sell_asset)
    }
}
