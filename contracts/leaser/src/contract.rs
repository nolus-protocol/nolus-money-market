#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use cw2::set_contract_version;
use cw_utils::parse_reply_instantiate_data;
use finance::percent::Percent;

use crate::error::ContractError;
use crate::leaser::Leaser;
use crate::msg::{ExecuteMsg, InstantiateMsg, Liability, QueryMsg, Repayment};
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
        } => try_configure(deps, info, lease_interest_rate_margin, liability, repayment),
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
        .map_err(|_| ContractError::ParseError {})?;

    let contract_addr = deps.api.addr_validate(&contract_addr_raw)?;
    register_lease(deps, msg.id, contract_addr)
}

pub fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    lease_interest_rate_margin: Percent,
    liability: crate::msg::Liability,
    repayment: Repayment,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    Liability::validate(liability.initial, liability.healthy, liability.max);
    Config::update(
        deps.storage,
        lease_interest_rate_margin,
        liability,
        repayment,
    )?;

    Ok(Response::default())
}

fn register_lease(deps: DepsMut, msg_id: u64, lease_addr: Addr) -> Result<Response, ContractError> {
    // TODO: Remove pending id if the creation was not successful
    Loans::save(deps.storage, msg_id, lease_addr.clone())?;
    Ok(Response::new().add_attribute("lease_address", lease_addr))
}
