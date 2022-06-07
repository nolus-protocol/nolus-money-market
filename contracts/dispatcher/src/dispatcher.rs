use cosmwasm_std::{
    ensure, to_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, Fraction, MessageInfo,
    Response, SubMsg, Timestamp, WasmMsg,
};
use cosmwasm_std::{Deps, StdResult};
use finance::duration::Duration;
use finance::interest::InterestPeriod;
use lpp::msg::ExecuteMsg as LPPExecuteMsg;

use crate::state::config::Config;
use crate::state::dispatch_log::DispatchLog;
use crate::ContractError;

const NATIVE_DENOM: &str = "unolus";

pub struct Dispatcher {}

impl Dispatcher {
    pub fn try_dispatch(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        time: Timestamp,
    ) -> Result<Response, ContractError> {
        ensure!(
            time >= env.block.time,
            ContractError::AlarmTimeValidation {}
        );
        let config = Config::load(deps.storage)?;

        if info.sender != config.time_oracle {
            return Err(ContractError::UnrecognisedAlarm(info.sender));
        }

        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let lpp_balance = Self::get_lpp_balance(deps.as_ref(), &config.lpp)?;

        // get annual percentage of return from configuration
        let arp_permille = config.tvl_to_apr.get_apr(lpp_balance.amount.u128())?;

        let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;
        // Calculate the reward in LPN,
        // which matches TVLdenom, since the last calculation
        let reward_lppdenom = InterestPeriod::with_interest(arp_permille)
            .from(last_dispatch)
            .spanning(Duration::between(last_dispatch, env.block.time))
            .interest(lpp_balance);

        if reward_lppdenom.amount.is_zero() {
            return Ok(Response::new()
                .add_attribute("method", "try_dispatch")
                .add_attribute("result", "no reward to dispatch"));
        }

        // Store the current time for use for the next calculation.
        DispatchLog::update(deps.storage, env.block.time)?;

        // Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
        let reward_unls =
            Self::swap_reward_in_unls(deps.as_ref(), config.market_oracle, reward_lppdenom)?;

        if reward_unls.amount.is_zero() {
            return Ok(Response::new()
                .add_attribute("method", "try_dispatch")
                .add_attribute("result", "no reward to dispatch"));
        }

        // Prepare a Send Rewards for the amount of Rewards_uNLS to the Treasury.
        let treasury_send_rewards_msg =
            Self::treasury_send_rewards(&config.treasury, &config.lpp, reward_unls)?;

        // Prepare LPP.Distribute Rewards command
        let lpp_distribute_rewards_msg = Self::lpp_distribute_rewards(config.lpp)?;
        Ok(Response::new()
            .add_submessages(vec![treasury_send_rewards_msg, lpp_distribute_rewards_msg]))
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
        Ok(Coin::new(2000000000, "unolus"))
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
        // get price of the unolus in UST(market oracle base asset)
        let native_denom_price = Self::get_market_price(deps, market_oracle, NATIVE_DENOM)?;

        // calculate the UST price from the response
        let reward_unls = reward_lppdenom.amount.multiply_ratio(
            native_denom_price.denominator(),
            native_denom_price.numerator(),
        );
        Ok(Coin::new(reward_unls.u128(), reward_lppdenom.denom))
    }

    fn treasury_send_rewards(treasury: &Addr, lpp: &Addr, reward: Coin) -> StdResult<SubMsg> {
        Ok(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![],
            contract_addr: treasury.to_string(),
            msg: to_binary(&treasury::msg::ExecuteMsg::SendRewards {
                lpp_addr: lpp.to_owned(),
                amount: reward,
            })?,
        })))
    }

    fn lpp_distribute_rewards(lpp: Addr) -> StdResult<SubMsg> {
        Ok(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![],
            contract_addr: lpp.to_string(),
            msg: to_binary(&LPPExecuteMsg::DistributeRewards {})?,
        })))
    }
}
