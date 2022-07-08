use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, DepsMut, Response, Timestamp, WasmMsg};
use cosmwasm_std::{Deps, StdResult};
use finance::coin::Coin;
use finance::coin_legacy::to_cosmwasm;
use finance::currency::{Currency, Nls, Usdc};
use finance::duration::Duration;
use finance::fraction::Fraction;
use finance::interest::InterestPeriod;
use finance::ratio::Rational;

use crate::state::config::Config;
use crate::state::dispatch_log::DispatchLog;
use crate::ContractError;

pub struct Dispatcher {}

impl Dispatcher {
    pub fn dispatch(
        deps: DepsMut,
        config: &Config,
        block_time: Timestamp,
    ) -> Result<Response, ContractError> {
        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let lpp_balance = Self::get_lpp_balance(deps.as_ref(), &config.lpp)?;

        // get annual percentage of return from configuration
        let arp_permille = config.tvl_to_apr.get_apr(lpp_balance.into())?;

        let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;
        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_in_lppdenom = InterestPeriod::with_interest(arp_permille)
            .from(last_dispatch)
            .spanning(Duration::between(last_dispatch, block_time))
            .interest(lpp_balance);

        if reward_in_lppdenom.is_zero() {
            return Self::no_reward_resp();
        }

        // Store the current time for use for the next calculation.
        DispatchLog::update(deps.storage, block_time)?;

        // Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
        let reward_unls =
            Self::swap_reward_in_unls(deps.as_ref(), config.oracle.to_owned(), reward_in_lppdenom)?;

        if reward_unls.is_zero() {
            return Self::no_reward_resp();
        }

        // Prepare a Send Rewards for the amount of Rewards_uNLS to the Treasury.
        let treasury_send_rewards_msg = Self::treasury_send_rewards(&config.treasury, reward_unls)?;

        let pay_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![to_cosmwasm(reward_unls)],
            contract_addr: config.lpp.to_string(),
            msg: to_binary(&lpp::msg::ExecuteMsg::DistributeRewards {})?,
        });

        let subsrcibe_msg = Dispatcher::alarm_subscribe_msg(
            &config.timealarms,
            block_time,
            Duration::from_hours(config.cadence_hours),
        )?;

        Ok(Response::new().add_messages([treasury_send_rewards_msg, pay_msg, subsrcibe_msg]))
    }

    //TODO: make it for other currencies also
    #[cfg(not(test))]
    // Get LPP balance and return TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
    fn get_lpp_balance(deps: Deps, lpp_addr: &Addr) -> Result<Coin<Usdc>, ContractError> {
        use finance::coin_legacy::from_cosmwasm;
        use lpp::msg::{LppBalanceResponse, QueryMsg as LPPQueryMsg};

        let query_msg: LPPQueryMsg = LPPQueryMsg::LppBalance {};
        let resp: LppBalanceResponse = deps
            .querier
            .query_wasm_smart(lpp_addr.to_string(), &query_msg)?;

        let balance = from_cosmwasm(resp.balance)?
            + from_cosmwasm(resp.total_principal_due)?
            + from_cosmwasm(resp.total_interest_due)?;

        Ok(balance)
    }

    #[cfg(test)]
    // Get LPP balance and return TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
    fn get_lpp_balance(_deps: Deps, _lpp_addr: &Addr) -> StdResult<Coin<Usdc>> {
        Ok(Coin::<Usdc>::new(2000000000))
    }

    #[cfg(not(test))]
    fn get_market_price(deps: Deps, market_oracle: Addr, denom: &str) -> StdResult<Decimal> {
        use oracle::msg::{PriceResponse, QueryMsg as MarketQueryMsg};

        let query_msg: MarketQueryMsg = MarketQueryMsg::PriceFor {
            denoms: vec![denom.to_string()],
        };
        let resp: PriceResponse = deps
            .querier
            .query_wasm_smart(market_oracle.to_string(), &query_msg)?;
        let denom_price = match resp.prices.first() {
            Some(d) => d.price.amount,
            None => todo!(),
        };

        Ok(denom_price)
    }

    #[cfg(test)]
    fn get_market_price(_deps: Deps, _market_oracle: Addr, _denom: &str) -> StdResult<Decimal> {
        use std::str::FromStr;

        Decimal::from_str("0.12345")
    }

    fn swap_reward_in_unls<C>(
        deps: Deps,
        market_oracle: Addr,
        reward_in_lppdenom: Coin<C>,
    ) -> StdResult<Coin<Nls>>
    where
        C: Currency,
    {
        // get price of the native denom in UST(market oracle base asset)
        let native_denom_price = Self::get_market_price(deps, market_oracle, Nls::SYMBOL)?;

        // calculate the UST price from the response
        let ratio = Rational::new(
            cosmwasm_std::Fraction::denominator(&native_denom_price).u128(),
            cosmwasm_std::Fraction::numerator(&native_denom_price).u128(),
        );

        let lpp_amount: u128 = reward_in_lppdenom.into();
        let nls_amount = <Rational<u128> as Fraction<u128>>::of(&ratio, lpp_amount);

        Ok(Coin::<Nls>::new(nls_amount))
    }

    fn treasury_send_rewards(treasury: &Addr, reward: Coin<Nls>) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![],
            contract_addr: treasury.to_string(),
            msg: to_binary(&treasury::msg::ExecuteMsg::SendRewards { amount: reward })?,
        }))
    }

    fn no_reward_resp() -> Result<Response, ContractError> {
        Ok(Response::new()
            .add_attribute("method", "try_dispatch")
            .add_attribute("result", "no reward to dispatch"))
    }

    pub(crate) fn alarm_subscribe_msg(
        timealarm_addr: &Addr,
        current_time: Timestamp,
        cadence_hours: Duration,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![],
            contract_addr: timealarm_addr.to_string(),
            msg: to_binary(&timealarms::msg::ExecuteMsg::AddAlarm {
                time: current_time + cadence_hours,
            })?,
        }))
    }
}
