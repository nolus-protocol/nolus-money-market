use cosmwasm_std::{
    coins, to_binary, Addr, Binary, Coin, Deps, DepsMut, Empty, Env, MessageInfo, Response, Uint64,
};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};
use lpp::error::ContractError;

use super::ADMIN;

fn mock_lpp_balance() -> Result<lpp::msg::LppBalanceResponse, ContractError> {
    Ok(lpp::msg::LppBalanceResponse {
        balance: Coin::new(100, "UST"),
        total_principal_due: Coin::new(100, "UST"),
        total_interest_due: Coin::new(100, "UST"),
    })
}

fn mock_distribute_rewards() -> Result<Response, ContractError> {
    Ok(Response::default())
}

// TODO: remove when lpp implements LppBalance
fn mock_lpp_query(deps: Deps, env: Env, msg: lpp::msg::QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        lpp::msg::QueryMsg::LppBalance => to_binary(&mock_lpp_balance()?),
        _ => Ok(lpp::contract::query(deps, env, msg)?),
    }?;

    Ok(res)
}

// TODO: remove when lpp implements LppBalance
fn mock_lpp_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: lpp::msg::ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        lpp::msg::ExecuteMsg::DistributeRewards => mock_distribute_rewards(),
        _ => Ok(lpp::contract::execute(deps, env, info, msg)?),
    }
}

pub fn contract_lpp_mock() -> Box<dyn Contract<Empty>> {
    let contract =
        ContractWrapper::new(mock_lpp_execute, lpp::contract::instantiate, mock_lpp_query);
    Box::new(contract)
}

#[track_caller]
pub fn instantiate_lpp(app: &mut App, lease_code_id: Uint64, denom: &str) -> (Addr, u64) {
    let lpp_id = app.store_code(contract_lpp_mock());
    let msg = lpp::msg::InstantiateMsg {
        denom: denom.to_string(),
        lease_code_id,
    };
    (
        app.instantiate_contract(
            lpp_id,
            Addr::unchecked(ADMIN),
            &msg,
            &coins(400, denom),
            "lpp",
            None,
        )
        .unwrap(),
        lpp_id,
    )
}
