use platform::reply::from_instantiate;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        to_binary, Api, Binary, Deps, DepsMut, Env, MessageInfo, Reply, StdError, Storage,
    },
    cw2::set_contract_version,
};

use crate::{
    cmd::Borrow,
    error::ContractError,
    leaser::Leaser,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{config::Config, leaser::Loans},
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    deps.api
        .addr_validate(msg.lpp_ust_addr.as_str())
        .map_err(|_| {
            StdError::generic_err(format!(
                "Invalid LPP address provided! Input: {:?}",
                msg.lpp_ust_addr.as_str(),
            ))
        })?;

    deps.api
        .addr_validate(msg.market_price_oracle.as_str())
        .map_err(|_| {
            StdError::generic_err(format!(
                "Invalid Market Price Oracle address provided! Input: {:?}",
                msg.market_price_oracle.as_str(),
            ))
        })?;

    let config = Config::new(info.sender, msg)?;
    config.store(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::OpenLease { currency } => Borrow::with(deps, info.funds, info.sender, currency),
        ExecuteMsg::Config {
            lease_interest_rate_margin,
            liability,
            repayment,
        } => Leaser::try_configure(deps, info, lease_interest_rate_margin, liability, repayment),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&Leaser::query_config(deps)?),
        QueryMsg::Quote { downpayment } => to_binary(&Leaser::query_quote(deps, downpayment)?),
        QueryMsg::Leases { owner } => to_binary(&Leaser::query_loans(deps, owner)?),
    };
    res.map_err(ContractError::from)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match on_reply(deps.api, deps.storage, msg.clone()) {
        Ok(resp) => Ok(resp),
        Err(err) => {
            Loans::remove(deps.storage, msg.id);
            Err(ContractError::CustomError {
                val: err.to_string(),
            })
        }
    }
}

fn on_reply(
    api: &dyn Api,
    storage: &mut dyn Storage,
    msg: Reply,
) -> Result<Response, ContractError> {
    let contract_addr = from_instantiate::<()>(api, msg.clone())
        .map(|r| r.address)
        .map_err(|err| ContractError::ParseError {
            err: err.to_string(),
        })?;

    Loans::save(storage, msg.id, contract_addr.clone())?;
    Ok(Response::new().add_attribute("lease_address", contract_addr))
}
