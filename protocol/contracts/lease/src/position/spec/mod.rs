use std::ops::Add;

use currency::Currency;
use finance::{
    coin::Coin,
    liability::Liability,
    percent::Percent,
    price::{self, Price},
};

use crate::{
    error::{ContractError, ContractResult},
    position::{Cause, Debt, Liquidation},
};

mod dto;

#[cfg_attr(test, derive(Debug))]
pub struct Spec<Lpn> {
    liability: Liability,
    min_asset: Coin<Lpn>,
    min_transaction: Coin<Lpn>,
}

impl<Lpn> Spec<Lpn>
where
    Lpn: Currency,
{
    pub fn new(liability: Liability, min_asset: Coin<Lpn>, min_transaction: Coin<Lpn>) -> Self {
        let obj = Self {
            liability,
            min_asset,
            min_transaction,
        };
        debug_assert_eq!(Ok(()), obj.invariant_held());
        obj
    }

    /// Calculate the borrow amount.
    /// Return 'error::ContractError::InsufficientTransactionAmount' when either the downpayment
    /// or the borrow amount is less than the minimum transaction amount.
    /// Return 'error::ContractError::InsufficientAssetAmount' when the lease (downpayment + borrow)
    /// is less than the minimum asset amount.
    pub fn calc_borrow_amount(
        &self,
        downpayment: Coin<Lpn>,
        may_max_ltd: Option<Percent>,
    ) -> ContractResult<Coin<Lpn>> {
        let one = Price::identity();

        if !self.valid_transaction(downpayment, one) {
            Err(ContractError::InsufficientTransactionAmount(
                self.min_transaction.into(),
            ))
        } else {
            let borrow = self.liability.init_borrow_amount(downpayment, may_max_ltd);
            if !self.valid_transaction(borrow, one) {
                Err(ContractError::InsufficientTransactionAmount(
                    self.min_transaction.into(),
                ))
            } else if !self.valid_asset(downpayment.add(borrow), one) {
                Err(ContractError::InsufficientAssetAmount(
                    self.min_asset.into(),
                ))
            } else {
                Ok(borrow)
            }
        }
    }

    pub fn debt<Asset>(
        &self,
        asset: Coin<Asset>,
        total_due: Coin<Lpn>,
        overdue: Coin<Lpn>,
        asset_in_lpns: Price<Asset, Lpn>,
    ) -> Debt<Asset>
    where
        Asset: Currency,
    {
        debug_assert!(overdue <= total_due);

        let total_due = price::total(total_due, asset_in_lpns.inv());
        let overdue = price::total(overdue, asset_in_lpns.inv());
        debug_assert!(overdue <= total_due);

        self.may_ask_liquidation_liability(asset, total_due, asset_in_lpns)
            .max(self.may_ask_liquidation_overdue(asset, overdue, asset_in_lpns))
            .map(Debt::Bad)
            .unwrap_or_else(|| {
                let ltv = Percent::from_ratio(total_due, asset);
                // The ltv can be above the max percent and due to other circumstances the liquidation may not happen
                self.no_liquidation(total_due, ltv.min(self.liability.third_liq_warn()))
            })
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
        if self.valid_transaction(payment, payment_currency_in_lpns) {
            Ok(())
        } else {
            Err(ContractError::InsufficientPayment(
                price::total(self.min_transaction, payment_currency_in_lpns.inv()).into(),
            ))
        }
    }

    /// Check if the amount can be used to close the position.
    /// Return `error::ContractError::PositionCloseAmountTooSmall` when a partial close is requested
    /// with amount less than the minimum transaction position parameter sent on lease open. Refer to
    /// `NewLeaseForm::position_spec`.
    ///
    /// Return `error::ContractError::PositionCloseAmountTooBig` when a partial close is requested
    /// with amount that would decrease a position less than the minimum asset parameter sent on
    /// lease open. Refer to `NewLeaseForm::position_spec`.
    pub fn validate_close_amount<Asset>(
        &self,
        asset: Coin<Asset>,
        close_amount: Coin<Asset>,
        asset_in_lpns: Price<Asset, Lpn>,
    ) -> ContractResult<()>
    where
        Asset: Currency,
    {
        if self.valid_transaction(close_amount, asset_in_lpns) {
            if self.valid_asset(asset.saturating_sub(close_amount), asset_in_lpns) {
                Ok(())
            } else {
                Err(ContractError::PositionCloseAmountTooBig(
                    self.min_asset.into(),
                ))
            }
        } else {
            Err(ContractError::PositionCloseAmountTooSmall(
                self.min_transaction.into(),
            ))
        }
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(
            !self.min_asset.is_zero(),
            "Min asset amount should be positive",
        )
        .and(Self::check(
            !self.min_transaction.is_zero(),
            "Min transaction amount should be positive",
        ))
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }

    fn valid_transaction<TransactionC>(
        &self,
        amount: Coin<TransactionC>,
        transaction_currency_in_lpn: Price<TransactionC, Lpn>,
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
        transaction_currency_in_lpn: Price<TransactionC, Lpn>,
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
        asset_in_lpns: Price<Asset, Lpn>,
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

    fn may_ask_liquidation_overdue<Asset>(
        &self,
        asset: Coin<Asset>,
        overdue: Coin<Asset>,
        asset_in_lpns: Price<Asset, Lpn>,
    ) -> Option<Liquidation<Asset>>
    where
        Asset: Currency,
    {
        self.may_ask_liquidation(asset, Cause::Overdue(), overdue, asset_in_lpns)
    }

    fn may_ask_liquidation<Asset>(
        &self,
        asset: Coin<Asset>,
        cause: Cause,
        liquidation: Coin<Asset>,
        asset_in_lpns: Price<Asset, Lpn>,
    ) -> Option<Liquidation<Asset>>
    where
        Asset: Currency,
    {
        match self.validate_close_amount(asset, liquidation, asset_in_lpns) {
            Err(ContractError::PositionCloseAmountTooSmall(_)) => None,
            Err(ContractError::PositionCloseAmountTooBig(_)) => Some(Liquidation::Full(cause)),
            Err(_) => unreachable!(),
            Ok(()) => {
                debug_assert!(liquidation < asset);
                Some(Liquidation::Partial {
                    amount: liquidation,
                    cause,
                })
            }
        }
    }

    fn no_liquidation<Asset>(&self, total_due: Coin<Asset>, ltv: Percent) -> Debt<Asset>
    where
        Asset: Currency,
    {
        debug_assert!(ltv < self.liability.max());
        if total_due.is_zero() {
            Debt::No
        } else {
            Debt::Ok {
                zone: self.liability.zone_of(ltv),
                recalc_in: self.liability.recalculation_time(),
            }
        }
    }
}

#[cfg(test)]
mod test_calc_borrow {
    use currencies::test::StableC1;
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        liability::Liability,
        percent::Percent,
    };

    use crate::error::ContractError;

    use super::Spec;

    type TestLpn = StableC1;

    #[test]
    fn downpayment_less_than_min() {
        let spec = spec(560, 300);

        let downpayment_less = spec.calc_borrow_amount(299.into(), None);
        assert!(matches!(
            downpayment_less,
            Err(ContractError::InsufficientTransactionAmount(_))
        ));

        let borrow = spec.calc_borrow_amount(300.into(), None);
        assert_eq!(coin_lpn(557), borrow.unwrap());
    }

    #[test]
    fn borrow_less_than_min() {
        let spec = spec(600, 300);

        let borrow_less = spec.calc_borrow_amount(300.into(), Some(Percent::from_percent(99)));
        assert!(matches!(
            borrow_less,
            Err(ContractError::InsufficientTransactionAmount(_))
        ));

        let borrow = spec.calc_borrow_amount(300.into(), Some(Percent::from_percent(100)));
        assert_eq!(coin_lpn(300), borrow.unwrap());
    }

    #[test]
    fn lease_less_than_min() {
        let spec = spec(1_000, 300);

        let borrow_1 = spec.calc_borrow_amount(349.into(), None);
        assert!(matches!(
            borrow_1,
            Err(ContractError::InsufficientAssetAmount(_))
        ));

        let borrow_2 = spec.calc_borrow_amount(350.into(), None);
        assert_eq!(coin_lpn(650), borrow_2.unwrap());

        let borrow_3 = spec.calc_borrow_amount(550.into(), Some(Percent::from_percent(81)));
        assert!(matches!(
            borrow_3,
            Err(ContractError::InsufficientAssetAmount(_))
        ));

        let borrow_3 = spec.calc_borrow_amount(550.into(), Some(Percent::from_percent(82)));
        assert_eq!(coin_lpn(451), borrow_3.unwrap());
    }

    #[test]
    fn valid_borrow_amount() {
        let spec = spec(1_000, 300);

        let borrow_1 = spec.calc_borrow_amount(540.into(), None);
        assert_eq!(coin_lpn(1002), borrow_1.unwrap());

        let borrow_2 = spec.calc_borrow_amount(870.into(), Some(Percent::from_percent(100)));
        assert_eq!(coin_lpn(870), borrow_2.unwrap());

        let borrow_3 = spec.calc_borrow_amount(650.into(), Some(Percent::from_percent(150)));
        assert_eq!(coin_lpn(975), borrow_3.unwrap());
    }

    fn spec<LpnAmount>(min_asset: LpnAmount, min_transaction: LpnAmount) -> Spec<TestLpn>
    where
        LpnAmount: Into<Coin<TestLpn>>,
    {
        let liability = Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(70),
            Percent::from_percent(73),
            Percent::from_percent(75),
            Percent::from_percent(78),
            Percent::from_percent(80),
            Duration::from_hours(1),
        );
        Spec::new(liability, min_asset.into(), min_transaction.into())
    }

    fn coin_lpn(amount: Amount) -> Coin<TestLpn> {
        Coin::<TestLpn>::new(amount)
    }
}

#[cfg(test)]
mod test_check_liability {

    use currencies::test::{PaymentC3, StableC1};
    use finance::{
        coin::Coin,
        duration::Duration,
        liability::{Liability, Zone},
        percent::Percent,
        price::{self, Price},
    };

    use crate::position::{Cause, Debt, Position};

    use super::Spec;

    type TestCurrency = PaymentC3;
    type TestLpn = StableC1;

    const RECALC_IN: Duration = Duration::from_hours(1);

    #[test]
    fn no_debt() {
        let warn_ltv = Percent::from_permille(11);
        let spec = position_with_first(warn_ltv, 100, 1, 1);
        assert_eq!(spec.debt(0.into(), 0.into(), price(1, 1)), Debt::No,);
        assert_eq!(spec.debt(0.into(), 0.into(), price(3, 1)), Debt::No,);
    }

    #[test]
    fn warnings_none_zero_liq() {
        let warn_ltv = Percent::from_percent(51);
        let position = position_with_first(warn_ltv, 100, 1, 1);

        assert_eq!(
            position.debt(1.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(1.into(), 0.into(), price(5, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(50.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(25.into(), 0.into(), price(2, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(51.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(17.into(), 0.into(), price(3, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
    }

    #[test]
    fn warnings_none_min_transaction() {
        let warn_ltv = Percent::from_percent(51);
        let position = position_with_first(warn_ltv, 100, 1, 15);

        assert_eq!(
            position.debt(50.into(), 14.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(25.into(), 4.into(), price(2, 3)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(51.into(), 14.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(17.into(), 4.into(), price(3, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
    }

    #[test]
    fn warnings_first() {
        let warn_ltv = Percent::from_permille(712);
        let position = position_with_first(warn_ltv, 1000, 10, 1);

        assert_eq!(
            position.debt(711.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(237.into(), 0.into(), price(3, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(712.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(178.into(), 0.into(), price(4, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(712.into(), 1.into(), price(1, 1)),
            Debt::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(89.into(), 1.into(), price(8, 1)),
            Debt::partial(8.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(721.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(103.into(), 0.into(), price(7, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(722.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv + STEP, warn_ltv + STEP + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(361.into(), 0.into(), price(2, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv + STEP, warn_ltv + STEP + STEP),
                recalc_in: RECALC_IN
            },
        );
    }

    #[test]
    fn warnings_first_min_transaction() {
        let warn_ltv = Percent::from_permille(712);
        let position = position_with_first(warn_ltv, 1000, 10, 3);

        assert_eq!(
            position.debt(712.into(), 2.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(356.into(), 1.into(), price(2, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(721.into(), 2.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(721.into(), 5.into(), price(1, 1)),
            Debt::partial(5.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(240.into(), 3.into(), price(3, 1)),
            Debt::partial(9.into(), Cause::Overdue()),
        );
    }

    #[test]
    fn warnings_second() {
        let warn_ltv = Percent::from_permille(123);
        let position = position_with_second(warn_ltv, 1000, 10, 1);

        assert_eq!(
            position.debt(122.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv - STEP, warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(15.into(), 0.into(), price(8, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv - STEP, warn_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(123.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(82.into(), 0.into(), price(3, 2)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(123.into(), 4.into(), price(1, 1)),
            Debt::partial(4.into(), Cause::Overdue())
        );
        assert_eq!(
            position.debt(132.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(66.into(), 0.into(), price(2, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(133.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_ltv + STEP, warn_ltv + STEP + STEP),
                recalc_in: RECALC_IN
            },
        );
    }

    #[test]
    fn warnings_second_min_transaction() {
        let warn_ltv = Percent::from_permille(123);
        let position = position_with_second(warn_ltv, 1000, 10, 5);

        assert_eq!(
            position.debt(128.into(), 4.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(32.into(), 1.into(), price(4, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(128.into(), 5.into(), price(1, 1)),
            Debt::partial(5.into(), Cause::Overdue())
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let position = position_with_third(warn_third_ltv, 1000, 100, 1);

        assert_eq!(
            position.debt(380.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(190.into(), 0.into(), price(2, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(381.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(381.into(), 375.into(), price(1, 1)),
            Debt::partial(375.into(), Cause::Overdue())
        );
        assert_eq!(
            position.debt(573.into(), 562.into(), price(2, 3)),
            Debt::partial(374.into(), Cause::Overdue())
        );
        assert_eq!(
            position.debt(390.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(391.into(), 0.into(), price(1, 1)),
            Debt::partial(
                384.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn warnings_third_min_transaction() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let position = position_with_third(warn_third_ltv, 1000, 100, 386);

        assert_eq!(
            position.debt(380.into(), 1.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(126.into(), 1.into(), price(3, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(381.into(), 375.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(391.into(), 385.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(391.into(), 386.into(), price(1, 1)),
            Debt::partial(386.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(392.into(), 0.into(), price(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(364.into(), 0.into(), price(2, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                recalc_in: RECALC_IN
            },
        );
        assert_eq!(
            position.debt(393.into(), 0.into(), price(1, 1)),
            Debt::partial(
                386.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.debt(788.into(), 0.into(), price(1, 2)),
            Debt::partial(
                387.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn liquidate_partial() {
        let max_ltv = Percent::from_permille(881);
        let position = position_with_max(max_ltv, 1000, 100, 1);

        assert_eq!(
            position.debt(880.into(), 1.into(), price(1, 1)),
            Debt::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(139.into(), 1.into(), price(4, 1)),
            Debt::partial(4.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(881.into(), 879.into(), price(1, 1)),
            Debt::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.debt(881.into(), 880.into(), price(1, 1)),
            Debt::partial(880.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(294.into(), 294.into(), price(1, 3)),
            Debt::partial(98.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(294.into(), 293.into(), price(3, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            position.debt(1000.into(), 1.into(), price(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_partial_min_asset() {
        let max_ltv = Percent::from_permille(881);
        let position = position_with_max(max_ltv, 1000, 100, 1);

        assert_eq!(
            position.debt(900.into(), 897.into(), price(1, 1)),
            Debt::partial(
                898.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.debt(900.into(), 899.into(), price(1, 1)),
            Debt::partial(899.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(233.into(), 233.into(), price(3, 1)),
            Debt::partial(699.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(901.into(), 889.into(), price(1, 1)),
            Debt::partial(
                900.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.debt(902.into(), 889.into(), price(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full() {
        let max_ltv = Percent::from_permille(768);
        let position = position_with_max(max_ltv, 1000, 230, 1);

        assert_eq!(
            position.debt(768.into(), 765.into(), price(1, 1)),
            Debt::partial(
                765.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.debt(1560.into(), 1552.into(), price(1, 2)),
            Debt::partial(
                777.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.debt(768.into(), 768.into(), price(1, 1)),
            Debt::partial(768.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(1560.into(), 1556.into(), price(1, 2)),
            Debt::partial(778.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(788.into(), 768.into(), price(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full_liability() {
        let max_ltv = Percent::from_permille(673);
        let position = position_with_max(max_ltv, 1000, 120, 15);

        assert_eq!(
            position.debt(882.into(), 1.into(), price(1, 1)),
            Debt::partial(
                880.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.debt(883.into(), 1.into(), price(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            position.debt(294.into(), 1.into(), price(3, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            position.debt(1000.into(), 1.into(), price(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full_overdue() {
        let max_ltv = Percent::from_permille(773);
        let position = position_with_max(max_ltv, 1000, 326, 15);

        assert_eq!(
            position.debt(772.into(), 674.into(), price(1, 1)),
            Debt::partial(674.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(1674.into(), 1674.into(), price(1, 2)),
            Debt::partial(837.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.debt(772.into(), 675.into(), price(1, 1)),
            Debt::full(Cause::Overdue()),
        );
        assert_eq!(
            position.debt(1676.into(), 1676.into(), price(1, 2)),
            Debt::full(Cause::Overdue()),
        );
    }

    const STEP: Percent = Percent::from_permille(10);

    fn price<Asset, Lpn>(price_asset: Asset, price_lpn: Lpn) -> Price<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        price::total_of(price_asset.into()).is(price_lpn.into())
    }

    fn position_with_first<Asset, Lpn>(
        warn: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_transaction: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        position_with_max(warn + STEP + STEP + STEP, asset, min_asset, min_transaction)
    }

    fn position_with_second<Asset, Lpn>(
        warn: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_transaction: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        position_with_max(warn + STEP + STEP, asset, min_asset, min_transaction)
    }

    fn position_with_third<Asset, Lpn>(
        warn: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_transaction: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        position_with_max(warn + STEP, asset, min_asset, min_transaction)
    }

    // init = 1%, healthy = 1%, first = max - 3, second = max - 2, third = max - 1
    fn position_with_max<Asset, Lpn>(
        max: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_transaction: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        let initial = STEP;
        assert!(initial < max - STEP - STEP - STEP);

        let healthy = initial + Percent::ZERO;
        let max = healthy + max - initial;
        let third_liquidity_warning = max - STEP;
        let second_liquidity_warning = third_liquidity_warning - STEP;
        let first_liquidity_warning = second_liquidity_warning - STEP;

        let liability = Liability::new(
            initial,
            healthy,
            first_liquidity_warning,
            second_liquidity_warning,
            third_liquidity_warning,
            max,
            RECALC_IN,
        );
        let spec = Spec::new(liability, min_asset.into(), min_transaction.into());

        Position::new(asset.into(), spec)
    }
}

#[cfg(test)]
mod test_validate_payment {
    use currencies::test::{LeaseC1, StableC1};
    use finance::{
        coin::Coin,
        duration::Duration,
        liability::Liability,
        percent::Percent,
        price::{self, Price},
    };

    use crate::error::ContractError;

    use super::Spec;

    type TestLpn = StableC1;
    type TestPaymentC = LeaseC1;

    #[test]
    fn insufficient_payment() {
        let spec = spec(65, 16);
        let result_1 = spec.validate_payment(15.into(), price(1, 1));
        assert!(matches!(
            result_1,
            Err(ContractError::InsufficientPayment(_))
        ));
        let result_2 = spec.validate_payment(16.into(), price(1, 1));
        assert!(result_2.is_ok());

        let result_3 = spec.validate_payment(45.into(), price(3, 1));
        assert!(matches!(
            result_3,
            Err(ContractError::InsufficientPayment(_))
        ));
        let result_4 = spec.validate_payment(8.into(), price(1, 2));
        assert!(result_4.is_ok());
    }

    fn spec<LpnAmount>(min_asset: LpnAmount, min_transaction: LpnAmount) -> Spec<TestLpn>
    where
        LpnAmount: Into<Coin<TestLpn>>,
    {
        let liability = Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(70),
            Percent::from_percent(73),
            Percent::from_percent(75),
            Percent::from_percent(78),
            Percent::from_percent(80),
            Duration::from_hours(1),
        );
        Spec::new(liability, min_asset.into(), min_transaction.into())
    }

    fn price<PaymentC, Lpn>(
        price_payment_currency: PaymentC,
        price_lpn: Lpn,
    ) -> Price<TestPaymentC, TestLpn>
    where
        PaymentC: Into<Coin<TestPaymentC>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        price::total_of(price_payment_currency.into()).is(price_lpn.into())
    }
}

#[cfg(test)]
mod test_validate_close {
    use currencies::test::{PaymentC3, StableC1};
    use finance::{
        coin::Coin,
        duration::Duration,
        liability::Liability,
        percent::Percent,
        price::{self, Price},
    };

    use crate::{
        error::ContractError,
        position::{Position, Spec},
    };

    type TestCurrency = PaymentC3;
    type TestLpn = StableC1;

    #[test]
    fn too_small_amount() {
        let spec = position(100, 75, 15);
        let result_1 = spec.validate_close_amount(14.into(), price(1, 1));
        assert!(matches!(
            result_1,
            Err(ContractError::PositionCloseAmountTooSmall(_))
        ));

        let result_2 = spec.validate_close_amount(6.into(), price(1, 2));
        assert!(matches!(
            result_2,
            Err(ContractError::PositionCloseAmountTooSmall(_))
        ));
    }

    #[test]
    fn amount_as_min_transaction() {
        let spec = position(100, 85, 15);
        let result_1 = spec.validate_close_amount(15.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(5.into(), price(1, 3));
        assert!(result_2.is_ok());
    }

    #[test]
    fn too_big_amount() {
        let spec = position(100, 25, 1);
        let result_1 = spec.validate_close_amount(76.into(), price(1, 1));
        assert!(matches!(
            result_1,
            Err(ContractError::PositionCloseAmountTooBig(_))
        ));

        let result_2 = spec.validate_close_amount(64.into(), price(3, 2));
        assert!(matches!(
            result_2,
            Err(ContractError::PositionCloseAmountTooBig(_))
        ));
    }

    #[test]
    fn amount_as_min_asset() {
        let spec = position(100, 25, 1);
        let result_1 = spec.validate_close_amount(75.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(62.into(), price(3, 2));
        assert!(result_2.is_ok());
    }

    #[test]
    fn valid_amount() {
        let spec = position(100, 40, 10);
        let result_1 = spec.validate_close_amount(53.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(89.into(), price(1, 4));
        assert!(result_2.is_ok());
    }

    fn position<Asset, Lpn>(
        amount: Asset,
        min_asset: Lpn,
        min_transaction: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        let liability = Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(70),
            Percent::from_percent(73),
            Percent::from_percent(75),
            Percent::from_percent(78),
            Percent::from_percent(80),
            Duration::from_hours(1),
        );
        let spec = Spec::<TestLpn>::new(liability, min_asset.into(), min_transaction.into());

        Position::<TestCurrency, TestLpn>::new(amount.into(), spec)
    }

    fn price<Asset, Lpn>(price_asset: Asset, price_lpn: Lpn) -> Price<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        price::total_of(price_asset.into()).is(price_lpn.into())
    }
}
