use currency::NlsPlatform;
use finance::{coin::Coin, duration::Duration};
use lpp_platform::{Lpp as LppTrait, Usd};
use oracle_platform::Oracle as OracleTrait;
use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::Addr;

use crate::{result::ContractResult, state::reward_scale::RewardScale, ContractError};

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
    treasury: Addr,
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
        })
        .and_then(|(total, lpp_responses)| {
            if total.is_zero() {
                Err(ContractError::ZeroReward {})
            } else {
                // the total should precede the lpp messages!
                create_total(total, treasury).map(|resp| resp.merge_with(lpp_responses))
            }
        })
}

fn create_total(reward: Coin<NlsPlatform>, treasury: Addr) -> ContractResult<MessageResponse> {
    let mut batch = Batch::default();
    batch
        .schedule_execute_wasm_no_reply_no_funds(
            treasury,
            &treasury::msg::ExecuteMsg::SendRewards { amount: reward },
        )
        .map(|()| batch.into())
        .map_err(Into::into)
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
    use currency::{NativePlatform, NlsPlatform};
    use finance::{coin::Coin, duration::Duration, fraction::Fraction, percent::Percent, price};
    use lpp_platform::{test::DummyLpp, CoinUsd, Usd};
    use oracle_platform::{test::DummyOracle, Oracle as OracleTrait};
    use platform::{batch::Batch, response};
    use sdk::cosmwasm_std::Addr;

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
        let treasury = Addr::unchecked("Treasury");

        let resp = response::response_only_messages(
            super::dispatch(
                Duration::YEAR,
                &scale,
                lpps,
                oracles.iter(),
                treasury.clone(),
            )
            .unwrap(),
        );
        assert_eq!(resp.messages.len(), 3 + 1);
        let reward1 = reward(&apr, lpp0_tvl, &oracles[0]);
        let reward2 = reward(&apr, lpp1_tvl, &oracles[1]);
        let reward3 = reward(&apr, lpp2_tvl, &oracles[2]);
        let total_rewards = reward1 + reward2 + reward3;
        let mut msgs = Batch::default();
        msgs.schedule_execute_wasm_no_reply_no_funds(
            treasury,
            &treasury::msg::ExecuteMsg::SendRewards {
                amount: total_rewards,
            },
        )
        .unwrap();

        assert_eq!(
            resp.messages[0],
            response::response_only_messages(msgs).messages[0]
        );
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
        let treasury = Addr::unchecked("Treasury");

        let resp = super::dispatch(Duration::YEAR, &scale, lpps, oracles.iter(), treasury);
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
        let treasury = Addr::unchecked("Treasury");

        let resp = super::dispatch(Duration::YEAR, &scale, lpps, oracles.iter(), treasury);
        assert!(matches!(resp, Err(ContractError::Oracle(_))));
    }

    fn reward<Oracle>(apr: &Percent, tvl: CoinUsd, oracle: &Oracle) -> Coin<NlsPlatform>
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
    }
}
