#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    entry_point, to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response,
    Storage, Timestamp,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryLoanOutstandingInterestResponse, QueryLoanResponse, QueryMsg,
    QueryQuoteResponse,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let lease_unchecked = info.sender;
    let funds = info.funds;

    match msg {
        ExecuteMsg::OpenLoan { amount } => try_open_loan(deps, env, lease_unchecked, amount),
        ExecuteMsg::RepayLoan => try_repay_loan(deps, env, lease_unchecked, funds),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Quote { amount } => to_binary(&query_quote(&deps, &env, amount)?),
        QueryMsg::Loan { lease_addr } => to_binary(&query_loan(deps.storage, lease_addr)?),
        QueryMsg::LoanOutstandingInterest {
            lease_addr,
            outstanding_time,
        } => to_binary(&query_loan_outstanding_interest(
            deps.storage,
            lease_addr,
            outstanding_time,
        )?),
    }?;

    Ok(res)
}

fn try_open_loan(
    _deps: DepsMut,
    _env: Env,
    lease_addr: Addr,
    amount: Coin,
) -> Result<Response, ContractError> {
    let transfer_msg = BankMsg::Send {
        to_address: lease_addr.to_string(),
        amount: vec![amount],
    };

    let response = Response::new()
        .add_attribute("method", "try_open_loan")
        .add_message(transfer_msg);

    Ok(response)
}

fn try_repay_loan(
    _deps: DepsMut,
    _env: Env,
    _lease_addr: Addr,
    _funds: Vec<Coin>,
) -> Result<Response, ContractError> {
    let response = Response::new().add_attribute("method", "try_repay_loan");

    Ok(response)
}

fn query_quote(
    _deps: &Deps,
    _env: &Env,
    _quote: Coin,
) -> Result<QueryQuoteResponse, ContractError> {
    Ok(QueryQuoteResponse::NoLiquidity)
}

fn query_loan(
    _storage: &dyn Storage,
    _lease_addr: Addr,
) -> Result<QueryLoanResponse, ContractError> {
    Ok(None)
}

fn query_loan_outstanding_interest(
    _storage: &dyn Storage,
    _loan: Addr,
    _outstanding_time: Timestamp,
) -> Result<QueryLoanOutstandingInterestResponse, ContractError> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, Uint64};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "ust"));

        let msg = InstantiateMsg {
            denom: "ust".into(),
            lease_code_id: Uint64::new(1000),
        };
        let info = mock_info("creator", &coins(1000, "ust"));

        instantiate(deps.as_mut(), mock_env(), info, msg).expect("can't instantiate");
    }
}
