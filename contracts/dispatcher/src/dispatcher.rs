use cosmwasm_std::{to_binary, Addr, Coin, Decimal};
use cosmwasm_std::{Deps, QueryRequest, StdResult, WasmQuery};
use finance::coin::add_coin;
use lpp::msg::{ExecuteMsg as LPPExecuteMsg, LppBalanceResponse, QueryMsg as LPPQueryMsg};
use oracle::msg::{PriceResponse, QueryMsg as MarketQueryMsg};

// Get LPP balance and return TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
pub(crate) fn get_lpp_balance(deps: Deps, lpp_addr: Addr) -> StdResult<Coin> {
    let query_msg: LPPQueryMsg = LPPQueryMsg::LppBalance {};
    let resp: LppBalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: lpp_addr.to_string(),
        msg: to_binary(&query_msg)?,
    }))?;

    let balance = add_coin(
        add_coin(resp.balance, resp.total_principal_due),
        resp.total_interest_due,
    );

    Ok(balance)
}

pub(crate) fn exec_lpp_distribute_rewards() -> LPPExecuteMsg {
    LPPExecuteMsg::DistributeRewards {}
}

pub(crate) fn get_market_price(deps: Deps, market_oracle: Addr, denom: &str) -> StdResult<Decimal> {
    // TODO:  market oracle will return the price in the configured base asset. This will not be the native currency.
    // TBD: we need to extend market price oracle
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

pub(crate) fn swap_reward_in_unls(
    deps: Deps,
    market_oracle: Addr,
    reward_lppdenom: Coin,
) -> StdResult<Coin> {
    let lppdenom_price = get_market_price(deps, market_oracle, &reward_lppdenom.denom)?;

    let reward_unls = reward_lppdenom.amount * lppdenom_price.atomics();
    Ok(Coin::new(reward_unls.u128(), reward_lppdenom.denom))
}
