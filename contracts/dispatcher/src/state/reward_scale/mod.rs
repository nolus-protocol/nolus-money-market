use serde::{Deserialize, Serialize};

use finance::{coin::Amount, percent::Percent};
use sdk::{
    cosmwasm_std::{StdError, StdResult},
    schemars::{self, JsonSchema},
};

mod unchecked;

#[derive(
    Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default, Serialize, Deserialize, JsonSchema,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct TotalValueLocked(u32);

impl TotalValueLocked {
    pub fn new(thousands: u32) -> Self {
        Self(thousands)
    }

    pub fn to_amount(&self) -> Amount {
        Amount::from(self.0) * 1000
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct Bar {
    pub tvl: TotalValueLocked,
    pub apr: Percent,
}

// A list of (minTVL_thousands: u32, APR%o) which defines the APR as per the TVL.
// The list represents intervals of TVL amounts starting from the given min TVL.
// A valid configuration shall include a pair with minTVL_thousands == 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "unchecked::RewardScale", into = "unchecked::RewardScale")]
pub struct RewardScale {
    bars: Vec<Bar>,
}

impl RewardScale {
    pub fn new(initial_apr: Percent) -> Self {
        RewardScale {
            bars: vec![Bar {
                tvl: Default::default(),
                apr: initial_apr,
            }],
        }
    }

    pub fn add(&mut self, mut bars: Vec<Bar>) -> StdResult<()> {
        if bars.is_empty() {
            return Err(StdError::generic_err("Argument vector contains no bars!"));
        }

        if bars.iter().any(|bar| {
            self.bars
                .binary_search_by_key(&bar.tvl, |bar| bar.tvl)
                .is_ok()
        }) {
            return Err(StdError::generic_err(
                "Argument bars duplicate already defined bars!",
            ));
        }

        self.bars.append(&mut bars);

        self.bars.sort_unstable();

        Ok(())
    }

    pub fn get_apr(&self, lpp_balance: Amount) -> Percent {
        self.bars[self
            .bars
            .partition_point(|bar| bar.tvl.to_amount() <= lpp_balance)
            .saturating_sub(1)]
        .apr
    }
}

impl TryFrom<Vec<Bar>> for RewardScale {
    type Error = StdError;

    fn try_from(mut bars: Vec<Bar>) -> Result<Self, Self::Error> {
        if !bars.iter().any(|bar| bar.tvl == Default::default()) {
            return Err(StdError::generic_err("No zero TVL reward scale bar found!"));
        }

        bars.sort_unstable();

        if bars
            .iter()
            .zip(bars.iter().skip(1))
            .any(|(left, right)| left.tvl == right.tvl)
        {
            return Err(StdError::generic_err("Duplicate reward scales found!"));
        }

        Ok(RewardScale { bars })
    }
}

#[cfg(test)]
mod tests {
    use finance::{coin::Amount, percent::Percent};

    use super::{Bar, RewardScale, TotalValueLocked};

    #[test]
    fn rewards_scale_new() {
        let cfg = RewardScale::new(Percent::from_permille(6));
        let initial = cfg.bars.first().unwrap();
        assert_eq!(initial.tvl, Default::default());
        assert_eq!(initial.apr, Percent::from_permille(6));
        assert_eq!(cfg.bars.len(), 1);
    }

    #[test]
    fn rewards_from() {
        let _ = RewardScale::try_from(vec![]).unwrap_err();

        let _ = RewardScale::try_from(vec![Bar {
            tvl: TotalValueLocked::new(30),
            apr: Percent::from_permille(6),
        }])
        .unwrap_err();

        let res = RewardScale::try_from(vec![
            Bar {
                tvl: Default::default(),
                apr: Percent::from_permille(6),
            },
            Bar {
                tvl: TotalValueLocked::new(30),
                apr: Percent::from_permille(10),
            },
        ])
        .unwrap();

        assert_eq!(res.bars.len(), 2);
        assert_eq!(res.bars[0].tvl, Default::default());
        assert_eq!(res.bars[0].apr, Percent::from_permille(6));
        assert_eq!(res.bars[1].tvl, TotalValueLocked::new(30));
        assert_eq!(res.bars[1].apr, Percent::from_permille(10));
    }

    #[test]
    fn rewards_scale_get_apr() {
        let res = RewardScale::try_from(vec![
            Bar {
                tvl: Default::default(),
                apr: Percent::from_permille(6),
            },
            Bar {
                tvl: TotalValueLocked::new(30),
                apr: Percent::from_permille(10),
            },
            Bar {
                tvl: TotalValueLocked::new(150),
                apr: Percent::from_permille(15),
            },
            Bar {
                tvl: TotalValueLocked::new(300),
                apr: Percent::from_permille(20),
            },
            Bar {
                tvl: TotalValueLocked::new(100),
                apr: Percent::from_permille(12),
            },
        ])
        .unwrap();

        assert_eq!(res.get_apr(0), Percent::from_permille(6));
        assert_eq!(res.get_apr(1000), Percent::from_permille(6));
        assert_eq!(res.get_apr(29999), Percent::from_permille(6));
        assert_eq!(res.get_apr(30000), Percent::from_permille(10));
        assert_eq!(res.get_apr(30001), Percent::from_permille(10));
        assert_eq!(res.get_apr(100051), Percent::from_permille(12));
        assert_eq!(res.get_apr(149999), Percent::from_permille(12));
        assert_eq!(res.get_apr(150000), Percent::from_permille(15));
        assert_eq!(res.get_apr(2000300), Percent::from_permille(15));
        assert_eq!(res.get_apr(3000000), Percent::from_permille(20));
        assert_eq!(res.get_apr(3000200), Percent::from_permille(20));
        assert_eq!(res.get_apr(13000200), Percent::from_permille(20));
        assert_eq!(res.get_apr(Amount::MAX), Percent::from_permille(20));
        assert_eq!(res.get_apr(Amount::MIN), Percent::from_permille(6));
    }
}
