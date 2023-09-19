use currency::Currency;
pub use dto::PositionDTO;
use finance::{
    coin::Coin,
    liability::{Liability, Status},
    price::{self, Price},
};

use crate::error::{ContractError, ContractResult};
pub use dto::try_from;

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

        self.liability
            .check(self.amount, total_due, overdue, min_asset, min_sell_asset)
    }

    fn invariant_held(&self) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Position<Asset, Lpn>>(
            self.amount.is_zero(),
            "The amount should be positive",
        )
        .and_then(|_| {
            ContractError::broken_invariant_if::<Position<Asset, Lpn>>(
                self.min_asset.is_zero(),
                "Min asset amount should be positive",
            )
        })
    }
}
