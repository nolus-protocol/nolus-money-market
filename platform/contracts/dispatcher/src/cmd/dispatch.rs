use currency::NlsPlatform;
use finance::{coin::Coin, interest::InterestPeriod, period::Period};
use lpp_platform::{Lpp as LppTrait, UsdGroup};
use oracle_platform::{convert, OracleRef};
use platform::batch::Batch;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{result::ContractResult, state::Config, ContractError};

use super::{reward_calculator::Reward, Result as DispatcherResult, RewardCalculator};

pub(crate) struct Dispatch<'a> {
    last_dispatch: Timestamp,
    config: &'a Config,
    block_time: Timestamp,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a> Dispatch<'a> {
    pub fn new(
        last_dispatch: Timestamp,
        config: &'a Config,
        block_time: Timestamp,
        querier: &'a QuerierWrapper<'a>,
    ) -> Dispatch<'a> {
        Self {
            last_dispatch,
            config,
            block_time,
            querier,
        }
    }

    pub fn do_dispatch<Lpp>(self, lpp: &Lpp) -> ContractResult<DispatcherResult>
    where
        Lpp: LppTrait,
    {
        // get annual percentage of return from configuration
        let Reward {
            tvl,
            apr: apr_permille,
        } = RewardCalculator::new(&self.config.tvl_to_apr).calculate(lpp)?;

        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_in_usd = InterestPeriod::with_interest(apr_permille)
            .and_period(Period::from_till(self.last_dispatch, self.block_time))
            .interest(tvl);

        if reward_in_usd.is_zero() {
            return Err(ContractError::ZeroReward {});
        }

        OracleRef::try_from(self.config.oracle.clone(), self.querier)
            .and_then(|oracle| {
                convert::from_base::<_, _, UsdGroup>(oracle, reward_in_usd, self.querier)
            })
            .map_err(Into::into)
            .and_then(|reward_unls| {
                self.create_batch(reward_unls)
                    .map(|batch| DispatcherResult {
                        batch,
                        receipt: super::Receipt {
                            in_stable: reward_in_usd,
                            in_nls: reward_unls,
                        },
                    })
            })
    }

    fn create_batch(&self, reward: Coin<NlsPlatform>) -> ContractResult<Batch> {
        let mut batch = Batch::default();
        // Prepare a Send Rewards for the amount of Rewards_uNLS to the Treasury.
        batch
            .schedule_execute_wasm_no_reply::<_, NlsPlatform>(
                &self.config.treasury,
                treasury::msg::ExecuteMsg::SendRewards { amount: reward },
                None,
            )
            .map_err(ContractError::from)?;

        batch
            .schedule_execute_wasm_no_reply(
                &self.config.lpp,
                lpp_platform::msg::ExecuteMsg::DistributeRewards {},
                Some(reward),
            )
            .map_err(ContractError::from)?;

        Ok(batch)
    }
}
