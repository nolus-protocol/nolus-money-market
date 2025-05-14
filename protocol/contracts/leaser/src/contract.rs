use std::ops::{Deref, DerefMut};

use serde::Serialize;

use access_control::ContractOwnerAccess;
use lease::api::{MigrateMsg as LeaseMigrateMsg, authz::AccessGranted};
use platform::{
    contract::{self, Code, CodeId},
    error as platform_error,
    message::Response as MessageResponse,
    reply, response,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Storage,
        entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, UpdatablePackage,
    VersionSegment, package_name, package_version,
};

use crate::{
    authz::AnomalyResolutionPermission,
    cmd::Borrow,
    error::ContractError,
    lease::CacheFirstRelease,
    leaser::{self, Leaser},
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MaxLeases, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    state::{config::Config, leases::Leases},
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 4;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

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
    // cannot validate the address since the Admin plays the role of the registry
    // and it is not yet instantiated
    deps.api.addr_validate(msg.protocols_registry.as_str())?;

    ContractOwnerAccess::new(deps.storage.deref_mut()).grant_to(&info.sender)?;

    new_code(msg.lease_code, deps.querier)
        .map(|lease_code| Config::new(lease_code, msg))
        .and_then(|config| config.store(deps.storage))
        .map(|()| response::empty_response())
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    ProtocolMigrationMessage {
        migrate_from,
        to_release,
        message: MigrateMsg {},
    }: ProtocolMigrationMessage<MigrateMsg>,
) -> ContractResult<Response> {
    migrate_from
        .update_software(&CURRENT_RELEASE, &to_release)
        .map(|()| response::empty_response())
        .map_err(ContractError::UpdateSoftware)
        .inspect_err(platform_error::log(deps.api))
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
                .and_then(|(customer, lease)| Leases::remove(deps.storage, customer, &lease))
                .map(|removed| {
                    debug_assert!(removed);
                    MessageResponse::default()
                })
        }
        ExecuteMsg::MigrateLeases {
            new_code_id,
            max_leases,
            to_release,
        } => ContractOwnerAccess::new(deps.storage.deref())
            .check(&info.sender)
            .map_err(Into::into)
            .and_then(|()| new_code(new_code_id, deps.querier))
            .and_then(|new_lease_code| {
                leaser::try_migrate_leases(
                    deps.storage,
                    CacheFirstRelease::new(deps.querier),
                    new_lease_code,
                    max_leases,
                    migrate_msg(to_release),
                )
            }),
        ExecuteMsg::MigrateLeasesCont {
            key: next_customer,
            max_leases,
            to_release,
        } => ContractOwnerAccess::new(deps.storage.deref())
            .check(&info.sender)
            .map_err(Into::into)
            .and_then(|()| validate_customer(next_customer, deps.api, deps.querier))
            .and_then(|next_customer_validated| {
                leaser::try_migrate_leases_cont(
                    deps.storage,
                    CacheFirstRelease::new(deps.querier),
                    next_customer_validated,
                    max_leases,
                    migrate_msg(to_release),
                )
            }),
    }
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
    match msg {
        SudoMsg::Config {
            lease_interest_rate_margin,
            lease_position_spec,
            lease_due_period,
            lease_max_slippage,
        } => leaser::try_configure(
            deps.storage,
            lease_interest_rate_margin,
            lease_position_spec,
            lease_due_period,
            lease_max_slippage,
        ),
        SudoMsg::CloseProtocol {
            new_lease_code_id,
            migration_spec,
            force,
        } => new_code(new_lease_code_id, deps.querier)
            .and_then(|new_lease_code| {
                leaser::try_close_leases(
                    deps.storage,
                    CacheFirstRelease::new(deps.querier),
                    new_lease_code,
                    MaxLeases::MAX,
                    migrate_msg(ProtocolPackageReleaseId::VOID),
                    force,
                )
            })
            .and_then(|leases_resp| {
                leaser::try_close_protocol(deps.storage, protocols_registry_load, migration_spec)
                    .map(|protocol_resp| leases_resp.merge_with(protocol_resp))
            }),
    }
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::CheckAnomalyResolutionPermission { by: caller } => Leaser::new(deps)
            .config()
            .map(|ref config| AnomalyResolutionPermission::from(config).granted_to(&caller))
            .map(|granted| {
                if granted {
                    AccessGranted::Yes
                } else {
                    AccessGranted::No
                }
            })
            .and_then(serialize_to_json),
        QueryMsg::Config {} => Leaser::new(deps)
            .config()
            .map(|config| ConfigResponse { config })
            .and_then(serialize_to_json),
        QueryMsg::Leases { owner } => Leaser::new(deps)
            .customer_leases(owner)
            .and_then(serialize_to_json),
        QueryMsg::MaxSlippage {} => Leaser::new(deps)
            .config()
            .map(|cfg| cfg.lease_max_slippage)
            .and_then(serialize_to_json),
        QueryMsg::ProtocolPackageRelease {} => serialize_to_json(CURRENT_RELEASE),
        QueryMsg::Quote {
            downpayment,
            lease_asset,
            max_ltd,
        } => Leaser::new(deps)
            .quote(downpayment, lease_asset, max_ltd)
            .and_then(serialize_to_json),
    }
    .inspect_err(platform_error::log(deps.api))
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
        .inspect_err(platform_error::log(deps.api))
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
        .map(|config| config.lease_code)
        .and_then(|lease_code| {
            contract::validate_code_id(deps.querier, &lease, lease_code).map_err(Into::into)
        })
        .map(|()| lease)
}

fn protocols_registry_load(storage: &dyn Storage) -> ContractResult<Addr> {
    Config::load(storage).map(|cfg| cfg.protocols_registry)
}

fn new_code<C>(new_code_id: C, querier: QuerierWrapper<'_>) -> ContractResult<Code>
where
    C: Into<CodeId>,
{
    Code::try_new(new_code_id.into(), &querier).map_err(Into::into)
}

fn migrate_msg(
    to_release: ProtocolPackageReleaseId,
) -> impl Fn(ProtocolPackageRelease) -> ProtocolMigrationMessage<LeaseMigrateMsg> {
    move |migrate_from| ProtocolMigrationMessage {
        migrate_from,
        to_release: to_release.clone(),
        message: LeaseMigrateMsg {},
    }
}

fn finalizer(env: Env) -> Addr {
    env.contract.address
}

fn serialize_to_json<Resp>(resp: Resp) -> ContractResult<Binary>
where
    Resp: Serialize,
{
    cosmwasm_std::to_json_binary(&resp).map_err(ContractError::SerializeToJson)
}
