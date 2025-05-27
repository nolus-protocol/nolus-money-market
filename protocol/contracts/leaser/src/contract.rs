use std::ops::{Deref, DerefMut};

use serde::Serialize;

use access_control::{ContractOwnerAccess, SingleUserPermission};
use lease::api::{MigrateMsg as LeaseMigrateMsg, authz::AccessGranted};
use platform::{
    contract::{self, Code, CodeId, Validator},
    error as platform_error,
    message::Response as MessageResponse,
    reply, response,
};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Storage, entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, UpdatablePackage,
    VersionSegment, package_name, package_version,
};

use crate::{
    cmd::Borrow,
    error::ContractError,
    lease::CacheFirstRelease,
    leaser::{self, Leaser},
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    state::{config::Config, leases::Leases},
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 5;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

pub type LeasesConfigurationPermission<'a> = SingleUserPermission<'a>;
pub type ChangeLeaseAdminPermission<'a> = SingleUserPermission<'a>;
pub type AnomalyResolutionPermission<'a> = SingleUserPermission<'a>;

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    let addr_validator = contract::validator(deps.querier);
    addr_validator.check_contract(&msg.lpp)?;
    addr_validator.check_contract(&msg.profit)?;
    addr_validator.check_contract(&msg.reserve)?;
    addr_validator.check_contract(&msg.time_alarms)?;
    addr_validator.check_contract(&msg.market_price_oracle)?;
    addr_validator.check_contract(&msg.protocols_registry)?;

    validate(&msg.lease_admin, deps.api)?;

    ContractOwnerAccess::new(deps.storage.deref_mut()).grant_to(&info.sender)?;

    new_code(msg.lease_code, &addr_validator)
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
        ExecuteMsg::ConfigLeases(new_config) => {
            Leaser::new(deps.as_ref()).config().and_then(|config| {
                access_control::check(
                    &LeasesConfigurationPermission::new(&config.lease_admin),
                    &info.sender,
                )?;
                leaser::try_configure(deps.storage, new_config)
            })
        }
        ExecuteMsg::FinalizeLease { customer } => {
            let addr_validator = contract::validator(deps.querier);
            validate_customer(customer, deps.api, &addr_validator)
                .and_then(|customer| {
                    validate_lease(info.sender, deps.as_ref(), &addr_validator)
                        .map(|lease| (customer, lease))
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
            .and_then(|()| new_code(new_code_id, &contract::validator(deps.querier)))
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
            .and_then(|()| {
                validate_customer(next_customer, deps.api, &contract::validator(deps.querier))
            })
            .and_then(|next_customer_validated| {
                leaser::try_migrate_leases_cont(
                    deps.storage,
                    CacheFirstRelease::new(deps.querier),
                    next_customer_validated,
                    max_leases,
                    migrate_msg(to_release),
                )
            }),
        ExecuteMsg::ChangeLeaseAdmin { new } => Leaser::new(deps.as_ref())
            .config()
            .and_then(|config| {
                access_control::check(
                    &ChangeLeaseAdminPermission::new(&config.lease_admin),
                    &info.sender,
                )?;
                validate(&new, deps.api)
            })
            .and_then(|valid_new_admin| {
                leaser::try_change_lease_admin(deps.storage, valid_new_admin)
            }),
    }
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
    match msg {
        SudoMsg::Config(new_config) => leaser::try_configure(deps.storage, new_config),
        SudoMsg::ChangeLeaseAdmin { new } => validate(&new, deps.api).and_then(|validated_admin| {
            leaser::try_change_lease_admin(deps.storage, validated_admin)
        }),
        SudoMsg::CloseProtocol { migration_spec } => check_no_leases(deps.storage)
            .and_then(|()| leaser::try_close_deposits(deps.storage, deps.querier))
            .and_then(|response| {
                leaser::try_close_protocol(deps.storage, protocols_registry_load, migration_spec)
                    .map(|protocol_resp| response.merge_with(protocol_resp))
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
            .and_then(|config| {
                access_control::check(
                    &AnomalyResolutionPermission::new(&config.lease_admin),
                    &caller,
                )
                .map(|_| AccessGranted::Yes)
                .or_else(|_| Ok(AccessGranted::No))
            })
            .and_then(serialize_to_json),
        QueryMsg::Config {} => Leaser::new(deps)
            .config()
            .map(|config| ConfigResponse { config })
            .and_then(serialize_to_json),
        QueryMsg::Leases { owner } => Leaser::new(deps)
            .customer_leases(owner)
            .and_then(serialize_to_json),
        QueryMsg::MaxSlippages {} => Leaser::new(deps)
            .config()
            .map(|cfg| cfg.lease_max_slippages)
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
        .map_err(|err| ContractError::Parsing {
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

fn validate_customer<V>(customer: Addr, api: &dyn Api, addr_validator: &V) -> ContractResult<Addr>
where
    V: Validator,
{
    api.addr_validate(customer.as_str())
        .map_err(|_| ContractError::InvalidContinuationKey {
            err: "invalid address".into(),
        })
        .and_then(|next_customer| {
            addr_validator
                .check_contract(&next_customer)
                .is_err()
                .then_some(next_customer)
                .ok_or_else(|| ContractError::InvalidContinuationKey {
                    err: "smart contract key".into(),
                })
        })
}

fn validate_lease<V>(lease: Addr, deps: Deps<'_>, addr_validator: &V) -> ContractResult<Addr>
where
    V: Validator,
{
    Leaser::new(deps)
        .config()
        .map(|config| config.lease_code)
        .and_then(|lease_code| {
            addr_validator
                .check_contract_code(lease, &lease_code)
                .map_err(Into::into)
        })
}

fn check_no_leases(storage: &dyn Storage) -> ContractResult<()> {
    if Leases::iter(storage, None).next().is_some() {
        Err(ContractError::ProtocolStillInUse())
    } else {
        Ok(())
    }
}

fn protocols_registry_load(storage: &dyn Storage) -> ContractResult<Addr> {
    Config::load(storage).map(|cfg| cfg.protocols_registry)
}

fn new_code<C, V>(new_code_id: C, addr_validator: &V) -> ContractResult<Code>
where
    C: Into<CodeId>,
    V: Validator,
{
    Code::try_new(new_code_id.into(), addr_validator).map_err(Into::into)
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

fn validate(addr: &Addr, sdk_api: &dyn Api) -> ContractResult<Addr> {
    sdk_api
        .addr_validate(addr.as_str())
        .map_err(ContractError::InvalidAddress)
}

fn serialize_to_json<Resp>(resp: Resp) -> ContractResult<Binary>
where
    Resp: Serialize,
{
    cosmwasm_std::to_json_binary(&resp).map_err(ContractError::SerializeToJson)
}
