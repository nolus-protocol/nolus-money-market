use std::cmp::Ordering;

use cosmwasm_std::{StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::percent::Percent;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub struct Stop {
    pub tvl: u32,
    pub apr: Percent,
}

impl Stop {
    pub fn new(tvl: u32, apr: u32) -> Self {
        Stop {
            tvl,
            apr: Percent::from_permille(apr),
        }
    }
}
impl Ord for Stop {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tvl.cmp(&other.tvl)
    }
}

impl PartialOrd for Stop {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// A list of (minTVL_thousands: u32, APR%o) which defines the APR as per the TVL.
// The list represents intervals of TVL amounts starting from the given min TVL.
// A valid configuration shall include a pair with minTVL_thousands == 0.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Eq)]
pub struct Intervals {
    intervals: Vec<Stop>,
}
impl Intervals {
    pub fn new(initial_apr: u32) -> Self {
        Intervals {
            intervals: vec![Stop::new(0, initial_apr)],
        }
    }
    pub fn from(mut stops: Vec<Stop>) -> StdResult<Self> {
        if Intervals::is_valid(&stops) {
            stops.sort_by(|a, b| a.tvl.cmp(&b.tvl));
            return Ok(Intervals { intervals: stops });
        }
        Err(StdError::generic_err(""))
    }
    pub fn add(&mut self, mut stops: Vec<Stop>) {
        self.intervals.append(&mut stops);
    }
    fn is_valid(stops: &[Stop]) -> bool {
        stops.iter().any(|stop| stop.tvl == 0)
        // TODO: check for duplicated intervals
    }
    pub fn get_apr(&self, lpp_balance: u128) -> StdResult<Percent> {
        let idx = match self
            .intervals
            .binary_search(&Stop::new(lpp_balance as u32, 0))
        {
            Ok(i) => i,
            Err(e) => e - 1,
        };
        let arp = match self.intervals.get(idx) {
            Some(tvl) => tvl.apr,
            None => return Err(StdError::generic_err("ARP not found")),
        };

        Ok(arp)
    }
}

#[cfg(test)]
mod tests {
    use finance::percent::Percent;

    use crate::state::tvl_intervals::Stop;

    use super::Intervals;

    #[test]
    fn interval_new() {
        let cfg = Intervals::new(6);
        let initial = cfg.intervals.get(0).unwrap();
        assert_eq!(0, initial.tvl);
        assert_eq!(Percent::from_permille(6), initial.apr);
        assert_eq!(1, cfg.intervals.len());
    }

    #[test]
    fn interval_from() {
        let res = Intervals::from(vec![]);
        assert!(res.is_err());

        let res = Intervals::from(vec![Stop::new(30000, 6)]);
        assert!(res.is_err());

        let res = Intervals::from(vec![Stop::new(0, 6), Stop::new(30000, 10)]).unwrap();
        assert_eq!(res.intervals.len(), 2);
        assert_eq!(res.intervals.get(0).unwrap().tvl, 0);
        assert_eq!(res.intervals.get(0).unwrap().apr, Percent::from_permille(6));
        assert_eq!(res.intervals.get(1).unwrap().tvl, 30000);
        assert_eq!(
            res.intervals.get(1).unwrap().apr,
            Percent::from_permille(10)
        );
    }
    #[test]
    fn interval_get_apr() {
        let res = Intervals::from(vec![
            Stop::new(0, 6),
            Stop::new(30000, 10),
            Stop::new(150000, 15),
            Stop::new(3000000, 20),
            Stop::new(100000, 12),
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
