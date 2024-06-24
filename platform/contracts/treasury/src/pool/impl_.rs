use finance::{duration::Duration, percent::Percent};
use lpp_platform::{CoinStable, Lpp as LppTrait, Stable};
use oracle_platform::Oracle;
use platform::message::Response as MessageResponse;

use crate::ContractError;

use super::Pool as PoolTrait;

pub struct Pool<Lpp, StableOracle> {
    _lpp: Lpp,
    _oracle: StableOracle,
    balance: CoinStable,
}

impl<Lpp, StableOracle> Pool<Lpp, StableOracle>
where
    Lpp: LppTrait,
    StableOracle: Oracle<Stable>,
{
    pub fn new(lpp: Lpp, oracle: StableOracle) -> Result<Self, ContractError> {
        lpp.balance(oracle.as_ref().addr().clone())
            .map_err(ContractError::ReadLppBalance)
            .map(|balance| Self {
                _lpp: lpp,
                _oracle: oracle,
                balance,
            })
    }
}

impl<Lpp, StableOracle> PoolTrait for Pool<Lpp, StableOracle>
where
    Lpp: LppTrait,
    StableOracle: Oracle<Stable>,
{
    fn balance(&self) -> CoinStable {
        self.balance
    }

    fn distribute_rewards(
        self,
        _apr: Percent,
        _period: Duration,
    ) -> Result<MessageResponse, ContractError> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use currency::NlsPlatform;
    use finance::{coin::Coin, duration::Duration, percent::Percent};
    use lpp_platform::{test::DummyLpp, CoinStable};
    use oracle_platform::test::DummyOracle;

    use crate::{
        pool::{Pool, PoolImpl},
        ContractError,
    };

    #[test]
    fn failing_balance() {
        let oracle = DummyOracle::failing();
        let lpp = DummyLpp::failing_balance();
        assert!(matches!(
            PoolImpl::new(lpp, oracle),
            Err(ContractError::ReadLppBalance(_))
        ));
    }

    #[test]
    fn failing_nls_price() {
        let bar0_apr = Percent::from_percent(20);
        let lpp0_tvl: CoinStable = 15_000.into();

        let lpp = DummyLpp::with_balance(lpp0_tvl, Coin::<NlsPlatform>::default());
        let oracle = DummyOracle::failing();

        let pool = PoolImpl::new(lpp, oracle).unwrap();
        assert_eq!(lpp0_tvl, pool.balance());

        assert!(matches!(
            pool.distribute_rewards(bar0_apr, Duration::YEAR),
            Err(ContractError::ConvertRewardsToNLS(_))
        ));
    }
}
