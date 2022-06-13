use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Fraction, Response, Timestamp, WasmMsg,
};
use cosmwasm_std::{Deps, StdResult};
use finance::duration::Duration;
use finance::interest::InterestPeriod;

use crate::state::config::Config;
use crate::state::dispatch_log::DispatchLog;
use crate::ContractError;

const NATIVE_DENOM: &str = "uNLS";

pub struct Dispatcher {}

impl Dispatcher {
    pub fn dispatch(
        deps: DepsMut,
        config: Config,
        block_time: Timestamp,
    ) -> Result<Response, ContractError> {
        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let lpp_balance = Self::get_lpp_balance(deps.as_ref(), &config.lpp)?;

        // get annual percentage of return from configuration
        let arp_permille = config.tvl_to_apr.get_apr(lpp_balance.amount.u128())?;

        let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;
        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_lppdenom = InterestPeriod::with_interest(arp_permille)
            .from(last_dispatch)
            .spanning(Duration::between(last_dispatch, block_time))
            .interest(lpp_balance);

        if reward_lppdenom.amount.is_zero() {
            return Self::no_reward_resp();
        }

        // Store the current time for use for the next calculation.
        DispatchLog::update(deps.storage, block_time)?;

        // Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
        let reward_unls =
            Self::swap_reward_in_unls(deps.as_ref(), config.market_oracle, reward_lppdenom)?;

        if reward_unls.amount.is_zero() {
            return Self::no_reward_resp();
        }

        // Prepare a Send Rewards for the amount of Rewards_uNLS to the Treasury.
        let treasury_send_rewards_msg =
            Self::treasury_send_rewards(&config.treasury, reward_unls.clone())?;

        let pay_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![reward_unls],
            contract_addr: config.lpp.to_string(),
            msg: to_binary(&lpp::msg::ExecuteMsg::DistributeRewards {})?,
        });

        Ok(Response::new().add_messages([treasury_send_rewards_msg, pay_msg]))
    }

    #[cfg(not(test))]
    // Get LPP balance and return TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
    fn get_lpp_balance(deps: Deps, lpp_addr: &Addr) -> StdResult<Coin> {
        use finance::coin::add_coin;
        use lpp::msg::{LppBalanceResponse, QueryMsg as LPPQueryMsg};

        let query_msg: LPPQueryMsg = LPPQueryMsg::LppBalance {};
        let resp: LppBalanceResponse = deps
            .querier
            .query_wasm_smart(lpp_addr.to_string(), &query_msg)?;

        let balance = add_coin(
            add_coin(resp.balance, resp.total_principal_due),
            resp.total_interest_due,
        );

        Ok(balance)
    }

    #[cfg(test)]
    // Get LPP balance and return TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
    fn get_lpp_balance(_deps: Deps, _lpp_addr: &Addr) -> StdResult<Coin> {
        Ok(Coin::new(2000000000, NATIVE_DENOM))
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

    fn swap_reward_in_unls(
        deps: Deps,
        market_oracle: Addr,
        reward_lppdenom: Coin,
    ) -> StdResult<Coin> {
        // get price of the native denom in UST(market oracle base asset)
        let native_denom_price = Self::get_market_price(deps, market_oracle, NATIVE_DENOM)?;

        // calculate the UST price from the response
        let reward_unls = reward_lppdenom.amount.multiply_ratio(
            native_denom_price.denominator(),
            native_denom_price.numerator(),
        );
        Ok(Coin::new(reward_unls.u128(), NATIVE_DENOM))
    }

    fn treasury_send_rewards(treasury: &Addr, reward: Coin) -> StdResult<CosmosMsg> {
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
}
