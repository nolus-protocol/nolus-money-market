use std::ops::Sub;

use currency::Currency;
use serde::Serialize;

use crate::{
    coin::Coin,
    duration::Duration,
    fraction::Fraction,
    fractionable::Percentable,
    percent::{Percent, Units},
    price::{self, Price},
    ratio::Rational,
    zero::Zero,
};

pub use self::dto::LiabilityDTO;
pub use self::level::Level;
pub use self::liquidation::{may_ask_liquidation, Cause, Liquidation, Status};
pub use self::zone::Zone;

mod dto;
mod level;
mod liquidation;
mod zone;

#[derive(Copy, Clone, Debug)]
pub struct Liability<Lpn> {
    /// The initial percentage of the amount due versus the locked collateral
    /// initial > 0
    initial: Percent,
    /// The healty percentage of the amount due versus the locked collateral
    /// healthy >= initial
    healthy: Percent,
    /// The percentage above which the first liquidity warning is issued.
    first_liq_warn: Percent,
    /// The percentage above which the second liquidity warning is issued.
    second_liq_warn: Percent,
    /// The percentage above which the third liquidity warning is issued.
    third_liq_warn: Percent,
    /// The maximum percentage of the amount due versus the locked collateral
    /// max > healthy
    max: Percent,
    /// The minimum amount that triggers a liquidation
    min_liquidation: Coin<Lpn>,
    ///  The minimum amount that a lease asset should be evaluated past any partial liquidation. If not, a full liquidation is performed
    min_asset: Coin<Lpn>,
    /// At what time cadence to recalculate the liability
    ///
    /// Limitation: recalc_time >= 1 hour
    recalc_time: Duration,
}

impl<Lpn> Liability<Lpn>
where
    Lpn: Currency + Serialize,
{
    pub fn check<Asset>(
        &self,
        asset: Coin<Asset>,
        total_due: Coin<Lpn>,
        overdue: Coin<Lpn>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Status<Asset>
    where
        Asset: Currency,
    {
        let total_due = price::total(total_due, lpn_in_assets);
        let overdue = price::total(overdue, lpn_in_assets);
        let min_liquidation = price::total(self.min_liquidation, lpn_in_assets);
        let min_asset = price::total(self.min_asset, lpn_in_assets);

        debug_assert!(asset != Coin::ZERO);
        debug_assert!(total_due <= asset);
        debug_assert!(overdue <= total_due);
        let ltv = Percent::from_ratio(total_due, asset);
        self.may_ask_liquidation_liability(asset, total_due, min_liquidation, min_asset)
            .max(may_ask_liquidation_overdue(
                asset,
                overdue,
                min_liquidation,
                min_asset,
            ))
            .unwrap_or_else(|| self.no_liquidation(total_due, ltv.min(self.third_liq_warn())))
    }

    fn no_liquidation<Asset>(&self, total_due: Coin<Asset>, ltv: Percent) -> Status<Asset>
    where
        Asset: Currency,
    {
        debug_assert!(ltv < self.max());
        if total_due.is_zero() {
            Status::NoDebt
        } else {
            Status::No(self.zone_of(ltv))
        }
    }

    fn may_ask_liquidation_liability<Asset>(
        &self,
        asset: Coin<Asset>,
        total_due: Coin<Asset>,
        min_liquidation: Coin<Asset>,
        min_asset: Coin<Asset>,
    ) -> Option<Status<Asset>>
    where
        Asset: Currency,
    {
        let liquidation_amount = self.amount_to_liquidate(asset, total_due);
        may_ask_liquidation(
            asset,
            Cause::Liability {
                ltv: self.max,
                healthy_ltv: self.healthy_percent(),
            },
            liquidation_amount,
            min_liquidation,
            min_asset,
        )
    }

    const fn healthy_percent(&self) -> Percent {
        self.healthy
    }

    pub const fn first_liq_warn(&self) -> Percent {
        self.first_liq_warn
    }

    pub const fn second_liq_warn(&self) -> Percent {
        self.second_liq_warn
    }

    pub const fn third_liq_warn(&self) -> Percent {
        self.third_liq_warn
    }

    pub const fn max(&self) -> Percent {
        self.max
    }

    fn zone_of(&self, ltv: Percent) -> Zone {
        debug_assert!(ltv < self.max, "Ltv >= max is outside any liability zone!");

        if ltv < self.first_liq_warn {
            Zone::no_warnings(self.first_liq_warn)
        } else if ltv < self.second_liq_warn {
            Zone::first(self.first_liq_warn, self.second_liq_warn)
        } else if ltv < self.third_liq_warn {
            Zone::second(self.second_liq_warn, self.third_liq_warn)
        } else {
            Zone::third(self.third_liq_warn, self.max)
        }
    }

    pub const fn recalculation_time(&self) -> Duration {
        self.recalc_time
    }

    pub fn init_borrow_amount<P>(&self, downpayment: P, may_max_ltd: Option<Percent>) -> P
    where
        P: Percentable + Ord + Copy,
    {
        debug_assert!(self.initial > Percent::ZERO);
        debug_assert!(self.initial < Percent::HUNDRED);

        let default_ltd = Rational::new(self.initial, Percent::HUNDRED - self.initial);
        let default_borrow = default_ltd.of(downpayment);
        may_max_ltd
            .map(|max_ltd| max_ltd.of(downpayment))
            .map(|requested_borrow| requested_borrow.min(default_borrow))
            .unwrap_or(default_borrow)
    }

    /// Post-assert: (total_due - amount_to_liquidate) / (lease_amount - amount_to_liquidate) ~= self.healthy_percent(), if total_due < lease_amount.
    /// Otherwise, amount_to_liquidate == total_due
    fn amount_to_liquidate<P>(&self, lease_amount: P, total_due: P) -> P
    where
        P: Percentable + Copy + Ord + Sub<Output = P> + Zero,
    {
        if total_due < self.max.of(lease_amount) {
            return P::ZERO;
        }
        if lease_amount <= total_due {
            return lease_amount;
        }

        // from 'due - liquidation = healthy% of (lease - liquidation)' follows
        // liquidation = 100% / (100% - healthy%) of (due - healthy% of lease)
        let multiplier = Rational::new(Percent::HUNDRED, Percent::HUNDRED - self.healthy_percent());
        let extra_liability_lpn =
            total_due - total_due.min(self.healthy_percent().of(lease_amount));
        Fraction::<Units>::of(&multiplier, extra_liability_lpn)
    }
}

fn may_ask_liquidation_overdue<Asset>(
    asset: Coin<Asset>,
    overdue: Coin<Asset>,
    min_liquidation: Coin<Asset>,
    min_asset: Coin<Asset>,
) -> Option<Status<Asset>>
where
    Asset: Currency,
{
    may_ask_liquidation(asset, Cause::Overdue(), overdue, min_liquidation, min_asset)
}

#[cfg(test)]
mod tests {
    use crate::{
        coin::{Amount, Coin},
        duration::Duration,
        liability::{Cause, LiabilityDTO, Status, Zone},
        percent::Percent,
        price::{self, Price},
    };
    use currency::{lpn::Usdc, test::Dai};

    use super::Liability;

    type TestLpn = Usdc;
    pub type TestCurrency = Dai;

    const MIN_DUE_AMOUNT: Coin<TestLpn> = Coin::new(100);
    const MIN_DUE_AMOUNT_TEST_CURRENCY: Coin<TestCurrency> = Coin::new(100);
    const LEASE_AMOUNT: Coin<TestCurrency> = Coin::new(1000);
    const LEASE_AMOUNT_TEST_LPN: Coin<TestLpn> = Coin::new(1000);

    #[test]
    fn no_debt() {
        let warn_ltv = Percent::from_permille(11);
        let spec = liability_with_first(warn_ltv, MIN_DUE_AMOUNT, 0.into());
        assert_eq!(
            spec.check::<TestCurrency>(100.into(), 0.into(), 0.into(), price()),
            Status::NoDebt,
        );
    }

    #[test]
    fn warnings_none() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv, MIN_DUE_AMOUNT, 0.into());
        assert_eq!(
            spec.check::<TestCurrency>(100.into(), 1.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(100.into(), 49.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(100.into(), 50.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                505.into(),
                MIN_DUE_AMOUNT - 1.into(),
                price()
            ),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 505.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_TEST_CURRENCY, Cause::Overdue()),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 509.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 510.into(), 0.into(), price()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                510.into(),
                MIN_DUE_AMOUNT - 1.into(),
                price()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 510.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_TEST_CURRENCY, Cause::Overdue()),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                515.into(),
                MIN_DUE_AMOUNT - 1.into(),
                price()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
    }

    #[test]
    fn warnings_first() {
        let spec = liability_with_first(Percent::from_permille(712), MIN_DUE_AMOUNT, 0.into());

        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 711.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.first_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 712.into(), 0.into(), price()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                712.into(),
                MIN_DUE_AMOUNT - 1.into(),
                price()
            ),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 712.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_TEST_CURRENCY, Cause::Overdue())
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 715.into(), 0.into(), price()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 721.into(), 0.into(), price()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 721.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_TEST_CURRENCY, Cause::Overdue()),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 722.into(), 0.into(), price()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
    }

    #[test]
    fn warnings_second() {
        let spec = liability_with_second(Percent::from_permille(123), MIN_DUE_AMOUNT, 0.into());

        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 122.into(), 0.into(), price()),
            Status::No(Zone::first(spec.first_liq_warn(), spec.second_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 123.into(), 0.into(), price()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 124.into(), 0.into(), price()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                128.into(),
                MIN_DUE_AMOUNT - 1.into(),
                price()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 128.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_TEST_CURRENCY, Cause::Overdue())
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 132.into(), 0.into(), price()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 133.into(), 0.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = liability_with_third(warn_third_ltv, MIN_DUE_AMOUNT, 0.into());

        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 380.into(), 0.into(), price()),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                380.into(),
                MIN_DUE_AMOUNT - 1.into(),
                price()
            ),
            Status::No(Zone::second(spec.second_liq_warn(), spec.third_liq_warn())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 381.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_TEST_CURRENCY, Cause::Overdue())
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 381.into(), 0.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 382.into(), 0.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 390.into(), 0.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 391.into(), 0.into(), price()),
            Status::partial(
                384.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn min_liquidation() {
        let max_ltv = Percent::from_permille(751);
        let spec = liability_with_max(max_ltv, 1000.into(), 0.into());

        assert_eq!(
            spec.check::<TestCurrency>(878.into(), 752.into(), 0.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(878.into(), 752.into(), 0.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 750.into(), 99.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 751.into(), 99.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 761.into(), 0.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
    }

    #[test]
    fn liquidate_partial() {
        let max_ltv = Percent::from_permille(881);
        const BACK_TO_HEALTHY: Amount = 879;
        let spec = liability_with_max(max_ltv, MIN_DUE_AMOUNT, 0.into());

        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                880.into(),
                MIN_DUE_AMOUNT - 1.into(),
                price()
            ),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 880.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_TEST_CURRENCY, Cause::Overdue()),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 881.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                881.into(),
                (BACK_TO_HEALTHY - 1).into(),
                price()
            ),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 881.into(), BACK_TO_HEALTHY.into(), price()),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                881.into(),
                (BACK_TO_HEALTHY + 1).into(),
                price()
            ),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 999.into(), 997.into(), price()),
            Status::partial(
                998.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 1000.into(), 1.into(), price()),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full() {
        let max_ltv = Percent::from_permille(768);
        let spec = liability_with_max(max_ltv, MIN_DUE_AMOUNT, 0.into());

        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 768.into(), 765.into(), price()),
            Status::partial(
                765.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 768.into(), 768.into(), price()),
            Status::partial(768.into(), Cause::Overdue()),
        );
        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 1000.into(), 1.into(), price()),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_partial_liability() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(
            max_ltv,
            MIN_DUE_AMOUNT,
            LEASE_AMOUNT_TEST_LPN - BACK_TO_HEALTHY.into() - 1.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 900.into(), BACK_TO_HEALTHY.into(), price()),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn liquidate_partial_overdue() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(
            max_ltv,
            MIN_DUE_AMOUNT,
            LEASE_AMOUNT_TEST_LPN - BACK_TO_HEALTHY.into() - 2.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                900.into(),
                (BACK_TO_HEALTHY + 1).into(),
                price()
            ),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
    }

    #[test]
    fn liquidate_full_liability() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(
            max_ltv,
            MIN_DUE_AMOUNT,
            LEASE_AMOUNT_TEST_LPN - BACK_TO_HEALTHY.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check::<TestCurrency>(LEASE_AMOUNT, 900.into(), BACK_TO_HEALTHY.into(), price()),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full_overdue() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(
            max_ltv,
            MIN_DUE_AMOUNT,
            LEASE_AMOUNT_TEST_LPN - BACK_TO_HEALTHY.into() - 1.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check::<TestCurrency>(
                LEASE_AMOUNT,
                900.into(),
                (BACK_TO_HEALTHY + 1).into(),
                price()
            ),
            Status::full(Cause::Overdue()),
        );
    }

    #[test]
    fn no_liquidate_min_asset() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(
            max_ltv,
            MIN_DUE_AMOUNT,
            LEASE_AMOUNT_TEST_LPN - BACK_TO_HEALTHY.into() - 1.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check(LEASE_AMOUNT, 572.into(), MIN_DUE_AMOUNT - 1.into(), price()),
            Status::No(Zone::third(spec.third_liq_warn(), spec.max())),
        );
    }

    const STEP: Percent = Percent::from_permille(10);

    fn price() -> Price<TestLpn, TestCurrency> {
        let amount_test_lpn: Coin<TestLpn> = Coin::new(1_000);
        let amount_test_currency: Coin<TestCurrency> = Coin::new(1_000);
        price::total_of(amount_test_lpn).is(amount_test_currency)
    }

    fn liability_with_first(
        warn: Percent,
        min_liquidation: Coin<TestLpn>,
        min_asset: Coin<TestLpn>,
    ) -> Liability<TestLpn> {
        liability_with_max(warn + STEP + STEP + STEP, min_liquidation, min_asset)
    }

    fn liability_with_second(
        warn: Percent,
        min_liquidation: Coin<TestLpn>,
        min_asset: Coin<TestLpn>,
    ) -> Liability<TestLpn> {
        liability_with_max(warn + STEP + STEP, min_liquidation, min_asset)
    }

    fn liability_with_third(
        warn: Percent,
        min_liquidation: Coin<TestLpn>,
        min_asset: Coin<TestLpn>,
    ) -> Liability<TestLpn> {
        liability_with_max(warn + STEP, min_liquidation, min_asset)
    }

    // init = 1%, healthy = 1%, first = max - 3, second = max - 2, third = max - 1
    fn liability_with_max(
        max: Percent,
        min_liquidation: Coin<TestLpn>,
        min_asset: Coin<TestLpn>,
    ) -> Liability<TestLpn> {
        let initial = STEP;
        assert!(initial < max - STEP - STEP - STEP);

        Liability::<TestLpn>::try_from(LiabilityDTO::new(
            initial,
            Percent::ZERO,
            max - initial,
            (STEP, STEP, STEP),
            min_liquidation.into(),
            min_asset.into(),
            Duration::from_hours(1),
        ))
        .unwrap()
    }
}
#[cfg(test)]
mod test_s {
    use currency::lpn::Usdc;

    use crate::{
        coin::{Amount, Coin},
        duration::Duration,
        fraction::Fraction,
        percent::{Percent, Units},
        zero::Zero,
    };

    use super::{Liability, Zone};

    pub type TestLpn = Usdc;

    #[test]
    fn test_zone_of() {
        let l = Liability::<TestLpn> {
            initial: Percent::from_percent(60),
            healthy: Percent::from_percent(65),
            max: Percent::from_percent(85),
            first_liq_warn: Percent::from_permille(792),
            second_liq_warn: Percent::from_permille(815),
            third_liq_warn: Percent::from_permille(826),
            min_liquidation: Coin::<TestLpn>::new(10_000),
            min_asset: Coin::<TestLpn>::new(15_000_000),
            recalc_time: Duration::from_secs(20000),
        };
        assert_eq!(zone_of(&l, 0), Zone::no_warnings(l.first_liq_warn()));
        assert_eq!(zone_of(&l, 660), Zone::no_warnings(l.first_liq_warn()));
        assert_eq!(zone_of(&l, 791), Zone::no_warnings(l.first_liq_warn()));
        assert_eq!(
            zone_of(&l, 792),
            Zone::first(l.first_liq_warn(), l.second_liq_warn())
        );
        assert_eq!(
            zone_of(&l, 814),
            Zone::first(l.first_liq_warn(), l.second_liq_warn())
        );
        assert_eq!(
            zone_of(&l, 815),
            Zone::second(l.second_liq_warn(), l.third_liq_warn())
        );
        assert_eq!(
            zone_of(&l, 825),
            Zone::second(l.second_liq_warn(), l.third_liq_warn())
        );
        assert_eq!(zone_of(&l, 826), Zone::third(l.third_liq_warn(), l.max()));
        assert_eq!(zone_of(&l, 849), Zone::third(l.third_liq_warn(), l.max()));
    }

    #[test]
    fn init_borrow() {
        test_init_borrow_amount(1000, 50, 1000, None);
        test_init_borrow_amount(1, 10, 0, None);
        test_init_borrow_amount(1000, 99, 990 * 100, None);
        test_init_borrow_amount(10, 65, 18, None);
        test_init_borrow_amount(100, 60, 150, None);
        test_init_borrow_amount(250, 59, 359, None);
        test_init_borrow_amount(70, 5, 3, None);
        test_init_borrow_amount(90, 25, 30, None);
    }

    #[test]
    fn init_borrow_max_ltd() {
        test_init_borrow_amount(50000, 60, 25000, Some(Percent::from_percent(50)));
        test_init_borrow_amount(1000, 10, 100, Some(Percent::from_percent(10)));
        test_init_borrow_amount(1, 10, 0, Some(Percent::from_percent(5)));
        test_init_borrow_amount(1000, 60, 1500, Some(Percent::from_percent(190)));
        test_init_borrow_amount(4000, 55, 4800, Some(Percent::from_percent(120)));
        test_init_borrow_amount(200, 49, 192, Some(Percent::from_percent(100)));
        test_init_borrow_amount(1, 65, 0, Some(Percent::from_percent(65)));
        test_init_borrow_amount(2000, 60, 3000, Some(Percent::from_percent(250)));
        test_init_borrow_amount(300000, 65, 450000, Some(Percent::from_percent(150)));
        test_init_borrow_amount(50, 45, 40, Some(Percent::from_permille(999)));

        test_init_borrow_amount(1000, 65, 0, Some(Percent::ZERO));
    }

    #[test]
    fn amount_to_liquidate() {
        let healthy = 85;
        let max = 90;
        let liability = Liability::<TestLpn> {
            initial: Percent::from_percent(60),
            healthy: Percent::from_percent(healthy),
            max: Percent::from_percent(max),
            first_liq_warn: Percent::from_permille(860),
            second_liq_warn: Percent::from_permille(865),
            third_liq_warn: Percent::from_permille(870),
            min_liquidation: Coin::<TestLpn>::new(10_000),
            min_asset: Coin::<TestLpn>::new(15_000_000),
            recalc_time: Duration::from_secs(20000),
        };
        let lease_amount: Amount = 100;
        let healthy_amount = Percent::from_percent(healthy).of(lease_amount);
        let max_amount = Percent::from_percent(max).of(lease_amount);
        amount_to_liquidate_int(liability, lease_amount, Amount::ZERO, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount - 10, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount - 1, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, healthy_amount + 1, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, max_amount - 1, Amount::ZERO);
        amount_to_liquidate_int(liability, lease_amount, max_amount, 33);
        amount_to_liquidate_int(liability, lease_amount, max_amount + 1, 40);
        amount_to_liquidate_int(liability, lease_amount, max_amount + 8, 86);
        amount_to_liquidate_int(liability, lease_amount, lease_amount - 1, 93);
        amount_to_liquidate_int(liability, lease_amount, lease_amount, lease_amount);
        amount_to_liquidate_int(liability, lease_amount, lease_amount + 1, lease_amount);
        amount_to_liquidate_int(liability, lease_amount, lease_amount + 10, lease_amount);
    }

    #[track_caller]
    fn amount_to_liquidate_int(
        liability: Liability<TestLpn>,
        lease: Amount,
        due: Amount,
        exp: Amount,
    ) {
        let liq = liability.amount_to_liquidate(lease, due);
        assert_eq!(exp, liq);
        if due.clamp(liability.max.of(lease), lease) == due {
            assert!(
                liability
                    .healthy_percent()
                    .of(lease - exp)
                    .abs_diff(due - exp)
                    <= 1,
                "Lease = {lease}, due = {due}, exp = {exp}"
            );
        }
    }

    fn zone_of(l: &Liability<TestLpn>, permilles: Units) -> Zone {
        l.zone_of(Percent::from_permille(permilles))
    }

    fn test_init_borrow_amount(d: u128, p: u16, exp: u128, max_p: Option<Percent>) {
        let downpayment = Coin::<TestLpn>::new(d);
        let percent = Percent::from_percent(p);
        let calculated = Liability::<TestLpn> {
            initial: percent,
            healthy: Percent::from_percent(99),
            max: Percent::from_percent(100),
            first_liq_warn: Percent::from_permille(992),
            second_liq_warn: Percent::from_permille(995),
            third_liq_warn: Percent::from_permille(998),
            min_liquidation: Coin::<TestLpn>::new(10_000),
            min_asset: Coin::<TestLpn>::new(15_000_000),
            recalc_time: Duration::from_secs(20000),
        }
        .init_borrow_amount(downpayment, max_p);

        assert_eq!(calculated, Coin::<TestLpn>::new(exp));
    }
}
