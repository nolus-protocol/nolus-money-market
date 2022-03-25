#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Timestamp,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, QueryQuoteResponse};
use crate::state::{Config, CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config::new(&msg.denom);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // TODO: addr check?
    let loan = info.sender;
    match msg {
        ExecuteMsg::Borrow { amount } => try_borrow(loan, amount),
        ExecuteMsg::Repay { amount } => try_repay(loan, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Quote { loan, amount } => to_binary(&query_quote(loan, amount)?),
        QueryMsg::Borrow { loan } => to_binary(&query_borrow(loan)?),
        QueryMsg::BorrowOutstandingInterest {
            loan,
            outstanding_by,
        } => to_binary(&query_borrow_outstanding_interest(loan, outstanding_by)?),
    }
}

fn try_borrow(_loan: Addr, _amount: Coin) -> Result<Response, ContractError> {
    unimplemented!()
}

fn try_repay(_loan: Addr, _amount: Coin) -> Result<Response, ContractError> {
    unimplemented!()
}

fn query_quote(_loan: Addr, _amount: Coin) -> StdResult<QueryQuoteResponse> {
    unimplemented!()
}

fn query_borrow(_loan: Addr) -> StdResult<QueryQuoteResponse> {
    unimplemented!()
}

fn query_borrow_outstanding_interest(
    _loan: Addr,
    _outstanding_by: Timestamp,
) -> StdResult<QueryQuoteResponse> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::coins;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "ust"));

        let msg = InstantiateMsg {
            denom: "ust".into(),
        };
        let info = mock_info("creator", &coins(1000, "ust"));

        instantiate(deps.as_mut(), mock_env(), info, msg).expect("can't instantiate");
    }
}
