use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use finance::percent::Percent;
use sdk::{
    cosmwasm_std::{StdError, StdResult},
    schemars::{self, JsonSchema},
};

mod unchecked;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Bar {
    pub tvl: u32,
    pub apr: Percent,
}

impl Bar {
    pub fn new(tvl: u32, apr: u32) -> Self {
        Bar {
            tvl,
            apr: Percent::from_permille(apr),
        }
    }
}

impl PartialOrd for Bar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Bar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tvl.cmp(&other.tvl)
    }
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
    pub fn new(initial_apr: u32) -> Self {
        RewardScale {
            bars: vec![Bar::new(0, initial_apr)],
        }
    }

    pub fn add(&mut self, mut stops: Vec<Bar>) {
        self.bars.append(&mut stops);
    }

    pub fn get_apr(&self, lpp_balance: u128) -> StdResult<Percent> {
        let idx = match self
            .bars
            .binary_search(&Bar::new(lpp_balance as u32, 0))
        {
            Ok(i) => i,
            Err(e) => e - 1,
        };
        let arp = match self.bars.get(idx) {
            Some(tvl) => tvl.apr,
            None => return Err(StdError::generic_err("ARP not found")),
        };

        Ok(arp)
    }
}

impl TryFrom<Vec<Bar>> for RewardScale {
    type Error = StdError;

    fn try_from(mut reward_scale: Vec<Bar>) -> Result<Self, Self::Error> {
        if !reward_scale.iter().any(|bar| bar.tvl == 0) {
            return Err(StdError::generic_err("No zero TVL reward scale bar found!"));
        }

        reward_scale.sort_unstable();

        if reward_scale
            .iter()
            .zip(reward_scale.iter().skip(1))
            .any(|(left, right)| left.tvl == right.tvl)
        {
            return Err(StdError::generic_err("Duplicate reward scales found!"));
        }

        Ok(RewardScale {
            bars: reward_scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use finance::percent::Percent;

    use crate::state::reward_scale::Bar;

    use super::RewardScale;

    #[test]
    fn interval_new() {
        let cfg = RewardScale::new(6);
        let initial = cfg.bars.get(0).unwrap();
        assert_eq!(0, initial.tvl);
        assert_eq!(Percent::from_permille(6), initial.apr);
        assert_eq!(1, cfg.bars.len());
    }

    #[test]
    fn interval_from() {
        let res = RewardScale::try_from(vec![]);
        assert!(res.is_err());

        let res = RewardScale::try_from(vec![Bar::new(30000, 6)]);
        assert!(res.is_err());

        let res = RewardScale::try_from(vec![Bar::new(0, 6), Bar::new(30000, 10)])
            .unwrap();
        assert_eq!(res.bars.len(), 2);
        assert_eq!(res.bars.get(0).unwrap().tvl, 0);
        assert_eq!(res.bars.get(0).unwrap().apr, Percent::from_permille(6));
        assert_eq!(res.bars.get(1).unwrap().tvl, 30000);
        assert_eq!(res.bars.get(1).unwrap().apr, Percent::from_permille(10));
    }
    #[test]
    fn interval_get_apr() {
        let res = RewardScale::try_from(vec![
            Bar::new(0, 6),
            Bar::new(30000, 10),
            Bar::new(150000, 15),
            Bar::new(3000000, 20),
            Bar::new(100000, 12),
        ])
        .unwrap();
        assert_eq!(res.get_apr(0).unwrap(), Percent::from_permille(6));
        assert_eq!(res.get_apr(1000).unwrap(), Percent::from_permille(6));
        assert_eq!(res.get_apr(29999).unwrap(), Percent::from_permille(6));
        assert_eq!(res.get_apr(30000).unwrap(), Percent::from_permille(10));
        assert_eq!(res.get_apr(30001).unwrap(), Percent::from_permille(10));
        assert_eq!(res.get_apr(100051).unwrap(), Percent::from_permille(12));
        assert_eq!(res.get_apr(149999).unwrap(), Percent::from_permille(12));
        assert_eq!(res.get_apr(150000).unwrap(), Percent::from_permille(15));
        assert_eq!(res.get_apr(2000300).unwrap(), Percent::from_permille(15));
        assert_eq!(res.get_apr(3000000).unwrap(), Percent::from_permille(20));
        assert_eq!(res.get_apr(3000200).unwrap(), Percent::from_permille(20));
        assert_eq!(res.get_apr(13000200).unwrap(), Percent::from_permille(20));
        assert_eq!(res.get_apr(u128::MAX).unwrap(), Percent::from_permille(20));
        assert_eq!(res.get_apr(u128::MIN).unwrap(), Percent::from_permille(6));
    }
}
