use finance::{duration::Duration, percent::Percent100};
use lpp_platform::CoinStable;
use platform::message::Response as MessageResponse;

use crate::{ContractError, pool::Pool as PoolTrait, state::reward_scale::RewardScale};

// TODO rename to Rewards and move out of 'cmd'
#[cfg_attr(test, derive(Debug))]
pub struct RewardCalculator<Pool> {
    pools: Vec<Pool>,
    apr: Percent100,
}

impl<Pool> RewardCalculator<Pool>
where
    Pool: PoolTrait,
{
    pub fn new<Pools>(pools: Pools, scale: &RewardScale) -> Self
    where
        Pools: IntoIterator<Item = Pool>,
    {
        let mut tvls_total = CoinStable::default();
        let pools = pools
            .into_iter()
            .inspect(|pool| tvls_total += pool.balance())
            .collect();
        Self {
            pools,
            apr: scale.get_apr(tvls_total),
        }
    }

    pub fn apr(&self) -> Percent100 {
        self.apr
    }

    pub fn distribute(self, period: Duration) -> Result<MessageResponse, ContractError> {
        self.pools
            .into_iter()
            .map(|pool| pool.distribute_rewards(self.apr, period))
            // use a short-circuiting fn here, avoiding swallowing of errors
            .try_fold(MessageResponse::default(), |resp1, resp2| {
                resp2.map(|lpp_resp2| resp1.merge_with(lpp_resp2))
            })
    }
}

#[cfg(test)]
mod tests {
    use finance::{coin::Coin, duration::Duration, percent::Percent100};
    use lpp_platform::CoinStable;
    use platform::response;

    use crate::{
        ContractError,
        pool::mock::MockPool,
        state::reward_scale::{Bar, RewardScale, TotalValueLocked},
    };

    use super::RewardCalculator;

    #[test]
    fn calc_apr() {
        let tvl_total = TotalValueLocked::new(54); //54k USD
        let bar0_apr = Percent100::from_percent(20);
        let bar1_apr = Percent100::from_percent(10);
        let scale = RewardScale::new(bar0_apr);
        let scale = scale
            .add_non_overlapping(vec![Bar {
                tvl: tvl_total,
                apr: bar1_apr,
            }])
            .unwrap();

        let lpp0_tvl: CoinStable = TotalValueLocked::new(23).as_coin(); //23k USD
        {
            let lpp1_tvl = tvl_total.as_coin() - lpp0_tvl - Coin::new(1);
            let lpps = vec![
                MockPool::reward_none(lpp0_tvl),
                MockPool::reward_none(lpp1_tvl),
            ];
            assert_eq!(RewardCalculator::new(lpps, &scale).apr(), bar0_apr);
        }
        {
            let lpp1_tvl = tvl_total.as_coin() - lpp0_tvl;
            let lpps = vec![
                MockPool::reward_none(lpp0_tvl),
                MockPool::reward_none(lpp1_tvl),
            ];
            assert_eq!(RewardCalculator::new(lpps, &scale).apr(), bar1_apr);
        }
    }

    #[test]
    fn calc_ok() {
        let bar0_apr = Percent100::from_percent(20);
        let scale = RewardScale::new(bar0_apr);
        let period = Duration::YEAR;

        let lpp0_tvl: CoinStable = Coin::new(23_000);
        let lpp1_tvl = Coin::new(3_000);
        let lpps = vec![
            MockPool::reward_ok(lpp0_tvl, bar0_apr, period),
            MockPool::reward_ok(lpp1_tvl, bar0_apr, period),
        ];
        let calc = RewardCalculator::new(lpps, &scale);
        assert_eq!(calc.apr(), bar0_apr);

        let resp = response::response_only_messages(calc.distribute(period).unwrap());
        assert_eq!(resp.messages.len(), 2);
        assert_eq!(resp.events.len(), 2);
    }

    #[test]
    fn distribute_err() {
        let bar0_apr = Percent100::from_percent(5);
        let scale = RewardScale::new(bar0_apr);
        let period = Duration::from_days(134);

        let lpp0_tvl: CoinStable = Coin::new(23_000);
        let lpp1_tvl = Coin::new(3_000);
        let lpps = vec![
            MockPool::reward_fail(lpp0_tvl, bar0_apr, period),
            MockPool::reward_none(lpp1_tvl),
        ];

        let calc = RewardCalculator::new(lpps, &scale);
        assert_eq!(calc.apr(), bar0_apr);
        assert!(matches!(
            calc.distribute(period),
            Err(ContractError::DistributeLppReward(_))
        ))
    }
}
