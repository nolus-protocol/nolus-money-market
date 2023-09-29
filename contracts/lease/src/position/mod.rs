use currency::Currency;
use finance::{
    coin::Coin,
    liability::Liability,
    percent::Percent,
    price::{self, Price},
};

use crate::{
    api::{LeaseCoin, PositionSpec},
    error::{ContractError, ContractResult},
};

pub use dto::PositionDTO;
pub use status::{Cause, Liquidation, Status};

mod dto;
mod status;

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

    pub fn try_from(amount: LeaseCoin, spec: PositionSpec) -> ContractResult<Self> {
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
    // for more insights refer to the TODO next to `Self::check_liability`
    pub fn liability(&self) -> Liability {
        self.liability
    }

    pub fn close(&mut self, asset: Coin<Asset>) {
        debug_assert!(
            asset < self.amount,
            "Liquidated asset {asset} should be less than the available {0}",
            self.amount
        );

        self.amount -= asset
    }

    // TODO return the next `recalc_time` as well to simplify the API and its usage
    // remove the `fn recalc_time`
    // compute the point of time based on the  provided current time
    pub fn check_liability(
        &self,
        total_due: Coin<Lpn>,
        overdue: Coin<Lpn>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Status<Asset> {
        debug_assert!(overdue <= total_due);

        let total_due = price::total(total_due, lpn_in_assets);
        let overdue = price::total(overdue, lpn_in_assets);
        debug_assert!(overdue <= total_due);

        let ltv = Percent::from_ratio(total_due, self.amount);
        self.may_ask_liquidation_liability(total_due, lpn_in_assets)
            .max(self.may_ask_liquidation_overdue(overdue, lpn_in_assets))
            .unwrap_or_else(|| {
                no_liquidation(
                    self.liability,
                    total_due,
                    ltv.min(self.liability.third_liq_warn()),
                )
            })
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

    fn may_ask_liquidation_liability(
        &self,
        total_due: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Option<Status<Asset>> {
        let liquidation_amount = self.liability.amount_to_liquidate(self.amount, total_due);
        self.may_ask_liquidation(
            Cause::Liability {
                ltv: self.liability.max(),
                healthy_ltv: self.liability.healthy_percent(),
            },
            liquidation_amount,
            lpn_in_assets,
        )
    }

    fn may_ask_liquidation_overdue(
        &self,
        overdue: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Option<Status<Asset>> {
        self.may_ask_liquidation(Cause::Overdue(), overdue, lpn_in_assets)
    }

    fn may_ask_liquidation(
        &self,
        cause: Cause,
        liquidation: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Option<Status<Asset>> {
        let min_asset = price::total(self.min_asset, lpn_in_assets);
        let min_sell_asset = price::total(self.min_sell_asset, lpn_in_assets);

        if liquidation.is_zero() || liquidation < min_sell_asset {
            None
        } else if self.amount.saturating_sub(liquidation) <= min_asset {
            Some(Status::full(cause))
        } else {
            Some(Status::partial(liquidation, cause))
        }
    }
}

fn no_liquidation<Asset>(
    liability: Liability,
    total_due: Coin<Asset>,
    ltv: Percent,
) -> Status<Asset>
where
    Asset: Currency,
{
    debug_assert!(ltv < liability.max());
    if total_due.is_zero() {
        Status::NoDebt
    } else {
        Status::No(liability.zone_of(ltv))
    }
}

#[cfg(test)]
mod test_check {

    use currency::{lpn::Usdc, test::Dai};
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        liability::{Liability, Zone},
        percent::Percent,
        price::{self, Price},
        zero::Zero,
    };

    use crate::position::{Cause, Position, Status};

    type TestCurrency = Dai;
    type TestLpn = Usdc;

    const LEASE_AMOUNT: Coin<TestCurrency> = Coin::new(1000);
    const LEASE_AMOUNT_LPN: Coin<TestLpn> = Coin::new(1000);
    const MIN_DUE_AMOUNT: Coin<TestLpn> = Coin::new(100);
    const MIN_DUE_AMOUNT_ASSET: Coin<TestCurrency> = Coin::new(100);
    const MIN_ASSET: Coin<TestLpn> = Coin::new(100);
    const MIN_SELL_ASSET: Coin<TestLpn> = Coin::ZERO;
    const PRICE_TEST_LPN: Coin<TestLpn> = Coin::new(1_000);
    const PRICE_TEST_CURRENCY: Coin<TestCurrency> = Coin::new(1_000);

    #[test]
    fn no_debt() {
        let warn_ltv = Percent::from_permille(11);
        let spec = liability_with_first(warn_ltv, 100.into(), MIN_ASSET, MIN_SELL_ASSET);
        assert_eq!(
            spec.check_liability(0.into(), 0.into(), price()),
            Status::NoDebt,
        );
    }

    #[test]
    fn warnings_none() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv, 100.into(), MIN_ASSET, MIN_SELL_ASSET);
        assert_eq!(
            spec.check_liability(1.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.liability.first_liq_warn())),
        );
        assert_eq!(
            spec.check_liability(49.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.liability.first_liq_warn())),
        );
        assert_eq!(
            spec.check_liability(50.into(), 0.into(), price()),
            Status::No(Zone::no_warnings(spec.liability.first_liq_warn())),
        );
    }

    #[test]
    fn no_zone() {
        let warn_ltv = Percent::from_percent(51);
        let spec = liability_with_first(warn_ltv, LEASE_AMOUNT, MIN_ASSET, 100.into());
        assert_eq!(
            spec.check_liability(505.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::no_warnings(spec.liability.first_liq_warn())),
        );
        assert_eq!(
            spec.check_liability(505.into(), MIN_DUE_AMOUNT, price(),),
            Status::partial(MIN_DUE_AMOUNT_ASSET, Cause::Overdue()),
        );
        assert_eq!(
            spec.check_liability(509.into(), 0.into(), price(),),
            Status::No(Zone::no_warnings(spec.liability.first_liq_warn())),
        );
        assert_eq!(
            spec.check_liability(510.into(), 0.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(510.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(510.into(), MIN_DUE_AMOUNT, price(),),
            Status::partial(MIN_DUE_AMOUNT_ASSET, Cause::Overdue()),
        );
        assert_eq!(
            spec.check_liability(515.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );
    }

    #[test]
    fn warnings_first() {
        let spec = liability_with_first(
            Percent::from_permille(712),
            LEASE_AMOUNT,
            MIN_ASSET,
            MIN_SELL_ASSET,
        );

        assert_eq!(
            spec.check_liability(711.into(), 0.into(), price(),),
            Status::No(Zone::no_warnings(spec.liability.first_liq_warn())),
        );
        assert_eq!(
            spec.check_liability(712.into(), 0.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );

        assert_eq!(
            spec.check_liability(712.into(), MIN_DUE_AMOUNT, price(),),
            Status::partial(MIN_DUE_AMOUNT_ASSET, Cause::Overdue())
        );
        assert_eq!(
            spec.check_liability(715.into(), 0.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(721.into(), 0.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(721.into(), MIN_DUE_AMOUNT, price(),),
            Status::partial(MIN_DUE_AMOUNT_ASSET, Cause::Overdue()),
        );
        assert_eq!(
            spec.check_liability(722.into(), 0.into(), price(),),
            Status::No(Zone::second(
                spec.liability.second_liq_warn(),
                spec.liability.third_liq_warn()
            )),
        );
    }

    #[test]
    fn warnings_first_min_sell_asset() {
        let spec = liability_with_first(
            Percent::from_permille(712),
            LEASE_AMOUNT,
            MIN_ASSET,
            100.into(),
        );

        assert_eq!(
            spec.check_liability(712.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );
    }

    #[test]
    fn warnings_second() {
        let spec = liability_with_second(
            Percent::from_permille(123),
            LEASE_AMOUNT,
            MIN_ASSET,
            MIN_SELL_ASSET,
        );

        assert_eq!(
            spec.check_liability(122.into(), 0.into(), price(),),
            Status::No(Zone::first(
                spec.liability.first_liq_warn(),
                spec.liability.second_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(123.into(), 0.into(), price(),),
            Status::No(Zone::second(
                spec.liability.second_liq_warn(),
                spec.liability.third_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(124.into(), 0.into(), price(),),
            Status::No(Zone::second(
                spec.liability.second_liq_warn(),
                spec.liability.third_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(128.into(), MIN_DUE_AMOUNT, price(),),
            Status::partial(MIN_DUE_AMOUNT_ASSET, Cause::Overdue())
        );
        assert_eq!(
            spec.check_liability(132.into(), 0.into(), price(),),
            Status::No(Zone::second(
                spec.liability.second_liq_warn(),
                spec.liability.third_liq_warn()
            )),
        );
        assert_eq!(
            spec.check_liability(133.into(), 0.into(), price(),),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
    }

    #[test]
    fn warnings_second_min_sell_asset() {
        let spec = liability_with_second(
            Percent::from_permille(123),
            LEASE_AMOUNT,
            MIN_ASSET,
            MIN_ASSET,
        );

        assert_eq!(
            spec.check_liability(128.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::second(
                spec.liability.second_liq_warn(),
                spec.liability.third_liq_warn()
            )),
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = liability_with_third(warn_third_ltv, LEASE_AMOUNT, MIN_ASSET, MIN_SELL_ASSET);

        assert_eq!(
            spec.check_liability(380.into(), 0.into(), price(),),
            Status::No(Zone::second(
                spec.liability.second_liq_warn(),
                spec.liability.third_liq_warn()
            )),
        );

        assert_eq!(
            spec.check_liability(381.into(), MIN_DUE_AMOUNT, price()),
            Status::partial(MIN_DUE_AMOUNT_ASSET, Cause::Overdue())
        );
        assert_eq!(
            spec.check_liability(381.into(), 0.into(), price()),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
        assert_eq!(
            spec.check_liability(382.into(), 0.into(), price()),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
        assert_eq!(
            spec.check_liability(390.into(), 0.into(), price()),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
        assert_eq!(
            spec.check_liability(391.into(), 0.into(), price()),
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
    fn warnings_third_min_sell_asset() {
        let warn_third_ltv = Percent::from_permille(381);
        let spec = liability_with_third(warn_third_ltv, LEASE_AMOUNT, MIN_ASSET, MIN_ASSET);

        assert_eq!(
            spec.check_liability(380.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::second(
                spec.liability.second_liq_warn(),
                spec.liability.third_liq_warn()
            )),
        );
    }

    #[test]
    fn min_sell_asset() {
        let max_ltv = Percent::from_permille(751);
        let spec = liability_with_max(max_ltv, LEASE_AMOUNT, 1000.into(), 1000.into());

        assert_eq!(
            spec.check_liability(750.into(), 99.into(), price(),),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
        assert_eq!(
            spec.check_liability(751.into(), 99.into(), price(),),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
        assert_eq!(
            spec.check_liability(761.into(), 0.into(), price(),),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
    }

    #[test]
    fn liquidate_partial() {
        let max_ltv = Percent::from_permille(881);
        const BACK_TO_HEALTHY: Amount = 879;
        let spec = liability_with_max(max_ltv, LEASE_AMOUNT, MIN_ASSET, MIN_SELL_ASSET);

        assert_eq!(
            spec.check_liability(880.into(), MIN_DUE_AMOUNT, price(),),
            Status::partial(MIN_DUE_AMOUNT_ASSET, Cause::Overdue()),
        );
        assert_eq!(
            spec.check_liability(881.into(), MIN_DUE_AMOUNT, price(),),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check_liability(881.into(), (BACK_TO_HEALTHY - 1).into(), price(),),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check_liability(881.into(), BACK_TO_HEALTHY.into(), price(),),
            Status::partial(
                BACK_TO_HEALTHY.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check_liability(881.into(), (BACK_TO_HEALTHY + 1).into(), price(),),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
        assert_eq!(
            spec.check_liability(1000.into(), 1.into(), price(),),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_partial_min_sell_asset() {
        let max_ltv = Percent::from_permille(881);
        let spec = liability_with_max(max_ltv, LEASE_AMOUNT, MIN_ASSET, MIN_ASSET);

        assert_eq!(
            spec.check_liability(880.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
    }

    #[test]
    fn liquidate_partial_min_asset() {
        let max_ltv = Percent::from_permille(881);
        let spec = liability_with_max(max_ltv, LEASE_AMOUNT, 1.into(), MIN_SELL_ASSET);

        assert_eq!(
            spec.check_liability(999.into(), 997.into(), price(),),
            Status::partial(
                998.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn liquidate_full() {
        let max_ltv = Percent::from_permille(768);
        let spec = liability_with_max(max_ltv, LEASE_AMOUNT, MIN_ASSET, MIN_SELL_ASSET);

        assert_eq!(
            spec.check_liability(768.into(), 765.into(), price(),),
            Status::partial(
                765.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.check_liability(768.into(), 768.into(), price(),),
            Status::partial(768.into(), Cause::Overdue()),
        );
        assert_eq!(
            spec.check_liability(1000.into(), 1.into(), price(),),
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
            LEASE_AMOUNT,
            MIN_ASSET,
            LEASE_AMOUNT_LPN - BACK_TO_HEALTHY.into() - 1.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check_liability(900.into(), BACK_TO_HEALTHY.into(), price(),),
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
            LEASE_AMOUNT,
            MIN_ASSET,
            LEASE_AMOUNT_LPN - BACK_TO_HEALTHY.into() - 2.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check_liability(900.into(), (BACK_TO_HEALTHY + 1).into(), price(),),
            Status::partial((BACK_TO_HEALTHY + 1).into(), Cause::Overdue()),
        );
    }

    #[test]
    fn liquidate_full_liability() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(
            max_ltv,
            LEASE_AMOUNT,
            MIN_ASSET + 2.into(),
            LEASE_AMOUNT_LPN - BACK_TO_HEALTHY.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check_liability(900.into(), BACK_TO_HEALTHY.into(), price(),),
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
            LEASE_AMOUNT,
            MIN_ASSET + 1.into(),
            LEASE_AMOUNT_LPN - BACK_TO_HEALTHY.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check_liability(900.into(), (BACK_TO_HEALTHY + 1).into(), price(),),
            Status::full(Cause::Overdue()),
        );
    }

    #[test]
    fn no_liquidate_min_asset() {
        let max_ltv = Percent::from_permille(573);
        let spec = liability_with_max(
            max_ltv,
            LEASE_AMOUNT,
            MIN_ASSET,
            LEASE_AMOUNT_LPN - BACK_TO_HEALTHY.into() - 1.into(),
        );

        const BACK_TO_HEALTHY: Amount = 898;

        assert_eq!(
            spec.check_liability(572.into(), MIN_DUE_AMOUNT - 1.into(), price(),),
            Status::No(Zone::third(
                spec.liability.third_liq_warn(),
                spec.liability.max()
            )),
        );
    }

    const STEP: Percent = Percent::from_permille(10);

    fn price() -> Price<TestLpn, TestCurrency> {
        price::total_of(PRICE_TEST_LPN).is(PRICE_TEST_CURRENCY)
    }

    fn liability_with_first(
        warn: Percent,
        asset: Coin<TestCurrency>,
        min_asset: Coin<TestLpn>,
        min_sell_asset: Coin<TestLpn>,
    ) -> Position<TestCurrency, TestLpn> {
        liability_with_max(warn + STEP + STEP + STEP, asset, min_asset, min_sell_asset)
    }

    fn liability_with_second(
        warn: Percent,
        asset: Coin<TestCurrency>,
        min_asset: Coin<TestLpn>,
        min_sell_asset: Coin<TestLpn>,
    ) -> Position<TestCurrency, TestLpn> {
        liability_with_max(warn + STEP + STEP, asset, min_asset, min_sell_asset)
    }

    fn liability_with_third(
        warn: Percent,
        asset: Coin<TestCurrency>,
        min_asset: Coin<TestLpn>,
        min_sell_asset: Coin<TestLpn>,
    ) -> Position<TestCurrency, TestLpn> {
        liability_with_max(warn + STEP, asset, min_asset, min_sell_asset)
    }

    // init = 1%, healthy = 1%, first = max - 3, second = max - 2, third = max - 1
    fn liability_with_max(
        max: Percent,
        asset: Coin<TestCurrency>,
        min_asset: Coin<TestLpn>,
        min_sell_asset: Coin<TestLpn>,
    ) -> Position<TestCurrency, TestLpn> {
        let initial = STEP;
        assert!(initial < max - STEP - STEP - STEP);

        let liability = Liability::new(
            initial,
            Percent::ZERO,
            max - initial,
            STEP,
            STEP,
            STEP,
            Duration::from_hours(1),
        );
        Position::new(asset, liability, min_asset, min_sell_asset)
    }
}
