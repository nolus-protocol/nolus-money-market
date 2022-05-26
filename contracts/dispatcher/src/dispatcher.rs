use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, DepsMut, Env, Fraction, MessageInfo, Response,
    SubMsg, Timestamp, WasmMsg,
};
use cosmwasm_std::{Deps, QueryRequest, StdResult, WasmQuery};
use finance::coin::add_coin;
use finance::duration::Duration;
use finance::interest::InterestPeriod;
use lpp::msg::{ExecuteMsg as LPPExecuteMsg, LppBalanceResponse, QueryMsg as LPPQueryMsg};
use oracle::msg::{PriceResponse, QueryMsg as MarketQueryMsg};

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
        _time: Timestamp,
    ) -> Result<Response, ContractError> {
        let config = Config::load(deps.storage)?;

        if info.sender != config.time_oracle {
            return Err(ContractError::UnrecognisedAlarm(info.sender));
        }

        // get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
        let lpp_balance = Self::get_lpp_balance(deps.as_ref(), config.lpp.clone())?;

        // get apr from configuration
        let arp_permille = config.tvl_to_apr.get_apr(lpp_balance.amount.u128())?;

        let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;
        // Use the finance::interest::interestPeriod::interest() to calculate the reward in LPN,
        //    which matches TVLdenom, since the last calculation, Rewards_TVLdenom
        let reward_lppdenom = InterestPeriod::with_interest(arp_permille)
            .from(last_dispatch)
            .spanning(Duration::between(last_dispatch, env.block.time))
            .interest(&lpp_balance);
        // Store the current time for use for the next calculation.
        DispatchLog::update(deps.storage, env.block.time)?;

        // Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
        let reward_unls =
            Self::swap_reward_in_unls(deps.as_ref(), config.market_oracle, reward_lppdenom)?;

        // Prepare a Send Rewards for the amount of Rewards_uNLS to the Treasury.
        let treasury_send_rewards_msg =
            Self::treasury_send_rewards(config.treasury, config.lpp.clone(), reward_unls)?;
        // LPP.Distribute Rewards command.
        let lpp_distribute_rewards_msg = Self::lpp_distribute_rewards(config.lpp)?;
        Ok(Response::new()
            .add_submessages(vec![treasury_send_rewards_msg, lpp_distribute_rewards_msg]))
    }

    // Get LPP balance and return TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
    fn get_lpp_balance(deps: Deps, lpp_addr: Addr) -> StdResult<Coin> {
        let query_msg: LPPQueryMsg = LPPQueryMsg::LppBalance {};
        let resp: LppBalanceResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: lpp_addr.to_string(),
                msg: to_binary(&query_msg)?,
            }))?;

        let balance = add_coin(
            add_coin(resp.balance, resp.total_principal_due),
            resp.total_interest_due,
        );

        Ok(balance)
    }

    fn get_market_price(deps: Deps, market_oracle: Addr, denom: &str) -> StdResult<Decimal> {
        let query_msg: MarketQueryMsg = MarketQueryMsg::PriceFor {
            denoms: vec![denom.to_string()],
        };
        let resp: PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: market_oracle.to_string(),
            msg: to_binary(&query_msg)?,
        }))?;
        let denom_price = match resp.prices.first() {
            Some(d) => d.price.amount,
            None => todo!(),
        };

        Ok(denom_price)
    }

    fn swap_reward_in_unls(
        deps: Deps,
        market_oracle: Addr,
        reward_lppdenom: Coin,
    ) -> StdResult<Coin> {
        //get price of the unolus in UST
        let native_denom_price = Self::get_market_price(deps, market_oracle, NATIVE_DENOM)?;

        let reward_unls = reward_lppdenom.amount.multiply_ratio(
            native_denom_price.denominator(),
            native_denom_price.numerator(),
        );
        Ok(Coin::new(reward_unls.u128(), reward_lppdenom.denom))
    }

    fn treasury_send_rewards(treasury: Addr, lpp: Addr, reward: Coin) -> StdResult<SubMsg> {
        Ok(SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![],
            contract_addr: treasury.to_string(),
            msg: to_binary(&treasury::msg::ExecuteMsg::SendRewards {
                lpp_addr: lpp,
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

#[cfg(test)]
mod tests {
    // use super::Dispatcher;

    // #[test]
    // fn test_private_function() {
    //     Dispatcher::swap_reward_in_unls
    // }
}
