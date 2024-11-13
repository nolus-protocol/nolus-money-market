use finance::{fraction::Fraction, fractionable::Percentable, percent::Percent};
use serde::{Deserialize, Serialize};

use crate::api::position::{ChangeCmd, ClosePolicyChange};

use super::error::{Error as PositionError, Result as PositionResult};

/// Close position policy
///
/// Not designed to be used as an input API component! Invariant checks are not done on deserialization!
///
#[derive(Copy, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Policy {
    stop_loss: Option<Percent>,
    take_profit: Option<Percent>,
}

/// A strategy triggered to close the position automatically
///
/// If a recent price movement have the position's LTV trigger a full close as per the configured `Policy`
/// then the close strategy carries details.
///
/// A full close of the position is triggered if:
/// - a Stop Loss is set up and a price decline have the position's LTV become higher than the specified percent, or
/// - a Take Profit is set up and a price rise have the position's LTV become lower than the specified percent.
#[derive(PartialEq, Eq)]
#[cfg_attr(test, derive(Debug))]
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
    pub fn change_policy(self, cmd: ClosePolicyChange) -> PositionResult<Self> {
        Self {
            stop_loss: cmd
                .stop_loss
                .map_or_else(|| self.stop_loss, Option::<Percent>::from),
            take_profit: cmd
                .take_profit
                .map_or_else(|| self.take_profit, Option::<Percent>::from),
        }
        .check_invariant()
    }

    pub fn may_trigger<P>(&self, lease_asset: P, total_due: P) -> Option<Strategy>
    where
        P: Percentable + PartialOrd + Copy,
    {
        self.may_stop_loss(lease_asset, total_due)
            .or_else(|| self.may_take_profit(lease_asset, total_due))
    }

    fn check_invariant(self) -> PositionResult<Self> {
        self.stop_loss
            .map(|sl| {
                self.take_profit
                    .map(|tp| {
                        if sl >= tp {
                            Ok(self)
                        } else {
                            Err(PositionError::invalid_close_policy(sl, tp))
                        }
                    })
                    .unwrap_or(Ok(self))
            })
            .unwrap_or(Ok(self))
    }

    fn may_stop_loss<P>(&self, lease_asset: P, total_due: P) -> Option<Strategy>
    where
        P: Percentable + PartialOrd,
    {
        self.stop_loss.map_or(None, |stop_loss| {
            (stop_loss.of(lease_asset) <= total_due).then_some(Strategy::StopLoss(stop_loss))
        })
    }

    fn may_take_profit<P>(&self, lease_asset: P, total_due: P) -> Option<Strategy>
    where
        P: Percentable + PartialOrd,
    {
        self.take_profit.map_or(None, |take_profit| {
            (take_profit.of(lease_asset) > total_due).then_some(Strategy::TakeProfit(take_profit))
        })
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
        use finance::percent::Percent;

        use crate::{
            api::position::{ChangeCmd, ClosePolicyChange},
            position::{close::Policy, error::Error as PositionError},
        };

        #[test]
        fn none() {
            assert_eq!(
                Ok(Policy::default()),
                Policy::default().change_policy(ClosePolicyChange {
                    stop_loss: None,
                    take_profit: None,
                })
            );
        }

        #[test]
        fn stop_loss_set_reset() {
            let sl = Percent::from_percent(45);
            assert_eq!(
                Ok(Policy {
                    stop_loss: Some(sl),
                    take_profit: None,
                }),
                Policy::default().change_policy(ClosePolicyChange {
                    stop_loss: Some(ChangeCmd::Set(sl)),
                    take_profit: None,
                })
            );

            assert_eq!(
                Ok(Policy::default()),
                Policy::default().change_policy(ClosePolicyChange {
                    stop_loss: Some(ChangeCmd::Reset),
                    take_profit: None,
                })
            );
        }

        #[test]
        fn take_profit_set_reset() {
            let tp = Percent::from_percent(45);
            assert_eq!(
                Ok(Policy {
                    stop_loss: None,
                    take_profit: Some(tp),
                }),
                Policy::default().change_policy(ClosePolicyChange {
                    stop_loss: None,
                    take_profit: Some(ChangeCmd::Set(tp)),
                })
            );

            assert_eq!(
                Ok(Policy::default()),
                Policy::default().change_policy(ClosePolicyChange {
                    stop_loss: None,
                    take_profit: Some(ChangeCmd::Reset),
                })
            );
        }

        #[test]
        fn invariant() {
            let lower = Percent::from_percent(45);
            let higher = Percent::from_percent(55);

            let may_p = Policy::default().change_policy(ClosePolicyChange {
                stop_loss: Some(ChangeCmd::Set(higher)),
                take_profit: Some(ChangeCmd::Set(lower)),
            });
            assert_eq!(
                Ok(Policy {
                    stop_loss: Some(higher),
                    take_profit: Some(lower),
                }),
                may_p
            );

            let may_p_1 = may_p.unwrap().change_policy(ClosePolicyChange {
                stop_loss: Some(ChangeCmd::Set(lower)),
                take_profit: Some(ChangeCmd::Reset),
            });
            assert_eq!(
                Ok(Policy {
                    stop_loss: Some(lower),
                    take_profit: None,
                }),
                may_p_1
            );

            let may_p_2 = may_p_1.unwrap().change_policy(ClosePolicyChange {
                stop_loss: None,
                take_profit: Some(ChangeCmd::Set(higher)),
            });
            assert_eq!(
                Err(PositionError::invalid_close_policy(lower, higher)),
                may_p_2
            );
        }
    }
}
