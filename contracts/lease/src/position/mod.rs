use currency::Currency;
pub use dto::PositionDTO;
use finance::{coin::Coin, liability::Liability, zero::Zero};

use crate::error::{ContractError, ContractResult};

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
    pub fn new(
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
}
