use currency::NlsPlatform;
use finance::{coin::Coin, percent::Percent, period::Period};
use lpp_platform::{msg::LppBalanceResponse, CoinUsd, Lpp as LppTrait, Usd};
use oracle_platform::Oracle as OracleTrait;

use crate::{
    result::ContractResult,
    state::{reward_scale::RewardScale, rewards},
};

#[cfg_attr(test, derive(Debug))]
pub struct RewardCalculator {
    apr: Percent,
    tvls: Vec<CoinUsd>,
}

impl RewardCalculator {
    pub fn new<'lpp, Lpp, Lpps>(lpps: Lpps, scale: &RewardScale) -> ContractResult<Self>
    where
        Lpp: LppTrait + 'lpp,
        Lpps: IntoIterator,
        Lpps::Item: AsRef<Lpp>,
        Lpps::IntoIter: 'lpp,
    {
        let tvls: ContractResult<Vec<CoinUsd>> = lpps
            .into_iter()
            .map(|lpp| lpp.as_ref().balance())
            .map(|may_resp| {
                may_resp
                    .map(
                        |LppBalanceResponse {
                             balance,
                             total_principal_due,
                             total_interest_due,
                             ..
                         }| {
                            balance + total_principal_due + total_interest_due
                        },
                    )
                    .map_err(Into::into)
            })
            .collect();
        tvls.map(|tvls| Self {
            apr: scale.get_apr::<Usd, CoinUsd>(tvls.iter().sum()),
            tvls,
        })
    }

    pub fn apr(&self) -> Percent {
        self.apr
    }

    pub fn calculate<'o, Oracle, Oracles>(
        self,
        period: Period,
        oracles: Oracles,
    ) -> impl Iterator<Item = ContractResult<Coin<NlsPlatform>>> + 'o
    where
        Oracle: OracleTrait<Usd> + 'o,
        Oracles: IntoIterator,
        Oracles::Item: AsRef<Oracle>,
        Oracles::IntoIter: 'o,
    {
        rewards::calculate(self.apr(), period, self.tvls.into_iter().zip(oracles))
    }
}

#[cfg(test)]
mod tests {
    use currency::{NativePlatform, NlsPlatform};
    use finance::{
        duration::Duration, fraction::Fraction, percent::Percent, period::Period, price,
    };
    use lpp_platform::{test::DummyLpp, CoinUsd};
    use oracle_platform::{test::DummyOracle, Oracle};
    use sdk::cosmwasm_std::Timestamp;

    use crate::{
        state::reward_scale::{Bar, RewardScale, TotalValueLocked},
        ContractError,
    };

    use super::RewardCalculator;

    #[test]
    fn calc_apr() {
        let tvl_total = TotalValueLocked::new(54); //54k USD
        let bar0_apr = Percent::from_percent(20);
        let bar1_apr = Percent::from_percent(10);
        let scale = RewardScale::new(bar0_apr);
        let scale = scale
            .add_non_overlapping(vec![Bar {
                tvl: tvl_total,
                apr: bar1_apr,
            }])
            .unwrap();

        let lpp0_tvl: CoinUsd = TotalValueLocked::new(23).as_coin(); //23k USD
        {
            let lpp1_tvl = tvl_total.as_coin() - lpp0_tvl - 1.into();
            let lpps = vec![DummyLpp::with_tvl(lpp0_tvl), DummyLpp::with_tvl(lpp1_tvl)];
            assert_eq!(RewardCalculator::new(lpps, &scale).unwrap().apr(), bar0_apr);
        }
        {
            let lpp1_tvl = tvl_total.as_coin() - lpp0_tvl;
            let lpps = vec![DummyLpp::with_tvl(lpp0_tvl), DummyLpp::with_tvl(lpp1_tvl)];
            assert_eq!(RewardCalculator::new(lpps, &scale).unwrap().apr(), bar1_apr);
        }
    }

    #[test]
    fn calc_ok() {
        let bar0_apr = Percent::from_percent(20);
        let scale = RewardScale::new(bar0_apr);

        let lpp0_tvl: CoinUsd = 23_000.into();
        let lpp1_tvl = 3_000.into();
        let lpps = vec![DummyLpp::with_tvl(lpp0_tvl), DummyLpp::with_tvl(lpp1_tvl)];
        let calc = RewardCalculator::new(lpps, &scale).unwrap();
        assert_eq!(calc.apr(), bar0_apr);

        let oracles = vec![DummyOracle::with_price(2), DummyOracle::with_price(3)];
        let mut rewards = calc.calculate(year(), &oracles);
        assert_eq!(
            Some(Ok(price::total(
                bar0_apr.of(lpp0_tvl),
                oracles[0]
                    .price_of::<NlsPlatform, NativePlatform>()
                    .unwrap()
                    .inv()
            ))),
            rewards.next()
        );
        assert_eq!(
            Some(Ok(price::total(
                bar0_apr.of(lpp1_tvl),
                oracles[1]
                    .price_of::<NlsPlatform, NativePlatform>()
                    .unwrap()
                    .inv()
            ))),
            rewards.next()
        );
        assert_eq!(None, rewards.next());
    }

    #[test]
    fn calc_err() {
        let scale = RewardScale::new(Percent::from_percent(5));
        let lpps = vec![DummyLpp::with_tvl(1_234_567.into()), DummyLpp::failing()];
        assert!(matches!(
            RewardCalculator::new(lpps, &scale),
            Err(ContractError::LppPlatformError(_))
        ))
    }

    fn year() -> Period {
        Period::from_length(Timestamp::from_nanos(0), Duration::YEAR)
    }
}
