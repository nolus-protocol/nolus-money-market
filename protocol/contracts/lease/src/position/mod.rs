use currency::Currency;
use finance::{
    coin::Coin,
    duration::Duration,
    fraction::Fraction,
    liability::Level,
    price::{total_of, Price},
};

use crate::{
    api::LeaseCoin,
    error::{ContractError, ContractResult},
};

pub use dto::PositionDTO;
pub use interest::InterestDue;
pub use spec::Spec;
pub use status::{Cause, Debt, Liquidation};

mod dto;
mod interest;
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

    pub(crate) fn amount(&self) -> Coin<Asset> {
        self.amount
    }

    pub fn close(&mut self, asset: Coin<Asset>) {
        debug_assert!(
            asset < self.amount,
            "Liquidated asset {asset} should be less than the available {0}",
            self.amount
        );

        self.amount -= asset
    }

    /// Compute how much time is necessary for the due interest to become collectable
    ///
    /// If it is already enough to be collected then return zero.
    pub fn overdue_liquidation_in<Interest>(&self, interest: &Interest) -> Duration
    where
        Interest: InterestDue<Lpn>,
    {
        self.spec.overdue_liquidation_in(interest)
    }

    pub fn debt(
        &self,
        total_due: Coin<Lpn>,
        overdue: Coin<Lpn>,
        asset_in_lpns: Price<Asset, Lpn>,
    ) -> Debt<Asset> {
        self.spec
            .debt(self.amount, total_due, overdue, asset_in_lpns)
    }

    /// Check if the amount can be used for repayment.
    /// Return `error::ContractError::InsufficientPayment` when the payment amount
    /// is less than the minimum transaction amount.
    pub fn validate_payment<PaymentC>(
        &self,
        payment: Coin<PaymentC>,
        payment_currency_in_lpns: Price<PaymentC, Lpn>,
    ) -> ContractResult<()>
    where
        PaymentC: Currency,
    {
        self.spec
            .validate_payment(payment, payment_currency_in_lpns)
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

    /// Calculate the price at which the lease reaches given ltv.
    pub(crate) fn price_at(
        &self,
        level: Level,
        total_due: Coin<Lpn>,
    ) -> ContractResult<Price<Asset, Lpn>> {
        debug_assert!(!total_due.is_zero());
        debug_assert!(!level.ltv().is_zero());

        Ok(total_of(level.ltv().of(self.amount)).is(total_due))
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(!self.amount.is_zero(), "The amount should be positive")
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }
}
