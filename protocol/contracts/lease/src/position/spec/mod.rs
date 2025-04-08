use std::{
    fmt::Debug,
    ops::{Add, Div, Rem},
};

use currency::{Currency, CurrencyDef, MemberOf};
use finance::{
    coin::Coin,
    duration::Duration,
    fraction::Fraction,
    fractionable::Fractionable,
    liability::Liability,
    percent::{Percent, Percent100},
    price::{self},
    ratio::CheckedMul,
    zero::Zero,
};

use crate::{
    api::{
        LeasePaymentCurrencies, position::ClosePolicyChange,
        query::opened::ClosePolicy as APIClosePolicy,
    },
    finance::{LpnCoin, LpnCoinDTO, Price},
};

use super::{
    Cause, CloseStrategy, Debt, DueTrait, Liquidation, PositionError, PositionResult,
    close::Policy as ClosePolicy, interest::OverdueCollection, steady::Steadiness,
};
pub use dto::SpecDTO;

mod dto;

#[cfg(all(feature = "internal.test.contract", test))]
mod test;

#[derive(Clone, Copy)]
#[cfg_attr(feature = "contract_testing", derive(Debug, PartialEq))]
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
        debug_assert!(close.liquidation_check(liability.max()).is_ok());
        Self {
            liability,
            close,
            min_asset,
            min_transaction,
        }
    }

    #[cfg(all(feature = "internal.test.contract", test))]
    pub(crate) fn no_close(
        liability: Liability,
        min_asset: LpnCoin,
        min_transaction: LpnCoin,
    ) -> Self {
        Self::new(
            liability,
            ClosePolicy::default(),
            min_asset,
            min_transaction,
        )
    }

    pub fn close_policy(&self) -> APIClosePolicy {
        self.close.into()
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
        let total_due = Self::to_assets(due.total_due(), asset_in_lpns);

        self.close
            .change_policy(cmd)
            .and_then(|close_policy| close_policy.liquidation_check(self.liability.max()))
            .and_then(|close_policy| {
                close_policy.may_trigger(asset, total_due).map_or_else(
                    || Ok(close_policy),
                    |strategy| {
                        Err(PositionError::trigger_close(
                            Self::ltv(total_due, asset),
                            strategy,
                        ))
                    },
                )
            })
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
    /// Return [`PositionError::InsufficientTransactionAmount`] when either the downpayment
    /// or the borrow amount is less than the minimum transaction amount.
    /// Return [`PositionError::InsufficientAssetAmount`] when the lease (downpayment + borrow)
    /// is less than the minimum asset amount.
    pub fn calc_borrow_amount(
        &self,
        downpayment: LpnCoin,
        may_max_ltd: Option<Percent>,
    ) -> PositionResult<LpnCoin> {
        let one = Price::identity();

        self.validate_transaction(
            downpayment,
            one,
            PositionError::InsufficientTransactionAmount,
        )
        .and_then(|()| {
            let borrow = self
                .liability
                .init_borrow_amount(downpayment, may_max_ltd)
                .expect("TODO: propagate up the stack potential overflow");
            self.validate_transaction(borrow, one, PositionError::InsufficientTransactionAmount)
                .and_then(|()| {
                    self.validate_asset(
                        downpayment.add(borrow),
                        one,
                        PositionError::InsufficientAssetAmount,
                    )
                })
                .map(|()| borrow)
        })
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
        let due_assets = Self::to_assets(due.total_due(), asset_in_lpns);

        self.may_ask_liquidation_liability(asset, due_assets, asset_in_lpns)
            .max(self.may_ask_liquidation_overdue(asset, due, asset_in_lpns))
            .map(Debt::Bad)
            .unwrap_or_else(|| {
                let position_ltv = Self::ltv(due_assets, asset);
                // The ltv can be above the max percent and due to other circumstances the liquidation may not happen,
                // for example, the liquidated amount is less than the `min_transaction_amount`
                let position_ltv_capped = self.liability.cap_to_zone(position_ltv);
                let due_assets_capped = if position_ltv_capped < position_ltv {
                    self.liability.max().of(asset) - Coin::new(1)
                } else {
                    due_assets
                };

                self.no_liquidation(asset, due, due_assets_capped, position_ltv_capped)
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
        self.validate_transaction(
            payment,
            payment_currency_in_lpns,
            PositionError::InsufficientTransactionAmount,
        )
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
        self.validate_transaction(
            close_amount,
            asset_in_lpns,
            PositionError::PositionCloseAmountTooSmall,
        )
        .and_then(|()| {
            self.validate_asset(
                asset.saturating_sub(close_amount),
                asset_in_lpns,
                PositionError::PositionCloseAmountTooBig,
            )
        })
    }

    fn validate_transaction<TransactionC, ErrFn>(
        &self,
        amount: Coin<TransactionC>,
        transaction_currency_in_lpn: Price<TransactionC>,
        err_fn: ErrFn,
    ) -> Result<(), PositionError>
    where
        TransactionC: Currency,
        ErrFn: FnOnce(LpnCoinDTO) -> PositionError,
    {
        let amount = price::total(amount, transaction_currency_in_lpn)
            .expect("TODO: propagate up the stack potential overflow");

        if amount >= self.min_transaction {
            Ok(())
        } else {
            Err(err_fn(self.min_transaction.into()))
        }
    }

    fn validate_asset<TransactionC, ErrFn>(
        &self,
        asset_amount: Coin<TransactionC>,
        transaction_currency_in_lpn: Price<TransactionC>,
        err_fn: ErrFn,
    ) -> Result<(), PositionError>
    where
        TransactionC: Currency,
        ErrFn: FnOnce(LpnCoinDTO) -> PositionError,
    {
        let asset_amount = price::total(asset_amount, transaction_currency_in_lpn)
            .expect("TODO: propagate up the stack potential overflow");

        if asset_amount >= self.min_asset {
            Ok(())
        } else {
            Err(err_fn(self.min_asset.into()))
        }
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
        let liquidation_amount = self
            .liability
            .amount_to_liquidate(asset, total_due)
            .expect("TODO: propagate up the stack potential overflow");
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
        _due_assets_capped: Coin<Asset>,
        position_ltv_capped: Percent100,
    ) -> Debt<Asset>
    where
        Asset: Currency,
        Due: DueTrait,
    {
        debug_assert!(position_ltv_capped < self.liability.max());
        if due.total_due().is_zero() {
            Debt::No
        } else {
            let zone = self.liability.zone_of(position_ltv_capped);
            debug_assert!(zone.range().contains(&position_ltv_capped));
            let steady_within = self.close.no_close(zone.range());
            #[cfg(debug_assertions)]
            debug_assert!(
                steady_within
                    .map(|ltv| ltv.of(asset))
                    .contains(&_due_assets_capped)
            );

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

    fn ltv<P>(total_due: P, lease_asset: P) -> Percent100
    where
        P: Copy + Debug + Div + Ord + PartialEq + PartialOrd + Rem<Output = P> + Zero,
        <P as Div>::Output: CheckedMul<Percent100, Output = Percent100>,
        Percent100: Fractionable<P>,
    {
        Percent100::from_ratio(total_due, lease_asset).expect("LTV must not exceed 100%")
    }

    fn to_assets<Asset>(lpn_coin: LpnCoin, asset_in_lpns: Price<Asset>) -> Coin<Asset>
    where
        Asset: Currency,
    {
        price::total(lpn_coin, asset_in_lpns.inv())
            .expect("TODO: propagate up the stack potential overflow")
    }
}
