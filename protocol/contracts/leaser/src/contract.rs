use std::ops::{Deref, DerefMut};

use access_control::ContractOwnerAccess;
use lease::api::MigrateMsg as LeaseMigrateMsg;
use platform::{
    batch::Batch, contract, error as platform_error, message::Response as MessageResponse, reply,
    response,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        entry_point, to_json_binary, Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo,
        QuerierWrapper, Reply,
    },
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    cmd::Borrow,
    error::ContractError,
    leaser::{self, Leaser},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    state::{config::Config, leases::Leases},
};

const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 1;
const CONTRACT_STORAGE_VERSION: VersionSegment = 2;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    contract::validate_addr(deps.querier, &msg.lpp)?;
    contract::validate_addr(deps.querier, &msg.time_alarms)?;
    contract::validate_addr(deps.querier, &msg.market_price_oracle)?;
    contract::validate_addr(deps.querier, &msg.profit)?;

    versioning::initialize(deps.storage, CONTRACT_VERSION)?;

    ContractOwnerAccess::new(deps.storage.deref_mut()).grant_to(&info.sender)?;

    let lease_code = msg.lease_code_id;
    Config::new(msg).store(deps.storage)?;

    leaser::update_lpp(deps.storage, lease_code.into(), Batch::default())
        .map(response::response_only_messages)
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    // Statically assert that the message is empty when doing a software-only update.
    let MigrateMsg {} = msg;

    versioning::update_software_and_storage::<_, CONTRACT_STORAGE_VERSION_FROM, _, _, _, _>(
        deps.storage,
        CONTRACT_VERSION,
        |storage: &mut _| {
            use super::state::v1::Config as ConfigOld;
            ConfigOld::migrate(storage)
                .and_then(|config_new| config_new.store(storage))
                .map(|()| MessageResponse::default())
        },
        Into::into,
    )
    .and_then(|(release_label, resp)| response::response_with_messages(release_label, resp))
    .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::OpenLease { currency, max_ltd } => Borrow::with(
            deps.storage,
            info.funds,
            info.sender,
            env.contract.address.clone(),
            finalizer(env),
            currency,
            max_ltd,
        ),
        ExecuteMsg::FinalizeLease { customer } => {
            validate_customer(customer, deps.api, deps.querier)
                .and_then(|customer| {
                    validate_lease(info.sender, deps.as_ref()).map(|lease| (customer, lease))
                })
                .and_then(|(customer, lease)| {
                    Leases::remove(deps.storage, customer, &lease).map_err(Into::into)
                })
                .map(|removed| {
                    debug_assert!(removed);
                    MessageResponse::default()
                })
        }
        ExecuteMsg::MigrateLeases {
            new_code_id,
            max_leases,
        } => ContractOwnerAccess::new(deps.storage.deref())
            .check(&info.sender)
            .map_err(Into::into)
            .and_then(move |()| {
                leaser::try_migrate_leases(
                    deps.storage,
                    new_code_id.into(),
                    max_leases,
                    migrate_msg(),
                )
            }),
        ExecuteMsg::MigrateLeasesCont {
            key: next_customer,
            max_leases,
        } => ContractOwnerAccess::new(deps.storage.deref())
            .check(&info.sender)
            .map_err(Into::into)
            .and_then(|()| validate_customer(next_customer, deps.api, deps.querier))
            .and_then(move |next_customer_validated| {
                leaser::try_migrate_leases_cont(
                    deps.storage,
                    next_customer_validated,
                    max_leases,
                    migrate_msg(),
                )
            }),
    }
    .map(response::response_only_messages)
    .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
    match msg {
        SudoMsg::Config {
            lease_interest_rate_margin,
            lease_position_spec,
            lease_due_period,
        } => leaser::try_configure(
            deps.storage,
            lease_interest_rate_margin,
            lease_position_spec,
            lease_due_period,
        ),
    }
    .map(response::response_only_messages)
    .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&Leaser::new(deps).config()?),
        QueryMsg::Quote {
            downpayment,
            lease_asset,
            max_ltd,
        } => to_json_binary(&Leaser::new(deps).quote(downpayment, lease_asset, max_ltd)?),
        QueryMsg::Leases { owner } => to_json_binary(&Leaser::new(deps).customer_leases(owner)?),
    }
    .map_err(Into::into)
    .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<Response> {
    reply::from_instantiate_addr_only(deps.api, msg)
        .map_err(|err| ContractError::ParseError {
            err: err.to_string(),
        })
        .and_then(|lease| {
            Leases::save(deps.storage, lease.clone()).map(|stored| {
                debug_assert!(stored);
                lease
            })
        })
        .map(|lease| Response::new().add_attribute("lease_address", lease))
        .or_else(|err| platform_error::log(err, deps.api))
}

fn validate_customer(
    customer: Addr,
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Addr> {
    api.addr_validate(customer.as_str())
        .map_err(|_| ContractError::InvalidContinuationKey {
            err: "invalid address".into(),
        })
        .and_then(|next_customer| {
            contract::validate_addr(querier, &next_customer)
                .is_err()
                .then_some(next_customer)
                .ok_or_else(|| ContractError::InvalidContinuationKey {
                    err: "smart contract key".into(),
                })
        })
}

fn validate_lease(lease: Addr, deps: Deps<'_>) -> ContractResult<Addr> {
    Leaser::new(deps)
        .config()
        .map(|config| config.config.lease_code_id)
        .and_then(|lease_code_id| {
            contract::validate_code_id(deps.querier, &lease, lease_code_id).map_err(Into::into)
        })
        .map(|()| lease)
}

fn migrate_msg() -> impl Fn(Addr) -> LeaseMigrateMsg {
    |_customer| LeaseMigrateMsg {}
}

fn finalizer(env: Env) -> Addr {
    env.contract.address
}
