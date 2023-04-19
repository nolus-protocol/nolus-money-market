use currency::native::Nls;
use finance::{coin::Coin, currency::Currency, duration::Duration, interest::InterestPeriod};
use lpp::stub::{Lpp as LppTrait, WithLpp};
use oracle::{convert, stub::OracleRef};
use platform::batch::Batch;
use sdk::cosmwasm_std::{QuerierWrapper, StdResult, Timestamp};

use crate::{result::ContractResult, state::Config, ContractError};

use super::{query_reward_scale::ActiveRewardScale, QueryRewardScale, Result as DispatcherResult};

pub struct Dispatch<'a> {
    last_dispatch: Timestamp,
    config: Config,
    block_time: Timestamp,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> Dispatch<'a> {
    pub fn new(
        last_dispatch: Timestamp,
        config: Config,
        block_time: Timestamp,
        querier: &'a QuerierWrapper<'a>,
    ) -> StdResult<Dispatch<'a>> {
        Ok(Self {
            last_dispatch,
            config,
            block_time,
            querier,
        })
    }

    fn create_batch(&self, reward: Coin<Nls>) -> ContractResult<Batch> {
        let mut batch = Batch::default();
        // Prepare a Send Rewards for the amount of Rewards_uNLS to the Treasury.
        batch
            .schedule_execute_wasm_no_reply::<_, Nls>(
                &self.config.treasury,
                treasury::msg::ExecuteMsg::SendRewards { amount: reward },
                None,
            )
            .map_err(ContractError::from)?;

        batch
            .schedule_execute_wasm_no_reply(
                &self.config.lpp,
                lpp::msg::ExecuteMsg::DistributeRewards {},
                Some(reward),
            )
            .map_err(ContractError::from)?;

        Ok(batch)
    }
}

impl<'a> WithLpp for Dispatch<'a> {
    type Output = DispatcherResult;
    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        // get annual percentage of return from configuration
        let ActiveRewardScale {
            tvl,
            apr: apr_permille,
        } = QueryRewardScale::new(&self.config.tvl_to_apr).reward_scale(&lpp)?;

        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_in_lppdenom = InterestPeriod::with_interest(apr_permille)
            .from(self.last_dispatch)
            .spanning(Duration::between(self.last_dispatch, self.block_time))
            .interest(tvl);

        if reward_in_lppdenom.is_zero() {
            return Err(ContractError::ZeroReward {});
        }

        OracleRef::try_from(self.config.oracle.clone(), self.querier)
            .and_then(|oracle| convert::from_base(oracle, reward_in_lppdenom, self.querier))
            .map_err(Into::into)
            .and_then(|reward_unls| {
                self.create_batch(reward_unls)
                    .map(|batch| DispatcherResult {
                        batch,
                        receipt: super::Receipt {
                            in_stable: reward_in_lppdenom.into(),
                            in_nls: reward_unls.into(),
                        },
                    })
            })
    }
}
