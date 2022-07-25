use std::marker::PhantomData;

use crate::state::Config;
use crate::state::DispatchLog;
use crate::ContractError;
use cosmwasm_std::StdResult;
use cosmwasm_std::{Decimal, QuerierWrapper, Response, Storage, Timestamp};
use finance::coin::Coin;
use finance::currency::{Currency, Nls};
use finance::duration::Duration;
use finance::fraction::Fraction;
use finance::interest::InterestPeriod;
use finance::ratio::Rational;
use lpp::stub::Lpp as LppTrait;
use platform::batch::Batch;

pub struct Dispatcher<'a, Lpn, Lpp> {
    lpn: PhantomData<Lpn>,
    lpp: Lpp,
    storage: &'a mut dyn Storage,
    querier: QuerierWrapper<'a>,
    config: Config,
    block_time: Timestamp,
}

impl<'a, Lpn, Lpp> Dispatcher<'a, Lpn, Lpp>
where
    Lpp: LppTrait<Lpn>,
    Lpn: Currency,
{
    pub fn new(
        lpp: Lpp,
        storage: &'a mut dyn Storage,
        querier: QuerierWrapper<'a>,
        config: Config,
        block_time: Timestamp,
    ) -> StdResult<Dispatcher<'a, Lpn, Lpp>> {
        Ok(Self {
            lpn: PhantomData,
            lpp,
            storage,
            querier,
            config,
            block_time,
        })
    }

    pub fn dispatch(&mut self) -> Result<Response, ContractError>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency,
    {
        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let resp = self.lpp.lpp_balance()?;
        let lpp_balance: Coin<Lpn> = resp.balance;

        // get annual percentage of return from configuration
        let arp_permille = self.config.tvl_to_apr.get_apr(lpp_balance.into())?;

        let last_dispatch = DispatchLog::last_dispatch(self.storage)?;
        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_in_lppdenom = InterestPeriod::with_interest(arp_permille)
            .from(last_dispatch)
            .spanning(Duration::between(last_dispatch, self.block_time))
            .interest(lpp_balance);

        if reward_in_lppdenom.is_zero() {
            return Self::no_reward_resp();
        }

        // Store the current time for use for the next calculation.
        DispatchLog::update(self.storage, self.block_time)?;

        // Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
        let reward_unls = self.swap_reward_in_unls(reward_in_lppdenom)?;

        if reward_unls.is_zero() {
            return Self::no_reward_resp();
        }

        let pay_msg = self.lpp.distribute_rewards_req(reward_unls)?;

        let mut resp = self.create_response(reward_unls)?;
        // TODO: use Platform to subscribe for lpp distribute message ?
        resp.messages.insert(resp.messages.len() - 1, pay_msg);
        Ok(resp)
    }

    fn create_response(&self, reward: Coin<Nls>) -> Result<Response, ContractError> {
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
                &self.config.timealarms,
                &timealarms::msg::ExecuteMsg::AddAlarm {
                    time: self.block_time + Duration::from_hours(self.config.cadence_hours),
                },
                None,
            )
            .map_err(ContractError::from)?;

        Ok(Response::from(batch))
    }

    fn get_market_price(&self, denom: &str) -> StdResult<Decimal> {
        use oracle::msg::{PriceResponse, QueryMsg as MarketQueryMsg};

        let query_msg: MarketQueryMsg = MarketQueryMsg::PriceFor {
            denoms: vec![denom.to_string()],
        };
        let resp: PriceResponse = self
            .querier
            .query_wasm_smart(self.config.oracle.to_string(), &query_msg)?;
        let denom_price = match resp.prices.first() {
            Some(d) => d.price.amount,
            None => todo!(),
        };

        Ok(denom_price)
    }

    fn swap_reward_in_unls(&self, reward_in_lppdenom: Coin<Lpn>) -> StdResult<Coin<Nls>>
    where
        Lpn: Currency,
    {
        // get price of the native denom in market oracle base asset
        let native_denom_price = self.get_market_price(Nls::SYMBOL)?;

        // calculate LPN price from the response
        let ratio = Rational::new(
            cosmwasm_std::Fraction::denominator(&native_denom_price).u128(),
            cosmwasm_std::Fraction::numerator(&native_denom_price).u128(),
        );

        let lpp_amount: u128 = reward_in_lppdenom.into();
        let nls_amount = <Rational<u128> as Fraction<u128>>::of(&ratio, lpp_amount);

        Ok(Coin::<Nls>::new(nls_amount))
    }

    fn no_reward_resp() -> Result<Response, ContractError> {
        Ok(Response::new()
            .add_attribute("method", "try_dispatch")
            .add_attribute("result", "no reward to dispatch"))
    }
}
