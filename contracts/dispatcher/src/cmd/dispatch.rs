use cosmwasm_std::{QuerierWrapper, StdResult, Timestamp};
use serde::Serialize;

use currency::native::Nls;
use finance::{coin::Coin, currency::Currency, duration::Duration, interest::InterestPeriod};
use lpp::stub::{Lpp as LppTrait, WithLpp};
use oracle::{convert, stub::OracleRef};
use platform::batch::{Batch, Emit, Emitter};

use crate::{cmd::Result as DispatcherResult, state::Config, ContractError};

use super::Dispatch;

impl<'a> WithLpp for Dispatch<'a> {
    type Output = Emitter;
    type Error = ContractError;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let resp = lpp.lpp_balance()?;
        let tvl: Coin<Lpn> = resp.balance + resp.total_principal_due + resp.total_interest_due;

        // get annual percentage of return from configuration
        let arp_permille = self.config.tvl_to_apr.get_apr(tvl.into())?;

        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_in_lppdenom = InterestPeriod::with_interest(arp_permille)
            .from(self.last_dispatch)
            .spanning(Duration::between(self.last_dispatch, self.block_time))
            .interest(tvl);

        if reward_in_lppdenom.is_zero() {
            return Err(ContractError::ZeroReward {});
        }

        let reward_unls =
            convert::from_base(self.oracle_ref.clone(), reward_in_lppdenom, &self.querier)?;

        let result = DispatcherResult {
            batch: self.create_response(reward_unls)?,
            receipt: super::Receipt {
                in_stable: reward_in_lppdenom,
                in_nls: reward_unls,
            },
        };
        Ok(result
            .batch
            .into_emitter("tr-rewards")
            .emit_coin("rewards", result.receipt.in_nls))
    }
}

impl<'a> Dispatch<'a> {
    pub fn new(
        oracle_ref: OracleRef,
        last_dispatch: Timestamp,
        config: Config,
        block_time: Timestamp,
        querier: QuerierWrapper<'a>,
    ) -> StdResult<Dispatch<'a>> {
        Ok(Self {
            oracle_ref,
            last_dispatch,
            config,
            block_time,
            querier,
        })
    }

    fn create_response(&self, reward: Coin<Nls>) -> Result<Batch, ContractError> {
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

        batch
            .schedule_execute_wasm_no_reply::<_, Nls>(
                &self.config.timealarms,
                &timealarms::msg::ExecuteMsg::AddAlarm {
                    time: self.block_time + Duration::from_hours(self.config.cadence_hours),
                },
                None,
            )
            .map_err(ContractError::from)?;

        Ok(batch)
    }
}
