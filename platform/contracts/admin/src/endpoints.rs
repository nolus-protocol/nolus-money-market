use access_control::ContractOwnerAccess;
use platform::{batch::Batch, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        self, entry_point, Addr, Api, Binary, CodeInfoResponse, Deps, DepsMut, Env, MessageInfo,
        QuerierWrapper, Reply, Storage, WasmMsg,
    },
};
use versioning::{
    package_name, package_version, PlatformMigrationMessage, PlatformPackageRelease,
    ProtocolPackageReleaseId, UpdatablePackage as _, VersionSegment,
};

use crate::{
    contracts::{MigrationSpec, Protocol, ProtocolContracts},
    error::Error as ContractError,
    msg::{
        ExecuteMsg, InstantiateMsg, MigrateContracts, MigrateMsg, PlatformQueryResponse,
        ProtocolQueryResponse, ProtocolsQueryResponse, QueryMsg, SudoMsg,
    },
    result::Result as ContractResult,
    state::{contract::ExpectedInstantiation, contracts as state_contracts},
    validate::Validate as _,
};

// version info for migration info
const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 3;
const CONTRACT_STORAGE_VERSION: VersionSegment = CONTRACT_STORAGE_VERSION_FROM + 1;
const CURRENT_RELEASE: PlatformPackageRelease = PlatformPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _: Env,
    _: MessageInfo,
    InstantiateMsg {
        ref dex_admin,
        contracts,
    }: InstantiateMsg,
) -> ContractResult<CwResponse> {
    ContractOwnerAccess::new(deps.branch().storage).grant_to(dex_admin)?;

    contracts.validate(deps.querier)?;

    state_contracts::store(deps.storage, contracts).map(|()| response::empty_response())
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _: Env,
    PlatformMigrationMessage {
        to_release,
        message: MigrateMsg {},
    }: PlatformMigrationMessage<MigrateMsg>,
) -> ContractResult<CwResponse> {
    PlatformPackageRelease::pull_prev(package_name!(), deps.storage)
        .and_then(|previous| previous.update_software(&CURRENT_RELEASE, &to_release))
        .map(|()| response::empty_response())
        .map_err(Into::into)
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::Instantiate {
            code_id,
            expected_address,
            protocol,
            label,
            message,
        } => {
            ensure_sender_is_owner(deps.storage, &info.sender)?;

            ExpectedInstantiation::new(code_id.u64(), expected_address).store(deps.storage)?;

            let mut batch: Batch = Batch::default();

            batch.schedule_execute_reply_on_success(
                WasmMsg::Instantiate2 {
                    admin: Some(env.contract.address.into_string()),
                    code_id: code_id.u64(),
                    label,
                    msg: Binary::new(message.into_bytes()),
                    funds: info.funds,
                    salt: Binary::new(protocol.into_bytes()),
                },
                Default::default(),
            );

            Ok(response::response_only_messages(batch))
        }
        ExecuteMsg::RegisterProtocol { name, ref protocol } => {
            ensure_sender_is_owner(deps.storage, &info.sender)?;

            register_protocol(deps.storage, deps.querier, name, protocol)
        }
        ExecuteMsg::DeregisterProtocol(migration_spec) => {
            deregister_protocol(deps.storage, &info.sender, migration_spec)
        }
    }
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::ChangeDexAdmin { new_dex_admin } => deps
            .api
            .addr_validate(new_dex_admin.as_str())
            .map_err(Into::into)
            .and_then(|new_dex_admin| {
                ContractOwnerAccess::new(deps.storage)
                    .grant_to(&new_dex_admin)
                    .map(|()| response::empty_response())
                    .map_err(Into::into)
            }),
        SudoMsg::RegisterProtocol { name, ref protocol } => {
            register_protocol(deps.storage, deps.querier, name, protocol)
        }
        SudoMsg::MigrateContracts(MigrateContracts {
            to_release,
            migration_spec,
        }) => crate::contracts::migrate(
            deps.storage,
            env.contract.address,
            to_release,
            migration_spec,
        )
        .map(response::response_only_messages),
        SudoMsg::ExecuteContracts(execute_messages) => {
            crate::contracts::execute(deps.storage, execute_messages)
                .map(response::response_only_messages)
        }
    }
}

#[entry_point]
pub fn reply(deps: DepsMut<'_>, _: Env, msg: Reply) -> ContractResult<CwResponse> {
    let expected_instantiation = ExpectedInstantiation::load(deps.storage)?;

    ExpectedInstantiation::clear(deps.storage);

    instantiate_reply(deps.api, deps.querier, msg, expected_instantiation)
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::InstantiateAddress { code_id, protocol } => {
            let CodeInfoResponse { checksum, .. } =
                deps.querier.query_wasm_code_info(code_id.u64())?;

            let creator = deps.api.addr_canonicalize(env.contract.address.as_str())?;

            let canonical_addr =
                cosmwasm_std::instantiate2_address(checksum.as_ref(), &creator, protocol.as_ref())?;

            let addr = deps.api.addr_humanize(&canonical_addr)?;

            cosmwasm_std::to_json_binary(&addr).map_err(From::from)
        }
        QueryMsg::Protocols {} => {
            state_contracts::protocols(deps.storage).and_then(|ref protocols| {
                cosmwasm_std::to_json_binary::<ProtocolsQueryResponse>(protocols)
                    .map_err(Into::into)
            })
        }
        QueryMsg::Platform {} => {
            state_contracts::load_platform(deps.storage).and_then(|ref platform| {
                cosmwasm_std::to_json_binary::<PlatformQueryResponse>(platform).map_err(Into::into)
            })
        }
        QueryMsg::Protocol(protocol) => state_contracts::load_protocol(deps.storage, protocol)
            .and_then(|ref protocol| {
                cosmwasm_std::to_json_binary::<ProtocolQueryResponse>(protocol).map_err(Into::into)
            }),
        QueryMsg::PlatformPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
}

fn instantiate_reply(
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
    msg: Reply,
    expected_instantiation: ExpectedInstantiation,
) -> ContractResult<CwResponse> {
    let instantiated_addr = platform::reply::from_instantiate2_addr_only(api, msg)?;

    if instantiated_addr != expected_instantiation.address() {
        return Err(ContractError::DifferentInstantiatedAddress {
            reported: instantiated_addr,
            expected: expected_instantiation.into_address(),
        });
    }

    let reported_code_id = querier.query_wasm_contract_info(instantiated_addr)?.code_id;

    if reported_code_id == expected_instantiation.code_id() {
        Ok(response::empty_response())
    } else {
        Err(ContractError::DifferentInstantiatedCodeId {
            reported: reported_code_id,
            expected: expected_instantiation.code_id(),
        })
    }
}

fn ensure_sender_is_owner(storage: &mut dyn Storage, sender: &Addr) -> ContractResult<()> {
    ContractOwnerAccess::new(storage)
        .check(sender)
        .map_err(Into::into)
}

fn register_protocol(
    storage: &mut dyn Storage,
    querier: QuerierWrapper<'_>,
    name: String,
    protocol: &Protocol<Addr>,
) -> ContractResult<CwResponse> {
    protocol.validate(querier)?;

    state_contracts::add_protocol(storage, name, protocol).map(|()| response::empty_response())
}

fn deregister_protocol(
    storage: &mut dyn Storage,
    sender: &Addr,
    migration_spec: ProtocolContracts<MigrationSpec>,
) -> ContractResult<CwResponse> {
    state_contracts::protocols(storage)?
        .into_iter()
        .find_map(|name| {
            state_contracts::load_protocol(storage, name.clone())
                .map(|protocol| {
                    (protocol.contracts.leaser == sender)
                        .then_some(protocol.contracts)
                        .inspect(|_| () = state_contracts::remove_protocol(storage, name))
                })
                .transpose()
        })
        .unwrap_or(Err(ContractError::SenderNotARegisteredLeaser {}))
        .and_then(|protocol| {
            protocol
                .migrate_standalone(ProtocolPackageReleaseId::VOID, migration_spec)
                .map(response::response_only_messages)
        })
}

#[test]
fn test_release() {
    assert_eq!(
        Ok(QueryMsg::PlatformPackageRelease {}),
        platform::tests::ser_de(&versioning::query::PlatformPackage::Release {}),
    );
}
