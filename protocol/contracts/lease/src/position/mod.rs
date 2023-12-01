use currency::Currency;
use finance::{coin::Coin, liability::Liability, price::Price};

use crate::{
    api::LeaseCoin,
    error::{ContractError, ContractResult},
};

pub use dto::PositionDTO;
pub use spec::Spec;
pub use status::{Cause, Liquidation, Status};

mod dto;
mod spec;
mod status;

#[cfg_attr(test, derive(Debug))]
pub struct Position<Asset, Lpn> {
    amount: Coin<Asset>,
    spec: Spec<Lpn>,
}

impl<Asset, Lpn> Position<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    fn new_internal(amount: Coin<Asset>, spec: Spec<Lpn>) -> Self {
        let obj = Self { amount, spec };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    pub fn try_from(amount: LeaseCoin, spec: Spec<Lpn>) -> ContractResult<Self> {
        amount
            .try_into()
            .map_err(Into::into)
            .map(|amount| Self::new_internal(amount, spec))
    }

    #[cfg(test)]
    pub fn new(amount: Coin<Asset>, spec: Spec<Lpn>) -> Self {
        Self::new_internal(amount, spec)
    }

    pub fn amount(&self) -> Coin<Asset> {
        self.amount
    }

    // `self.liability` is semi-hidden, semi-public - it's exposed just for computing the recalculation period
    // let's return `recalc_liability_at` as a data of `Status`
    // for more insights refer to the TODO next to `Spec::check_liability`
    pub fn liability(&self) -> Liability {
        self.spec.liability()
    }

    pub fn close(&mut self, asset: Coin<Asset>) {
        debug_assert!(
            asset < self.amount,
            "Liquidated asset {asset} should be less than the available {0}",
            self.amount
        );

        self.amount -= asset
    }

    pub fn check_liability(
        &self,
        total_due: Coin<Lpn>,
        overdue: Coin<Lpn>,
        asset_in_lpns: Price<Asset, Lpn>,
    ) -> Status<Asset> {
        self.spec
            .check_liability(self.amount, total_due, overdue, asset_in_lpns)
    }

    /// Check if the amount can be used to repay the interests.
    /// Return `error::ContractError::InsufficientPayment` when the payment amount
    /// is less than the minimum transaction amount.
    /// Return `error::ContractError::RestAmountTooSmall` when after the repayment the rest amount
    /// is less than the minimum transaction amount.
    pub fn validate_payment(
        &self,
        payment: Coin<Lpn>,
        total_due: Coin<Lpn>,
    ) -> ContractResult<Coin<Lpn>> {
        self.spec.validate_payment(payment, total_due)
    }

    /// Check if the amount can be used to close the position.
    /// Return `error::ContractError::PositionCloseAmountTooSmall` when a partial close is requested
    /// with amount less than the minimum transaction position parameter sent on lease open. Refer to
    /// `NewLeaseForm::position_spec`.
    ///
    /// Return `error::ContractError::PositionCloseAmountTooBig` when a partial close is requested
    /// with amount that would decrease a position less than the minimum asset parameter sent on
    /// lease open. Refer to `NewLeaseForm::position_spec`.
    pub fn validate_close_amount(
        &self,
        close_amount: Coin<Asset>,
        asset_in_lpns: Price<Asset, Lpn>,
    ) -> ContractResult<()> {
        self.spec
            .validate_close_amount(self.amount, close_amount, asset_in_lpns)
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(!self.amount.is_zero(), "The amount should be positive")
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }
}
