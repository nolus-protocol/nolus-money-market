use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use finance::{
    fraction::Fraction,
    fractionable::{Fractionable, Percentable},
    percent::Percent,
    range::{Ascending, RightOpenRange},
    zero::Zero,
};
use serde::{Deserialize, Serialize};

use crate::api::position::{ChangeCmd, ClosePolicyChange};

use super::error::{Error as PositionError, Result as PositionResult};

/// Close position policy
///
/// Not designed to be used as an input API component! Invariant checks are not done on deserialization!
/// A position is subject to close if its LTV pertains to the right-open intervals (-inf., `take_profit`),
/// or [`stop_loss`, +inf)
#[derive(Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Policy {
    take_profit: Option<Percent>,
    stop_loss: Option<Percent>,
}

/// A strategy triggered to close the position automatically
///
/// If a recent price movement have the position's LTV trigger a full close as per the configured `Policy`
/// then the close strategy carries details.
///
/// A full close of the position is triggered if:
/// - a Stop Loss is set up and a price decline have the position's LTV become higher than the specified percent, or
/// - a Take Profit is set up and a price rise have the position's LTV become lower than the specified percent.
#[derive(Debug, Eq, PartialEq)]
pub enum Strategy {
    StopLoss(Percent),
    TakeProfit(Percent),
}

impl From<ChangeCmd> for Option<Percent> {
    fn from(cmd: ChangeCmd) -> Self {
        match cmd {
            ChangeCmd::Reset => None,
            ChangeCmd::Set(new_value) => Some(new_value),
        }
    }
}

impl Policy {
    pub fn change_policy<P>(
        self,
        cmd: ClosePolicyChange,
        lease_asset: P,
        total_due: P,
    ) -> PositionResult<Self>
    where
        P: Copy + Debug + PartialOrd + Percentable + Zero,
        Percent: Fractionable<P>,
    {
        Self {
            stop_loss: cmd
                .stop_loss
                .map_or_else(|| self.stop_loss, Option::<Percent>::from),
            take_profit: cmd
                .take_profit
                .map_or_else(|| self.take_profit, Option::<Percent>::from),
        }
        .check_invariant(lease_asset, total_due)
    }

    /// Determine the 'no-close' intersection with the provided range
    ///
    /// Pre: `self.may_trigger() == None` for ltv contained in `during`.
    /// This implies that `during` is not a sub-range of any of the policy ranges.
    pub fn no_close(
        &self,
        during: RightOpenRange<Percent, Ascending>,
    ) -> RightOpenRange<Percent, Ascending> {
        // we may have implemented this in a more conscise form if we have introduced other kind of ranges,
        // for example, RangeFrom
        let tp_cut = self
            .take_profit
            .map_or_else(|| during, |tp| during.cut_to(tp));
        self.stop_loss
            .map_or_else(|| tp_cut, |sl| tp_cut.cut_from(sl))
    }

    pub fn may_trigger<P>(&self, lease_asset: P, total_due: P) -> Option<Strategy>
    where
        P: Percentable + PartialOrd + Copy,
    {
        self.may_stop_loss(lease_asset, total_due)
            .or_else(|| self.may_take_profit(lease_asset, total_due))
    }

    fn check_invariant<P>(self, lease_asset: P, total_due: P) -> PositionResult<Self>
    where
        P: Copy + Debug + Percentable + PartialOrd + Zero,
        Percent: Fractionable<P>,
    {
        self.may_trigger(lease_asset, total_due).map_or_else(
            || Ok(self),
            |strategy| {
                Err(PositionError::trigger_close(
                    ltv(total_due, lease_asset),
                    strategy,
                ))
            },
        )
    }

    fn may_stop_loss<P>(&self, lease_asset: P, total_due: P) -> Option<Strategy>
    where
        P: Percentable + PartialOrd,
    {
        self.stop_loss.and_then(|stop_loss| {
            (stop_loss.of(lease_asset) <= total_due).then_some(Strategy::StopLoss(stop_loss))
        })
    }

    fn may_take_profit<P>(&self, lease_asset: P, total_due: P) -> Option<Strategy>
    where
        P: Percentable + PartialOrd,
    {
        self.take_profit.and_then(|take_profit| {
            (take_profit.of(lease_asset) > total_due).then_some(Strategy::TakeProfit(take_profit))
        })
    }
}

fn ltv<P>(total_due: P, lease_asset: P) -> Percent
where
    P: Copy + Debug + PartialEq + Zero,
    Percent: Fractionable<P>,
{
    Percent::from_ratio(total_due, lease_asset)
}

impl Display for Strategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        fn dump(description: &str, arg: &Percent, f: &mut Formatter<'_>) -> FmtResult {
            f.write_str(description).and_then(|()| Display::fmt(arg, f))
        }

        match self {
            Strategy::TakeProfit(tp) => dump("take profit below ", tp, f),
            Strategy::StopLoss(sl) => dump("stop loss above or equal to ", sl, f),
        }
    }
}

#[cfg(test)]
mod test {

    mod may_trigger {
        use finance::{coin::Amount, percent::Percent};

        use crate::position::{close::Policy, CloseStrategy};

        #[test]
        fn no_sl_no_tp() {
            assert_eq!(None, may_trigger(None, None, 100, 67));
            assert_eq!(None, may_trigger(None, None, 100, 99));
        }

        #[test]
        fn sl_no_tp() {
            let sl_tvl = Percent::from_permille(567);
            assert_eq!(
                Some(CloseStrategy::StopLoss(sl_tvl)),
                may_trigger(Some(sl_tvl), None, 100, 67)
            );

            assert_eq!(
                Some(CloseStrategy::StopLoss(sl_tvl)),
                may_trigger(Some(sl_tvl), None, 1000, 568)
            );

            assert_eq!(
                Some(CloseStrategy::StopLoss(sl_tvl)),
                may_trigger(Some(sl_tvl), None, 1000, 567)
            );
            assert_eq!(None, may_trigger(Some(sl_tvl), None, 1000, 566));
        }

        #[test]
        fn no_sl_tp() {
            let tp_tvl = Percent::from_permille(342);
            assert_eq!(None, may_trigger(None, Some(tp_tvl), 100, 35));
            assert_eq!(None, may_trigger(None, Some(tp_tvl), 1000, 342));

            assert_eq!(
                Some(CloseStrategy::TakeProfit(tp_tvl)),
                may_trigger(None, Some(tp_tvl), 1000, 341)
            );

            assert_eq!(
                Some(CloseStrategy::TakeProfit(tp_tvl)),
                may_trigger(None, Some(tp_tvl), 1000, 336)
            );

            assert_eq!(
                Some(CloseStrategy::TakeProfit(tp_tvl)),
                may_trigger(None, Some(tp_tvl), 100, 20)
            );
        }

        #[test]
        fn sl_tp() {
            let sl_tvl = Percent::from_permille(567);
            let tp_tvl = Percent::from_permille(342);
            assert_eq!(
                Some(CloseStrategy::StopLoss(sl_tvl)),
                may_trigger(Some(sl_tvl), Some(tp_tvl), 100, 64)
            );

            assert_eq!(
                Some(CloseStrategy::StopLoss(sl_tvl)),
                may_trigger(Some(sl_tvl), Some(tp_tvl), 1000, 568)
            );

            assert_eq!(
                Some(CloseStrategy::StopLoss(sl_tvl)),
                may_trigger(Some(sl_tvl), Some(tp_tvl), 1000, 567)
            );

            assert_eq!(None, may_trigger(Some(sl_tvl), Some(tp_tvl), 1000, 566));
            assert_eq!(None, may_trigger(Some(sl_tvl), Some(tp_tvl), 100, 35));
            assert_eq!(None, may_trigger(Some(sl_tvl), Some(tp_tvl), 1000, 342));

            assert_eq!(
                Some(CloseStrategy::TakeProfit(tp_tvl)),
                may_trigger(Some(sl_tvl), Some(tp_tvl), 1000, 341)
            );

            assert_eq!(
                Some(CloseStrategy::TakeProfit(tp_tvl)),
                may_trigger(Some(sl_tvl), Some(tp_tvl), 1000, 336)
            );

            assert_eq!(
                Some(CloseStrategy::TakeProfit(tp_tvl)),
                may_trigger(Some(sl_tvl), Some(tp_tvl), 100, 20)
            );
        }

        fn may_trigger(
            sl: Option<Percent>,
            tp: Option<Percent>,
            asset: Amount,
            due: Amount,
        ) -> Option<CloseStrategy> {
            Policy {
                stop_loss: sl,
                take_profit: tp,
            }
            .may_trigger(asset, due)
        }
    }

    mod change_policy {
        use finance::{fraction::Fraction, percent::Percent};

        use crate::{
            api::position::{ChangeCmd, ClosePolicyChange},
            position::{close::Policy, error::Error as PositionError, CloseStrategy},
        };

        #[test]
        fn none() {
            assert_eq!(
                Ok(Policy::default()),
                Policy::default().change_policy(
                    ClosePolicyChange {
                        stop_loss: None,
                        take_profit: None,
                    },
                    1000,
                    200
                )
            );
        }

        #[test]
        fn stop_loss_set_reset() {
            let sl = Percent::from_percent(45);
            assert_eq!(
                Ok(Policy {
                    take_profit: None,
                    stop_loss: Some(sl),
                }),
                Policy::default().change_policy(
                    ClosePolicyChange {
                        take_profit: None,
                        stop_loss: Some(ChangeCmd::Set(sl)),
                    },
                    1000,
                    449
                )
            );

            assert_eq!(
                Ok(Policy::default()),
                Policy::default().change_policy(
                    ClosePolicyChange {
                        stop_loss: Some(ChangeCmd::Reset),
                        take_profit: None,
                    },
                    100,
                    45
                )
            );
        }

        #[test]
        fn take_profit_set_reset() {
            let tp = Percent::from_percent(45);
            assert_eq!(
                Ok(Policy {
                    take_profit: Some(tp),
                    stop_loss: None,
                }),
                Policy::default().change_policy(
                    ClosePolicyChange {
                        take_profit: Some(ChangeCmd::Set(tp)),
                        stop_loss: None,
                    },
                    100,
                    45
                )
            );

            assert_eq!(
                Ok(Policy::default()),
                Policy::default().change_policy(
                    ClosePolicyChange {
                        take_profit: Some(ChangeCmd::Reset),
                        stop_loss: None,
                    },
                    1000,
                    451
                )
            );
        }

        #[test]
        fn invariant_no_current_ltv() {
            let lower = Percent::from_percent(45);
            let higher = Percent::from_percent(55);

            let may_p = Policy::default().change_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(lower)),
                    stop_loss: Some(ChangeCmd::Set(higher)),
                },
                100,
                47,
            );
            assert_eq!(
                Ok(Policy {
                    take_profit: Some(lower),
                    stop_loss: Some(higher),
                }),
                may_p
            );

            let may_p_1 = may_p.unwrap().change_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Reset),
                    stop_loss: Some(ChangeCmd::Set(lower)),
                },
                1000,
                449,
            );
            assert_eq!(
                Ok(Policy {
                    take_profit: None,
                    stop_loss: Some(lower),
                }),
                may_p_1
            );

            let may_p_2 = may_p_1.unwrap().change_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(higher)),
                    stop_loss: None,
                },
                100,
                higher.of(100),
            );
            assert_eq!(
                Err(PositionError::trigger_close(
                    higher,
                    CloseStrategy::StopLoss(lower)
                )),
                may_p_2
            );
        }

        #[test]
        fn invariant_full() {
            const THOUSAND: u32 = 1000;
            let lower = Percent::from_percent(45);
            let higher = Percent::from_percent(55);
            let lease_invalid1 = higher - Percent::from_permille(1);
            let lease_invalid2 = lower;

            assert_eq!(
                Err(PositionError::trigger_close(
                    lease_invalid1,
                    CloseStrategy::TakeProfit(higher),
                )),
                Policy::default()
                    .change_policy(
                        ClosePolicyChange {
                            take_profit: Some(ChangeCmd::Set(lower)),
                            stop_loss: Some(ChangeCmd::Set(higher)),
                        },
                        THOUSAND,
                        lower.of(THOUSAND)
                    )
                    .unwrap()
                    .change_policy(
                        ClosePolicyChange {
                            take_profit: Some(ChangeCmd::Set(higher)),
                            stop_loss: Some(ChangeCmd::Reset),
                        },
                        THOUSAND,
                        lease_invalid1.of(THOUSAND)
                    )
            );

            assert_eq!(
                Err(PositionError::trigger_close(
                    lease_invalid2,
                    CloseStrategy::StopLoss(lower)
                )),
                Policy::default().change_policy(
                    ClosePolicyChange {
                        take_profit: None,
                        stop_loss: Some(ChangeCmd::Set(lease_invalid2)),
                    },
                    THOUSAND,
                    lease_invalid2.of(THOUSAND)
                )
            );
        }
    }

    mod display {
        use finance::percent::Percent;

        use crate::position::CloseStrategy;

        #[test]
        fn take_profit() {
            assert_eq!(
                "take profit below 45%",
                format!("{}", CloseStrategy::TakeProfit(Percent::from_percent(45)))
            )
        }

        #[test]
        fn stop_loss() {
            assert_eq!(
                "stop loss above or equal to 55.4%",
                format!("{}", CloseStrategy::StopLoss(Percent::from_permille(554)))
            )
        }
    }

    mod no_close {
        use finance::{
            percent::Percent,
            range::{Ascending, RightOpenRange},
        };

        use crate::position::close::Policy;

        #[test]
        fn unbound() {
            const GAP: Percent = Percent::from_permille(50);

            let below = Percent::from_percent(36);
            let tp_in = below - GAP - GAP;
            let sl_in = below - GAP;
            let tp_out = below + GAP;
            let sl_out = below + GAP + GAP;

            let range = RightOpenRange::up_to(below);
            no_close(None, None, range, range);

            no_close(Some(sl_out), None, range, range);
            no_close(Some(sl_in), None, range, RightOpenRange::up_to(sl_in));

            no_close(None, Some(tp_in), range, range.cut_to(tp_in));
            no_close(None, Some(tp_out), range, range.cut_to(tp_out)); //empty range!

            no_close(Some(sl_out), Some(tp_in), range, range.cut_to(tp_in));
            no_close(Some(sl_out), Some(tp_out), range, range.cut_to(tp_out)); //empty range!
            no_close(
                Some(sl_in),
                Some(tp_in),
                range,
                range.cut_to(tp_in).cut_from(sl_in),
            );
            no_close(Some(sl_in), Some(tp_out), range, range.cut_to(tp_out)); //empty range!
        }

        #[test]
        fn bound() {
            const GAP: Percent = Percent::from_permille(50);

            let below = Percent::from_percent(36);
            let above = below - GAP - GAP;
            let tp_out = above - GAP;
            let tp_in = above;
            let sl_in = below - GAP;
            let sl_out = below + GAP;

            let range = RightOpenRange::up_to(below).cut_to(above);
            no_close(None, None, range, range);

            no_close(Some(sl_out), None, range, range);
            no_close(Some(sl_in), None, range, range.cut_from(sl_in));

            no_close(None, Some(tp_in), range, range.cut_to(tp_in));
            no_close(None, Some(tp_out), range, range);

            no_close(Some(sl_out), Some(tp_in), range, range.cut_to(tp_in));
            no_close(Some(sl_out), Some(tp_out), range, range);
            no_close(
                Some(sl_in),
                Some(tp_in),
                range,
                range.cut_to(tp_in).cut_from(sl_in),
            );
            no_close(Some(sl_in), Some(tp_out), range, range.cut_from(sl_in));
        }

        fn no_close(
            sl: Option<Percent>,
            tp: Option<Percent>,
            during: RightOpenRange<Percent, Ascending>,
            exp: RightOpenRange<Percent, Ascending>,
        ) {
            assert_eq!(
                exp,
                Policy {
                    stop_loss: sl,
                    take_profit: tp,
                }
                .no_close(during)
            );
        }
    }
}
