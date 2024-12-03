use std::ops::Add;

use currency::{Currency, CurrencyDef, MemberOf};
use finance::{
    coin::Coin,
    duration::Duration,
    fraction::Fraction,
    liability::Liability,
    percent::Percent,
    price::{self},
};

use crate::{
    api::{position::ClosePolicyChange, LeasePaymentCurrencies},
    finance::{LpnCoin, Price},
};

use super::{
    close::Policy as ClosePolicy, interest::OverdueCollection, steady::Steadiness, Cause,
    CloseStrategy, Debt, DueTrait, Liquidation, PositionError, PositionResult,
};
pub use dto::SpecDTO;

mod dto;

#[cfg(test)]
mod test;

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug))]
pub struct Spec {
    liability: Liability,
    close: ClosePolicy,
    min_asset: LpnCoin,
    min_transaction: LpnCoin,
}

impl Spec {
    pub fn new(
        liability: Liability,
        close: ClosePolicy,
        min_asset: LpnCoin,
        min_transaction: LpnCoin,
    ) -> Self {
        debug_assert!(!min_asset.is_zero(), "Min asset amount should be positive",);
        debug_assert!(
            !min_transaction.is_zero(),
            "Min transaction amount should be positive",
        );
        Self {
            liability,
            close,
            min_asset,
            min_transaction,
        }
    }

    #[cfg(test)]
    pub fn no_close(liability: Liability, min_asset: LpnCoin, min_transaction: LpnCoin) -> Self {
        Self::new(
            liability,
            ClosePolicy::default(),
            min_asset,
            min_transaction,
        )
    }

    pub fn change_close_policy<Asset, Due>(
        self,
        cmd: ClosePolicyChange,
        asset: Coin<Asset>,
        due: &Due,
        asset_in_lpns: Price<Asset>,
    ) -> PositionResult<Self>
    where
        Asset: Currency,
        Due: DueTrait,
    {
        self.close
            .change_policy(cmd, asset, Self::to_assets(due.total_due(), asset_in_lpns))
            .map(|close_policy| {
                Self::new(
                    self.liability,
                    close_policy,
                    self.min_asset,
                    self.min_transaction,
                )
            })
    }

    /// Calculate the borrow amount.
    /// Return 'error::PositionError::InsufficientTransactionAmount' when either the downpayment
    /// or the borrow amount is less than the minimum transaction amount.
    /// Return 'error::PositionError::InsufficientAssetAmount' when the lease (downpayment + borrow)
    /// is less than the minimum asset amount.
    pub fn calc_borrow_amount(
        &self,
        downpayment: LpnCoin,
        may_max_ltd: Option<Percent>,
    ) -> PositionResult<LpnCoin> {
        let one = Price::identity();

        if !self.valid_transaction(downpayment, one) {
            Err(PositionError::InsufficientTransactionAmount(
                self.min_transaction.into(),
            ))
        } else {
            let borrow = self.liability.init_borrow_amount(downpayment, may_max_ltd);
            if !self.valid_transaction(borrow, one) {
                Err(PositionError::InsufficientTransactionAmount(
                    self.min_transaction.into(),
                ))
            } else if !self.valid_asset(downpayment.add(borrow), one) {
                Err(PositionError::InsufficientAssetAmount(
                    self.min_asset.into(),
                ))
            } else {
                Ok(borrow)
            }
        }
    }

    pub fn overdue_collection_in<Due>(&self, due: &Due) -> Duration
    where
        Due: DueTrait,
    {
        self.overdue_collection(due).start_in()
    }

    /// Determine the debt status of a position
    ///
    /// Pre: `self.check_close(...) == None`
    pub fn debt<Asset, Due>(
        &self,
        asset: Coin<Asset>,
        due: &Due,
        asset_in_lpns: Price<Asset>,
    ) -> Debt<Asset>
    where
        Asset: Currency,
        Due: DueTrait,
    {
        debug_assert_eq!(None, self.check_close(asset, due, asset_in_lpns));
        let total_due = Self::to_assets(due.total_due(), asset_in_lpns);

        self.may_ask_liquidation_liability(asset, total_due, asset_in_lpns)
            .max(self.may_ask_liquidation_overdue(asset, due, asset_in_lpns))
            .map(Debt::Bad)
            .unwrap_or_else(|| {
                let asset_ltv = Percent::from_ratio(total_due, asset);
                // The ltv can be above the max percent and due to other circumstances the liquidation may not happen
                self.no_liquidation(asset, due, asset_ltv.min(self.liability.third_liq_warn()))
            })
    }

    /// Check if the position is subject of a full close due to trigerred close policy
    pub fn check_close<Asset, Due>(
        &self,
        asset: Coin<Asset>,
        due: &Due,
        asset_in_lpns: Price<Asset>,
    ) -> Option<CloseStrategy>
    where
        Asset: Currency,
        Due: DueTrait,
    {
        self.close
            .may_trigger(asset, Self::to_assets(due.total_due(), asset_in_lpns))
    }

    /// Check if the amount can be used for repayment.
    /// Return `error::PositionError::InsufficientTransactionAmount` when the payment amount
    /// is less than the minimum transaction amount.
    pub fn validate_payment<PaymentC>(
        &self,
        payment: Coin<PaymentC>,
        payment_currency_in_lpns: Price<PaymentC>,
    ) -> PositionResult<()>
    where
        PaymentC: CurrencyDef,
        PaymentC::Group: MemberOf<LeasePaymentCurrencies>,
    {
        if self.valid_transaction(payment, payment_currency_in_lpns) {
            Ok(())
        } else {
            Err(PositionError::InsufficientTransactionAmount(
                self.min_transaction.into(),
            ))
        }
    }

    /// Check if the amount can be used to close the position.
    /// Return `error::PositionError::PositionCloseAmountTooSmall` when a partial close is requested
    /// with amount less than the minimum transaction position parameter sent on lease open. Refer to
    /// `NewLeaseForm::position_spec`.
    ///
    /// Return `error::PositionError::PositionCloseAmountTooBig` when a partial close is requested
    /// with amount that would decrease a position less than the minimum asset parameter sent on
    /// lease open. Refer to `NewLeaseForm::position_spec`.
    pub fn validate_close_amount<Asset>(
        &self,
        asset: Coin<Asset>,
        close_amount: Coin<Asset>,
        asset_in_lpns: Price<Asset>,
    ) -> PositionResult<()>
    where
        Asset: Currency,
    {
        if self.valid_transaction(close_amount, asset_in_lpns) {
            if self.valid_asset(asset.saturating_sub(close_amount), asset_in_lpns) {
                Ok(())
            } else {
                Err(PositionError::PositionCloseAmountTooBig(
                    self.min_asset.into(),
                ))
            }
        } else {
            Err(PositionError::PositionCloseAmountTooSmall(
                self.min_transaction.into(),
            ))
        }
    }

    fn valid_transaction<TransactionC>(
        &self,
        amount: Coin<TransactionC>,
        transaction_currency_in_lpn: Price<TransactionC>,
    ) -> bool
    where
        TransactionC: Currency,
    {
        let amount = price::total(amount, transaction_currency_in_lpn);

        amount >= self.min_transaction
    }

    fn valid_asset<TransactionC>(
        &self,
        asset_amount: Coin<TransactionC>,
        transaction_currency_in_lpn: Price<TransactionC>,
    ) -> bool
    where
        TransactionC: Currency,
    {
        let asset_amount = price::total(asset_amount, transaction_currency_in_lpn);

        asset_amount >= self.min_asset
    }

    fn may_ask_liquidation_liability<Asset>(
        &self,
        asset: Coin<Asset>,
        total_due: Coin<Asset>,
        asset_in_lpns: Price<Asset>,
    ) -> Option<Liquidation<Asset>>
    where
        Asset: Currency,
    {
        let liquidation_amount = self.liability.amount_to_liquidate(asset, total_due);
        self.may_ask_liquidation(
            asset,
            Cause::Liability {
                ltv: self.liability.max(),
                healthy_ltv: self.liability.healthy_percent(),
            },
            liquidation_amount,
            asset_in_lpns,
        )
    }

    fn may_ask_liquidation_overdue<Asset, Due>(
        &self,
        asset: Coin<Asset>,
        due: &Due,
        asset_in_lpns: Price<Asset>,
    ) -> Option<Liquidation<Asset>>
    where
        Asset: Currency,
        Due: DueTrait,
    {
        let collectable = self.overdue_collection(due).amount();
        debug_assert!(collectable <= due.total_due());
        let to_liquidate = Self::to_assets(collectable, asset_in_lpns);
        self.may_ask_liquidation(asset, Cause::Overdue(), to_liquidate, asset_in_lpns)
    }

    fn may_ask_liquidation<Asset>(
        &self,
        asset: Coin<Asset>,
        cause: Cause,
        liquidation: Coin<Asset>,
        asset_in_lpns: Price<Asset>,
    ) -> Option<Liquidation<Asset>>
    where
        Asset: Currency,
    {
        match self.validate_close_amount(asset, liquidation, asset_in_lpns) {
            Err(PositionError::PositionCloseAmountTooSmall(_)) => None,
            Err(PositionError::PositionCloseAmountTooBig(_)) => Some(Liquidation::Full(cause)),
            Err(_) => unreachable!(), // TODO extract the two PositionError variants to a dedicated type to avoid this match arm
            Ok(()) => {
                debug_assert!(liquidation < asset);
                Some(Liquidation::Partial {
                    amount: liquidation,
                    cause,
                })
            }
        }
    }

    fn no_liquidation<Asset, Due>(
        &self,
        asset: Coin<Asset>,
        due: &Due,
        asset_ltv: Percent,
    ) -> Debt<Asset>
    where
        Asset: Currency,
        Due: DueTrait,
    {
        debug_assert!(asset_ltv < self.liability.max());
        if due.total_due().is_zero() {
            Debt::No
        } else {
            let zone = self.liability.zone_of(asset_ltv);
            debug_assert!(zone.range().contains(&asset_ltv));
            let steady_within = self.close.no_close(zone.range());
            debug_assert!(steady_within.contains(&asset_ltv));
            Debt::Ok {
                zone,
                steadiness: Steadiness::new(
                    self.overdue_collection_in(due)
                        .min(self.liability.recalculation_time()),
                    steady_within.invert(|ltv| {
                        debug_assert!(!ltv.is_zero());
                        price::total_of(ltv.of(asset)).is(due.total_due())
                    }),
                ),
            }
        }
    }

    fn overdue_collection<Due>(&self, due: &Due) -> OverdueCollection
    where
        Due: DueTrait,
    {
        due.overdue_collection(self.min_transaction)
    }

    fn to_assets<Asset>(lpn_coin: LpnCoin, asset_in_lpns: Price<Asset>) -> Coin<Asset>
    where
        Asset: Currency,
    {
        price::total(lpn_coin, asset_in_lpns.inv())
    }
}
