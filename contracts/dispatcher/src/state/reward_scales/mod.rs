use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use finance::percent::Percent;
use sdk::{
    cosmwasm_std::{StdError, StdResult},
    schemars::{self, JsonSchema},
};

mod unchecked;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RewardScale {
    pub tvl: u32,
    pub apr: Percent,
}

impl RewardScale {
    pub fn new(tvl: u32, apr: u32) -> Self {
        RewardScale {
            tvl,
            apr: Percent::from_permille(apr),
        }
    }
}

impl PartialOrd for RewardScale {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RewardScale {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tvl.cmp(&other.tvl)
    }
}

// A list of (minTVL_thousands: u32, APR%o) which defines the APR as per the TVL.
// The list represents intervals of TVL amounts starting from the given min TVL.
// A valid configuration shall include a pair with minTVL_thousands == 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "unchecked::RewardScales", into = "unchecked::RewardScales")]
pub struct RewardScales {
    scales: Vec<RewardScale>,
}

impl RewardScales {
    pub fn new(initial_apr: u32) -> Self {
        RewardScales {
            scales: vec![RewardScale::new(0, initial_apr)],
        }
    }

    pub fn add(&mut self, mut stops: Vec<RewardScale>) {
        self.scales.append(&mut stops);
    }

    pub fn get_apr(&self, lpp_balance: u128) -> StdResult<Percent> {
        let idx = match self
            .scales
            .binary_search(&RewardScale::new(lpp_balance as u32, 0))
        {
            Ok(i) => i,
            Err(e) => e - 1,
        };
        let arp = match self.scales.get(idx) {
            Some(tvl) => tvl.apr,
            None => return Err(StdError::generic_err("ARP not found")),
        };

        Ok(arp)
    }
}

impl TryFrom<Vec<RewardScale>> for RewardScales {
    type Error = StdError;

    fn try_from(mut reward_scales: Vec<RewardScale>) -> Result<Self, Self::Error> {
        if !reward_scales.iter().any(|reward_scale| reward_scale.tvl == 0) {
            return Err(StdError::generic_err("No zero TVL reward scale found!"));
        }

        reward_scales.sort_unstable();

        if reward_scales
            .iter()
            .zip(reward_scales.iter().skip(1))
            .any(|(left, right)| left.tvl == right.tvl)
        {
            return Err(StdError::generic_err("Duplicate reward scales found!"));
        }

        Ok(RewardScales {
            scales: reward_scales,
        })
    }
}

#[cfg(test)]
mod tests {
    use finance::percent::Percent;

    use crate::state::reward_scales::RewardScale;

    use super::RewardScales;

    #[test]
    fn interval_new() {
        let cfg = RewardScales::new(6);
        let initial = cfg.scales.get(0).unwrap();
        assert_eq!(0, initial.tvl);
        assert_eq!(Percent::from_permille(6), initial.apr);
        assert_eq!(1, cfg.scales.len());
    }

    #[test]
    fn interval_from() {
        let res = RewardScales::try_from(vec![]);
        assert!(res.is_err());

        let res = RewardScales::try_from(vec![RewardScale::new(30000, 6)]);
        assert!(res.is_err());

        let res = RewardScales::try_from(vec![RewardScale::new(0, 6), RewardScale::new(30000, 10)])
            .unwrap();
        assert_eq!(res.scales.len(), 2);
        assert_eq!(res.scales.get(0).unwrap().tvl, 0);
        assert_eq!(res.scales.get(0).unwrap().apr, Percent::from_permille(6));
        assert_eq!(res.scales.get(1).unwrap().tvl, 30000);
        assert_eq!(res.scales.get(1).unwrap().apr, Percent::from_permille(10));
    }
    #[test]
    fn interval_get_apr() {
        let res = RewardScales::try_from(vec![
            RewardScale::new(0, 6),
            RewardScale::new(30000, 10),
            RewardScale::new(150000, 15),
            RewardScale::new(3000000, 20),
            RewardScale::new(100000, 12),
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
