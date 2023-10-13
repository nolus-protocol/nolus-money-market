use currency::Currency;
use finance::{
    coin::Coin,
    liability::Liability,
    percent::Percent,
    price::{self, Price},
};

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
        Ok(Self::new_internal(amount.try_into()?, spec))
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
    // for more insights refer to the TODO next to `Self::check_liability`
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
            .map(Status::Liquidation)
            .unwrap_or_else(|| {
                no_liquidation(
                    self.spec.liability(),
                    total_due,
                    ltv.min(self.spec.liability().third_liq_warn()),
                )
            })
    }

    /// Check if the amount can be used to close the position.
    /// Return `error::ContractError::PositionCloseAmountTooSmall` when a partial close is requested
    /// with amount less than the minimum sell asset position parameter sent on lease open. Refer to
    /// `NewLeaseForm::position_spec`.
    ///
    /// Return `error::ContractError::PositionCloseAmountTooBig` when a partial close is requested
    /// with amount that would decrease a position less than the minimum asset parameter sent on
    /// lease open. Refer to `NewLeaseForm::position_spec`.
    pub fn validate_close_amount(
        &self,
        close_amount: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> ContractResult<()> {
        let trasaction_in_lpn = lpn_in_assets.inv();

        self.spec
            .check_trasaction_amount(close_amount, trasaction_in_lpn)
            .map_err(|err| match err {
                ContractError::InsufficientTrasactionAmount(min_trasaction_amount) => {
                    ContractError::PositionCloseAmountTooSmall(min_trasaction_amount)
                }
                _ => err,
            })
            .and_then(|_| {
                self.spec
                    .check_asset_amount(self.amount.saturating_sub(close_amount), trasaction_in_lpn)
            })
            .map_err(|err| match err {
                ContractError::InsufficientAssetAmount(min_asset) => {
                    ContractError::PositionCloseAmountTooBig(min_asset)
                }
                _ => err,
            })
    }

    fn invariant_held(&self) -> ContractResult<()> {
        Self::check(!self.amount.is_zero(), "The amount should be positive")
    }

    fn check(invariant: bool, msg: &str) -> ContractResult<()> {
        ContractError::broken_invariant_if::<Self>(!invariant, msg)
    }

    fn may_ask_liquidation_liability(
        &self,
        total_due: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Option<Liquidation<Asset>> {
        let liquidation_amount = self
            .spec
            .liability()
            .amount_to_liquidate(self.amount, total_due);
        self.may_ask_liquidation(
            Cause::Liability {
                ltv: self.spec.liability().max(),
                healthy_ltv: self.spec.liability().healthy_percent(),
            },
            liquidation_amount,
            lpn_in_assets,
        )
    }

    fn may_ask_liquidation_overdue(
        &self,
        overdue: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Option<Liquidation<Asset>> {
        self.may_ask_liquidation(Cause::Overdue(), overdue, lpn_in_assets)
    }

    fn may_ask_liquidation(
        &self,
        cause: Cause,
        liquidation: Coin<Asset>,
        lpn_in_assets: Price<Lpn, Asset>,
    ) -> Option<Liquidation<Asset>> {
        match self.validate_close_amount(liquidation, lpn_in_assets) {
            Err(ContractError::PositionCloseAmountTooSmall(_)) => None,
            Err(ContractError::PositionCloseAmountTooBig(_)) => Some(Liquidation::Full(cause)),
            Err(_) => unreachable!(),
            Ok(()) => {
                debug_assert!(liquidation < self.amount);
                Some(Liquidation::Partial {
                    amount: liquidation,
                    cause,
                })
            }
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

    use currency::dex::test::{PaymentC3, StableC1};
    use finance::{
        coin::Coin,
        duration::Duration,
        liability::{Liability, Zone},
        percent::Percent,
        price::{self, Price},
    };

    use crate::position::{Cause, Position, Spec, Status};

    type TestCurrency = PaymentC3;
    type TestLpn = StableC1;

    #[test]
    fn no_debt() {
        let warn_ltv = Percent::from_permille(11);
        let spec = position_with_first(warn_ltv, 100, 1, 1);
        assert_eq!(
            spec.check_liability(0.into(), 0.into(), price(1, 1)),
            Status::NoDebt,
        );
        assert_eq!(
            spec.check_liability(0.into(), 0.into(), price(1, 3)),
            Status::NoDebt,
        );
    }

    #[test]
    fn warnings_none_zero_liq() {
        let warn_ltv = Percent::from_percent(51);
        let position = position_with_first(warn_ltv, 100, 1, 1);
        assert_eq!(
            position.check_liability(1.into(), 0.into(), price(1, 1)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(1.into(), 0.into(), price(1, 5)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(50.into(), 0.into(), price(1, 1)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(25.into(), 0.into(), price(1, 2)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(51.into(), 0.into(), price(1, 1)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(17.into(), 0.into(), price(1, 3)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
    }

    #[test]
    fn warnings_none_min_sell_asset() {
        let warn_ltv = Percent::from_percent(51);
        let position = position_with_first(warn_ltv, 100, 1, 15);
        assert_eq!(
            position.check_liability(50.into(), 14.into(), price(1, 1)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(25.into(), 4.into(), price(3, 2)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(51.into(), 14.into(), price(1, 1)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(17.into(), 4.into(), price(1, 3)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
    }

    #[test]
    fn warnings_first() {
        let position = position_with_first(Percent::from_permille(712), 1000, 10, 1);

        assert_eq!(
            position.check_liability(711.into(), 0.into(), price(1, 1)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(237.into(), 0.into(), price(1, 3)),
            Status::No(Zone::no_warnings(
                position.spec.liability().first_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(712.into(), 0.into(), price(1, 1)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(178.into(), 0.into(), price(1, 4)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(712.into(), 1.into(), price(1, 1)),
            Status::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(89.into(), 1.into(), price(1, 8)),
            Status::partial(8.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(721.into(), 0.into(), price(1, 1)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(103.into(), 0.into(), price(1, 7)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(722.into(), 0.into(), price(1, 1)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(361.into(), 0.into(), price(1, 2)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
    }

    #[test]
    fn warnings_first_min_sell_asset() {
        let position = position_with_first(Percent::from_permille(712), 1000, 10, 3);

        assert_eq!(
            position.check_liability(712.into(), 2.into(), price(1, 1)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(356.into(), 1.into(), price(1, 2)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(721.into(), 2.into(), price(1, 1)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(721.into(), 5.into(), price(1, 1)),
            Status::partial(5.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(240.into(), 3.into(), price(1, 3)),
            Status::partial(9.into(), Cause::Overdue()),
        );
    }

    #[test]
    fn warnings_second() {
        let position = position_with_second(Percent::from_permille(123), 1000, 10, 1);

        assert_eq!(
            position.check_liability(122.into(), 0.into(), price(1, 1)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(15.into(), 0.into(), price(1, 8)),
            Status::No(Zone::first(
                position.spec.liability().first_liq_warn(),
                position.spec.liability().second_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(123.into(), 0.into(), price(1, 1)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(82.into(), 0.into(), price(2, 3)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(123.into(), 4.into(), price(1, 1)),
            Status::partial(4.into(), Cause::Overdue())
        );
        assert_eq!(
            position.check_liability(132.into(), 0.into(), price(1, 1)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(66.into(), 0.into(), price(1, 2)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(133.into(), 0.into(), price(1, 1)),
            Status::No(Zone::third(
                position.spec.liability().third_liq_warn(),
                position.spec.liability().max()
            )),
        );
    }

    #[test]
    fn warnings_second_min_sell_asset() {
        let position = position_with_second(Percent::from_permille(123), 1000, 10, 5);

        assert_eq!(
            position.check_liability(128.into(), 4.into(), price(1, 1)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(32.into(), 1.into(), price(1, 4)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                position.spec.liability().third_liq_warn()
            )),
        );
        assert_eq!(
            position.check_liability(128.into(), 5.into(), price(1, 1)),
            Status::partial(5.into(), Cause::Overdue())
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let position = position_with_third(warn_third_ltv, 1000, 100, 1);

        assert_eq!(
            position.check_liability(380.into(), 0.into(), price(1, 1)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                warn_third_ltv
            )),
        );
        assert_eq!(
            position.check_liability(190.into(), 0.into(), price(1, 2)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                warn_third_ltv
            )),
        );
        assert_eq!(
            position.check_liability(381.into(), 0.into(), price(1, 1)),
            Status::No(Zone::third(warn_third_ltv, max_ltv)),
        );
        assert_eq!(
            position.check_liability(381.into(), 375.into(), price(1, 1)),
            Status::partial(375.into(), Cause::Overdue())
        );
        assert_eq!(
            position.check_liability(573.into(), 562.into(), price(3, 2)),
            Status::partial(374.into(), Cause::Overdue())
        );
        assert_eq!(
            position.check_liability(390.into(), 0.into(), price(1, 1)),
            Status::No(Zone::third(warn_third_ltv, max_ltv)),
        );
        assert_eq!(
            position.check_liability(391.into(), 0.into(), price(1, 1)),
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
        let max_ltv = warn_third_ltv + STEP;
        let position = position_with_third(warn_third_ltv, 1000, 100, 386);

        assert_eq!(
            position.check_liability(380.into(), 1.into(), price(1, 1)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                warn_third_ltv
            )),
        );
        assert_eq!(
            position.check_liability(126.into(), 1.into(), price(1, 3)),
            Status::No(Zone::second(
                position.spec.liability().second_liq_warn(),
                warn_third_ltv
            )),
        );
        assert_eq!(
            position.check_liability(381.into(), 375.into(), price(1, 1)),
            Status::No(Zone::third(warn_third_ltv, max_ltv)),
        );
        assert_eq!(
            position.check_liability(391.into(), 385.into(), price(1, 1)),
            Status::No(Zone::third(warn_third_ltv, max_ltv)),
        );
        assert_eq!(
            position.check_liability(391.into(), 386.into(), price(1, 1)),
            Status::partial(386.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(392.into(), 0.into(), price(1, 1)),
            Status::No(Zone::third(warn_third_ltv, max_ltv)),
        );
        assert_eq!(
            position.check_liability(364.into(), 0.into(), price(1, 2)),
            Status::No(Zone::third(warn_third_ltv, max_ltv)),
        );
        assert_eq!(
            position.check_liability(393.into(), 0.into(), price(1, 1)),
            Status::partial(
                386.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.check_liability(788.into(), 0.into(), price(2, 1)),
            Status::partial(
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
            position.check_liability(880.into(), 1.into(), price(1, 1)),
            Status::partial(1.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(139.into(), 1.into(), price(1, 4)),
            Status::partial(4.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(881.into(), 879.into(), price(1, 1)),
            Status::partial(
                879.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.check_liability(881.into(), 880.into(), price(1, 1)),
            Status::partial(880.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(294.into(), 294.into(), price(3, 1)),
            Status::partial(98.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(294.into(), 293.into(), price(1, 3)),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            position.check_liability(1000.into(), 1.into(), price(1, 1)),
            Status::full(Cause::Liability {
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
            position.check_liability(900.into(), 897.into(), price(1, 1)),
            Status::partial(
                898.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.check_liability(900.into(), 899.into(), price(1, 1)),
            Status::partial(899.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(233.into(), 233.into(), price(1, 3)),
            Status::partial(699.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(901.into(), 889.into(), price(1, 1)),
            Status::partial(
                900.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.check_liability(902.into(), 889.into(), price(1, 1)),
            Status::full(Cause::Liability {
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
            position.check_liability(768.into(), 765.into(), price(1, 1)),
            Status::partial(
                765.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.check_liability(1560.into(), 1552.into(), price(2, 1)),
            Status::partial(
                777.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.check_liability(768.into(), 768.into(), price(1, 1)),
            Status::partial(768.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(1560.into(), 1556.into(), price(2, 1)),
            Status::partial(778.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(788.into(), 768.into(), price(1, 1)),
            Status::full(Cause::Liability {
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
            position.check_liability(882.into(), 1.into(), price(1, 1)),
            Status::partial(
                880.into(),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            position.check_liability(883.into(), 1.into(), price(1, 1)),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            position.check_liability(294.into(), 1.into(), price(1, 3)),
            Status::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            position.check_liability(1000.into(), 1.into(), price(1, 1)),
            Status::full(Cause::Liability {
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
            position.check_liability(772.into(), 674.into(), price(1, 1)),
            Status::partial(674.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(1674.into(), 1674.into(), price(2, 1)),
            Status::partial(837.into(), Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(772.into(), 675.into(), price(1, 1)),
            Status::full(Cause::Overdue()),
        );
        assert_eq!(
            position.check_liability(1676.into(), 1676.into(), price(2, 1)),
            Status::full(Cause::Overdue()),
        );
    }

    const STEP: Percent = Percent::from_permille(10);

    fn price<Lpn, Asset>(price_lpn: Lpn, price_asset: Asset) -> Price<TestLpn, TestCurrency>
    where
        Lpn: Into<Coin<TestLpn>>,
        Asset: Into<Coin<TestCurrency>>,
    {
        price::total_of(price_lpn.into()).is(price_asset.into())
    }

    fn position_with_first<Asset, Lpn>(
        warn: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_sell_asset: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        position_with_max(warn + STEP + STEP + STEP, asset, min_asset, min_sell_asset)
    }

    fn position_with_second<Asset, Lpn>(
        warn: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_sell_asset: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        position_with_max(warn + STEP + STEP, asset, min_asset, min_sell_asset)
    }

    fn position_with_third<Asset, Lpn>(
        warn: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_sell_asset: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        position_with_max(warn + STEP, asset, min_asset, min_sell_asset)
    }

    // init = 1%, healthy = 1%, first = max - 3, second = max - 2, third = max - 1
    fn position_with_max<Asset, Lpn>(
        max: Percent,
        asset: Asset,
        min_asset: Lpn,
        min_sell_asset: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
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
        let spec = Spec::new(liability, min_asset.into(), min_sell_asset.into());

        Position::new(asset.into(), spec)
    }
}

#[cfg(test)]
mod test_validate_close {
    use currency::dex::test::{PaymentC3, StableC1};
    use finance::{
        coin::Coin,
        duration::Duration,
        liability::Liability,
        percent::Percent,
        price::{self, Price},
    };

    use crate::error::ContractError;

    use super::{Position, Spec};

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

        let result_2 = spec.validate_close_amount(6.into(), price(2, 1));
        assert!(matches!(
            result_2,
            Err(ContractError::PositionCloseAmountTooSmall(_))
        ));
    }

    #[test]
    fn amount_as_min_sell_asset() {
        let spec = position(100, 85, 15);
        let result_1 = spec.validate_close_amount(15.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(5.into(), price(3, 1));
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

        let result_2 = spec.validate_close_amount(64.into(), price(2, 3));
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

        let result_2 = spec.validate_close_amount(62.into(), price(2, 3));
        assert!(result_2.is_ok());
    }

    #[test]
    fn valid_amount() {
        let spec = position(100, 40, 10);
        let result_1 = spec.validate_close_amount(53.into(), price(1, 1));
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(89.into(), price(4, 1));
        assert!(result_2.is_ok());
    }

    fn position<Asset, Lpn>(
        amount: Asset,
        min_asset: Lpn,
        min_sell_asset: Lpn,
    ) -> Position<TestCurrency, TestLpn>
    where
        Asset: Into<Coin<TestCurrency>>,
        Lpn: Into<Coin<TestLpn>>,
    {
        let liability = Liability::new(
            Percent::from_percent(65),
            Percent::from_percent(5),
            Percent::from_percent(10),
            Percent::from_percent(2),
            Percent::from_percent(3),
            Percent::from_percent(2),
            Duration::from_hours(1),
        );
        let spec = Spec::<TestLpn>::new(liability, min_asset.into(), min_sell_asset.into());

        Position::<TestCurrency, TestLpn>::new(amount.into(), spec)
    }

    fn price<Lpn, Asset>(price_lpn: Lpn, price_asset: Asset) -> Price<TestLpn, TestCurrency>
    where
        Lpn: Into<Coin<TestLpn>>,
        Asset: Into<Coin<TestCurrency>>,
    {
        price::total_of(price_lpn.into()).is(price_asset.into())
    }
}
