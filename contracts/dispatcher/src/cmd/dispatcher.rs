use std::marker::PhantomData;

use crate::cmd::Result as DispatcherResult;
use crate::state::Config;
use crate::ContractError;
use cosmwasm_std::StdResult;
use cosmwasm_std::Timestamp;
use finance::coin::Coin;
use finance::currency::{Currency, Nls};
use finance::duration::Duration;
use finance::interest::InterestPeriod;
use finance::price::{total, Price};
use lpp::stub::Lpp as LppTrait;
use platform::batch::Batch;

pub struct Dispatcher<Lpn, Lpp> {
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    last_dispatch: Timestamp,
    config: Config,
    block_time: Timestamp,
}

impl<'a, Lpn, Lpp> Dispatcher<Lpn, Lpp>
where
    Lpp: LppTrait<Lpn>,
    Lpn: Currency,
{
    pub fn new(
        lpp: Lpp,
        last_dispatch: Timestamp,
        config: Config,
        block_time: Timestamp,
    ) -> StdResult<Dispatcher<Lpn, Lpp>> {
        Ok(Self {
            lpn: PhantomData,
            lpp,
            last_dispatch,
            config,
            block_time,
        })
    }

    pub fn dispatch(
        &mut self,
        native_price: Price<Nls, Lpn>,
    ) -> Result<DispatcherResult<Lpn>, ContractError>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let resp = self.lpp.lpp_balance()?;
        let lpp_balance: Coin<Lpn> = resp.balance;

        // get annual percentage of return from configuration
        let arp_permille = self.config.tvl_to_apr.get_apr(lpp_balance.into())?;

        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_in_lppdenom = InterestPeriod::with_interest(arp_permille)
            .from(self.last_dispatch)
            .spanning(Duration::between(self.last_dispatch, self.block_time))
            .interest(lpp_balance);

        if reward_in_lppdenom.is_zero() {
            return Err(ContractError::ZeroReward {});
        }

        // Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
        let reward_unls = total(reward_in_lppdenom, native_price.inv());

        if reward_unls.is_zero() {
            return Err(ContractError::ZeroReward {});
        }

        Ok(DispatcherResult {
            batch: self.create_response(reward_unls)?,
            receipt: super::Receipt {
                in_stable: reward_in_lppdenom,
                in_nls: reward_unls,
            },
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
            .schedule_execute_wasm_no_reply::<_, Nls>(
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
