use currency::NlsPlatform;
use finance::{coin::Coin, duration::Duration};
use lpp_platform::{Lpp as LppTrait, Usd};
use oracle_platform::Oracle as OracleTrait;
use platform::message::Response as MessageResponse;

use crate::{result::ContractResult, state::reward_scale::RewardScale};

use super::RewardCalculator;

/// Dispatches the rewards to Lpp-s for further distribution among the liquidity providers
///
/// The total amount is transferred from the Treasury.
/// `lpps` and `oracles` should match in length.
pub fn dispatch<Lpps, Oracle, Oracles>(
    period: Duration,
    scale: &RewardScale,
    lpps: Lpps,
    oracles: Oracles,
) -> ContractResult<MessageResponse>
where
    Lpps: IntoIterator,
    Lpps::Item: LppTrait,
    Oracle: OracleTrait<Usd>,
    Oracles: IntoIterator,
    Oracles::Item: AsRef<Oracle>,
{
    let lpps: Vec<_> = lpps.into_iter().collect();
    RewardCalculator::new(lpps.iter(), scale)
        .and_then(|calc| {
            let rewards = calc.calculate(period, oracles);
            build_lpp_rewards(lpps, rewards).unwrap_or_else(|| Ok(Default::default()))
            // TODO stop calculating a total!
        })
        .map(|(_total, lpp_responses)| lpp_responses)
}

fn build_lpp_rewards<LppIter, RewardsIter>(
    lpps: LppIter,
    mut rewards: RewardsIter,
) -> Option<ContractResult<(Coin<NlsPlatform>, MessageResponse)>>
where
    LppIter: IntoIterator,
    LppIter::Item: LppTrait,
    RewardsIter: Iterator<Item = ContractResult<Coin<NlsPlatform>>>,
{
    let res = lpps
        .into_iter()
        .map(|lpp| {
            rewards
                .next()
                .expect("Lpps match oracles")
                .and_then(|reward| {
                    lpp.ditribute_rewards(reward)
                        .map(|response| (reward, response))
                        .map_err(Into::into)
                })
        })
        .reduce(|resp1, resp2| {
            resp1.and_then(|(reward1, lpp_resp1)| {
                resp2.map(|(reward2, lpp_resp2)| {
                    (reward1 + reward2, lpp_resp1.merge_with(lpp_resp2))
                })
            })
        });
    debug_assert_eq!(rewards.next(), None);
    res
}

#[cfg(test)]
mod test {
    use finance::{duration::Duration, percent::Percent};
    use lpp_platform::{test::DummyLpp, CoinUsd};
    use oracle_platform::test::DummyOracle;
    use platform::response;

    use crate::{
        state::reward_scale::{RewardScale, TotalValueLocked},
        ContractError,
    };

    #[test]
    fn dispatch() {
        let apr = Percent::from_percent(12);
        let scale = RewardScale::new(apr);
        let lpp0_tvl: CoinUsd = TotalValueLocked::new(50).as_coin(); //50k USD
        let lpp1_tvl: CoinUsd = TotalValueLocked::new(150).as_coin(); //150k USD
        let lpp2_tvl: CoinUsd = TotalValueLocked::new(200).as_coin(); //200k USD
        let lpps = [
            DummyLpp::with_tvl(lpp0_tvl),
            DummyLpp::with_tvl(lpp1_tvl),
            DummyLpp::with_tvl(lpp2_tvl),
        ];
        let oracles = [
            DummyOracle::with_price(2),
            DummyOracle::with_price(3),
            DummyOracle::with_price(1),
        ];

        let resp = response::response_only_messages(
            super::dispatch(Duration::YEAR, &scale, lpps, oracles.iter()).unwrap(),
        );
        assert_eq!(resp.messages.len(), 3);
        assert_eq!(resp.events.len(), 3);
    }

    #[test]
    fn balance_err() {
        let apr = Percent::from_percent(12);
        let scale = RewardScale::new(apr);
        let lpp0_tvl: CoinUsd = TotalValueLocked::new(50).as_coin(); //50k USD
        let lpp2_tvl: CoinUsd = TotalValueLocked::new(200).as_coin(); //200k USD
        let lpps = [
            DummyLpp::with_tvl(lpp0_tvl),
            DummyLpp::failing(),
            DummyLpp::with_tvl(lpp2_tvl),
        ];
        let oracles = [
            DummyOracle::with_price(2),
            DummyOracle::with_price(3),
            DummyOracle::with_price(1),
        ];

        let resp = super::dispatch(Duration::YEAR, &scale, lpps, oracles.iter());
        assert!(matches!(resp, Err(ContractError::LppPlatformError(_))));
    }

    #[test]
    fn oracle_err() {
        let apr = Percent::from_percent(12);
        let scale = RewardScale::new(apr);
        let lpp0_tvl: CoinUsd = TotalValueLocked::new(50).as_coin(); //50k USD
        let lpp1_tvl: CoinUsd = TotalValueLocked::new(150).as_coin(); //150k USD
        let lpp2_tvl: CoinUsd = TotalValueLocked::new(200).as_coin(); //200k USD
        let lpps = [
            DummyLpp::with_tvl(lpp0_tvl),
            DummyLpp::with_tvl(lpp1_tvl),
            DummyLpp::with_tvl(lpp2_tvl),
        ];
        let oracles = [
            DummyOracle::with_price(2),
            DummyOracle::with_price(3),
            DummyOracle::failing(),
        ];

        let resp = super::dispatch(Duration::YEAR, &scale, lpps, oracles.iter());
        assert!(matches!(resp, Err(ContractError::Oracle(_))));
    }
}
