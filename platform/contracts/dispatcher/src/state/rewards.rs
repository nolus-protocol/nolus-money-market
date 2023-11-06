use currency::{NativePlatform, NlsPlatform};
use finance::{coin::Coin, interest::InterestPeriod, percent::Percent, period::Period};
use lpp_platform::{CoinUsd, Usd, UsdGroup};
use oracle_platform::{convert, Oracle};

use crate::result::ContractResult;

pub(crate) fn calculate<'a, Dexes, DexOracleRef, DexOracle>(
    apr: Percent,
    period: Period,
    tvls_oracles: Dexes,
) -> impl Iterator<Item = ContractResult<Coin<NlsPlatform>>> + 'a
where
    Dexes: IntoIterator<Item = (CoinUsd, DexOracleRef)>,
    Dexes::IntoIter: 'a,
    DexOracleRef: AsRef<DexOracle>,
    DexOracle: Oracle<Usd> + 'a,
{
    let interest_period = InterestPeriod::with_interest(apr).and_period(period);
    tvls_oracles.into_iter().map(move |tvl_oracle| {
        let reward_in_usd = interest_period.interest(tvl_oracle.0);
        convert::from_base::<_, UsdGroup, _, _, NativePlatform>(
            tvl_oracle.1.as_ref(),
            reward_in_usd,
        )
        .map_err(Into::into)
    })
}

#[cfg(test)]
mod test {
    use currency::{NativePlatform, NlsPlatform};
    use finance::{
        coin::Coin, duration::Duration, fraction::Fraction, percent::Percent, period::Period, price,
    };
    use lpp_platform::Usd;
    use oracle_platform::{test::DummyOracle, Oracle as OracleTrait};
    use sdk::cosmwasm_std::Timestamp;

    use crate::ContractError;

    #[test]
    fn calculate_ok() {
        let apr = Percent::from_percent(20);
        let period = Period::from_length(Timestamp::from_nanos(239471696), Duration::from_days(1));
        let lpp_tvls: Vec<Coin<Usd>> = vec![25_000.into(), 8_000.into()];
        let oracles = vec![DummyOracle::with_price(2), DummyOracle::with_price(3)];

        let mut rewards = super::calculate(
            apr,
            period,
            lpp_tvls.clone().into_iter().zip(oracles.iter()),
        );
        assert_eq!(
            Some(Ok(reward(apr, lpp_tvls[0], &oracles[0]))),
            rewards.next()
        );
        assert_eq!(
            Some(Ok(reward(apr, lpp_tvls[1], &oracles[1]))),
            rewards.next()
        );
        assert_eq!(None, rewards.next());
    }

    #[test]
    fn calculate_no_price() {
        let apr = Percent::from_percent(20);
        let period = Period::from_length(Timestamp::from_nanos(239471696), Duration::from_days(1));
        let lpp_tvls: Vec<Coin<Usd>> = vec![25_000.into(), 8_000.into()];
        let oracles = vec![DummyOracle::with_price(2), DummyOracle::failing()];

        let mut rewards = super::calculate(
            apr,
            period,
            lpp_tvls.clone().into_iter().zip(oracles.iter()),
        );
        assert_eq!(
            Some(Ok(reward(apr, lpp_tvls[0], &oracles[0]))),
            rewards.next()
        );
        assert!(matches!(
            rewards.next(),
            Some(Err(ContractError::Oracle(_)))
        ));
        assert_eq!(None, rewards.next());
    }

    fn reward<Oracle>(apr: Percent, tvl: Coin<Usd>, oracle: &Oracle) -> Coin<NlsPlatform>
    where
        Oracle: OracleTrait<Usd>,
    {
        price::total(
            apr.of(tvl),
            oracle
                .price_of::<NlsPlatform, NativePlatform>()
                .unwrap()
                .inv(),
        )
        .checked_div(365)
        .unwrap()
    }
}
