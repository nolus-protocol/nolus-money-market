use serde::{Deserialize, Serialize};

use finance::{coin::Amount, percent::Percent};
use sdk::{
    cosmwasm_std::{StdError, StdResult},
    schemars::{self, JsonSchema},
};

mod unchecked;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
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

    pub fn add(&mut self, mut bars: Vec<Bar>) {
        self.bars.append(&mut bars);
    }

    pub fn get_apr(&self, lpp_balance: Amount) -> StdResult<Percent> {
        self.bars
            .binary_search_by_key(&lpp_balance, |bar| bar.tvl.into())
            .map_or_else(|index| index.checked_sub(1), Some)
            .and_then(|index| self.bars.get(index).map(|bar| bar.apr))
            .ok_or_else(|| StdError::generic_err("ARP not found"))
    }
}

impl TryFrom<Vec<Bar>> for RewardScale {
    type Error = StdError;

    fn try_from(mut bars: Vec<Bar>) -> Result<Self, Self::Error> {
        if !bars.iter().any(|bar| bar.tvl == u32::default()) {
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
    use finance::coin::Amount;
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

        let res = RewardScale::try_from(vec![Bar::new(0, 6), Bar::new(30000, 10)]).unwrap();
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
        assert_eq!(
            res.get_apr(Amount::MAX).unwrap(),
            Percent::from_permille(20)
        );
        assert_eq!(res.get_apr(Amount::MIN).unwrap(), Percent::from_permille(6));
    }
}
