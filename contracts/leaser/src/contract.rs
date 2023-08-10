use std::ops::{Deref, DerefMut};

use access_control::ContractOwnerAccess;
use cosmwasm_std::{Addr, Api, QuerierWrapper};
use platform::{batch::Batch, reply::from_instantiate, response};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
};
use versioning::{version, VersionSegment};

use crate::{
    cmd::Borrow,
    error::ContractError,
    leaser::{self, Leaser},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    state::{config::Config, leases::Leases},
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    platform::contract::validate_addr(&deps.querier, &msg.lpp_ust_addr)?;
    platform::contract::validate_addr(&deps.querier, &msg.time_alarms)?;
    platform::contract::validate_addr(&deps.querier, &msg.market_price_oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.profit)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    ContractOwnerAccess::new(deps.storage.deref_mut()).grant_to(&info.sender)?;

    let lease_code = msg.lease_code_id;
    Config::new(msg)?.store(deps.storage)?;

    leaser::update_lpp(deps.storage, lease_code.u64(), Batch::default())
        .map(response::response_only_messages)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<Response> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION), Into::into)
        .and_then(response::response)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::OpenLease { currency, max_ltd } => Borrow::with(
            deps,
            info.funds,
            info.sender,
            env.contract.address,
            currency,
            max_ltd,
        )
        .map(response::response_only_messages),
        ExecuteMsg::MigrateLeases {
            new_code_id,
            max_leases,
        } => ContractOwnerAccess::new(deps.storage.deref())
            .check(&info.sender)
            .map_err(Into::into)
            .and_then(move |()| {
                leaser::try_migrate_leases(deps.storage, new_code_id.u64(), max_leases)
            })
            .map(response::response_only_messages),
        ExecuteMsg::MigrateLeasesCont {
            key: next_customer,
            max_leases,
        } => ContractOwnerAccess::new(deps.storage.deref())
            .check(&info.sender)
            .map_err(Into::into)
            .and_then(|()| validate(next_customer, deps.api, &deps.querier))
            .and_then(move |next_customer_validated| {
                leaser::try_migrate_leases_cont(deps.storage, next_customer_validated, max_leases)
            })
            .map(response::response_only_messages),
        ExecuteMsg::DetachClosed {
            max_leases,
            next_key,
        } => Leases::purge_closed(deps.storage, &deps.querier, max_leases, next_key)
            .and_then(|next_key: Option<Addr>| response::response(&next_key)),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
    match msg {
        SudoMsg::SetupDex(params) => leaser::try_setup_dex(deps.storage, params),
        SudoMsg::Config {
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        } => leaser::try_configure(
            deps.storage,
            lease_interest_rate_margin,
            liability,
            lease_interest_payment,
        ),
    }
    .map(response::response_only_messages)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&Leaser::new(deps).config()?),
        QueryMsg::Quote {
            downpayment,
            lease_asset,
            max_ltd,
        } => to_binary(&Leaser::new(deps).quote(downpayment, lease_asset, max_ltd)?),
        QueryMsg::Leases { owner } => to_binary(&Leaser::new(deps).customer_leases(owner)?),
    }
    .map_err(Into::into)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<Response> {
    let msg_id = msg.id;
    let contract_addr = from_instantiate::<()>(deps.api, msg)
        .map(|r| r.address)
        .map_err(|err| ContractError::ParseError {
            err: err.to_string(),
        })?;

    Leases::save(deps.storage, msg_id, contract_addr.clone())?;
    Ok(Response::new().add_attribute("lease_address", contract_addr))
}

fn validate(
    next_customer: Addr,
    api: &dyn Api,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Addr> {
    api.addr_validate(next_customer.as_str())
        .map_err(|_| ContractError::InvalidContinuationKey {
            err: "invalid address".into(),
        })
        .and_then(|next_customer| {
            platform::contract::validate_addr(querier, &next_customer)
                .is_err()
                .then_some(next_customer)
                .ok_or_else(|| ContractError::InvalidContinuationKey {
                    err: "smart contract key".into(),
                })
        })
}
