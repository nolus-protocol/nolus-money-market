#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::leaser::Leaser;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::config::Config;
use crate::state::leaser::Loans;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config::new(info.sender, msg)?;
    config.store(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::OpenLease { currency } => {
            Leaser::try_borrow(deps, info.funds, info.sender, currency)
        }
        ExecuteMsg::Config {
            lease_interest_rate_margin,
            liability,
            repayment,
        } => Leaser::try_configure(deps, info, lease_interest_rate_margin, liability, repayment),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&Leaser::query_config(deps)?),
        QueryMsg::Quote { downpayment } => to_binary(&Leaser::query_quote(env, deps, downpayment)?),
        QueryMsg::Leases { owner } => to_binary(&Leaser::query_loans(deps, owner)?),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let contract_addr_raw = parse_reply_instantiate_data(msg.clone())
        .map(|r| r.contract_address)
        .map_err(|err| ContractError::ParseError {
            err: err.to_string(),
        })?;

    let contract_addr = deps.api.addr_validate(&contract_addr_raw)?;

    match Loans::save(deps.storage, msg.id, contract_addr.clone()) {
        Ok(_) => Ok(Response::new().add_attribute("lease_address", contract_addr)),
        Err(err) => {
            Loans::remove(deps.storage, msg.id);
            Err(ContractError::CustomError {
                val: err.to_string(),
            })
        }
    }
}
