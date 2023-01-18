use access_control::SingleUserAccess;
use platform::reply::from_instantiate;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
};
use versioning::Version;

use crate::{
    cmd::Borrow,
    error::ContractError,
    leaser::Leaser,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{config::Config, leases::Leases},
};

const CONTRACT_VERSION: Version = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    platform::contract::validate_addr(&deps.querier, &msg.lpp_ust_addr)?;
    platform::contract::validate_addr(&deps.querier, &msg.time_alarms)?;
    platform::contract::validate_addr(&deps.querier, &msg.market_price_oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.profit)?;

    versioning::initialize::<CONTRACT_VERSION>(deps.storage)?;

    SingleUserAccess::new_contract_owner(info.sender).store(deps.storage)?;

    Config::new(msg)?.store(deps.storage)?;

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
        ExecuteMsg::SetupDex(params) => Leaser::try_setup_dex(deps.storage, info, params),
        ExecuteMsg::Config {
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        } => Leaser::try_configure(
            deps.storage,
            info,
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        ),
        ExecuteMsg::MigrateLeases { new_code_id } => {
            Leaser::try_migrate_leases(deps.storage, info, new_code_id)
        }
        ExecuteMsg::OpenLease { currency } => Borrow::with(deps, info.funds, info.sender, currency),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&Leaser::query_config(deps)?),
        QueryMsg::Quote {
            downpayment,
            lease_asset,
        } => to_binary(&Leaser::query_quote(deps, downpayment, lease_asset)?),
        QueryMsg::Leases { owner } => to_binary(&Leaser::query_loans(deps, owner)?),
    };
    res.map_err(ContractError::from)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let msg_id = msg.id;
    let contract_addr = from_instantiate::<()>(deps.api, msg)
        .map(|r| r.address)
        .map_err(|err| ContractError::ParseError {
            err: err.to_string(),
        })?;

    Leases::save(deps.storage, msg_id, contract_addr.clone())?;
    Ok(Response::new().add_attribute("lease_address", contract_addr))
}
