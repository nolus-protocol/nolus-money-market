use access_control::ContractOwnerAccess;
use platform::{batch::Batch, contract::CodeId, response};
#[cfg(feature = "cosmwasm-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        ensure_eq, Addr, Binary, CodeInfoResponse, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
        Reply, StdError as CwError, Storage, WasmMsg,
    },
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use self::{
    contracts::Protocol,
    error::Error as ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateContracts, MigrateMsg, QueryMsg, SudoMsg},
    result::Result as ContractResult,
    state::{contract::Contract as ContractState, contracts as state_contracts},
    validate::Validate as _,
};

pub mod contracts;
pub mod error;
pub mod msg;
pub mod result;
pub mod state;
pub mod validate;

// version info for migration info
const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    InstantiateMsg {
        ref dex_admin,
        contracts,
    }: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, CONTRACT_VERSION)?;

    ContractOwnerAccess::new(deps.branch().storage).grant_to(dex_admin)?;

    contracts.validate(deps.querier)?;

    state_contracts::store(deps.storage, contracts).map(|()| response::empty_response())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn migrate(
    mut deps: DepsMut<'_>,
    _env: Env,
    MigrateMsg {
        protocol_name,
        ref dex_admin,
    }: MigrateMsg,
) -> ContractResult<CwResponse> {
    ContractOwnerAccess::new(deps.branch().storage)
        .grant_to(dex_admin)
        .map_err(From::from)
        .and_then(|()| {
            versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _, _>(
                deps.storage,
                CONTRACT_VERSION,
                |storage: &mut dyn Storage| state_contracts::migrate(storage, protocol_name),
                Into::into,
            )
        })
        .and_then(|(label, ())| response::response(label))
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    ContractOwnerAccess::new(deps.branch().storage).check(&info.sender)?;

    match msg {
        ExecuteMsg::Instantiate {
            code_id,
            expected_address,
            protocol,
            label,
            message,
        } => {
            ContractState::Instantiate {
                expected_code_id: code_id,
                expected_address,
            }
            .store(deps.storage)?;

            let mut batch: Batch = Batch::default();

            batch.schedule_execute_on_success_reply(
                WasmMsg::Instantiate2 {
                    admin: Some(env.contract.address.into_string()),
                    code_id,
                    label,
                    msg: Binary(message.into_bytes()),
                    funds: info.funds,
                    salt: Binary(protocol.into_bytes()),
                },
                Default::default(),
            );

            Ok(response::response_only_messages(batch))
        }
        ExecuteMsg::RegisterProtocol {
            name,
            ref contracts,
        } => register_protocol(deps.storage, deps.querier, name, contracts),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::ChangeDexAdmin { ref new_dex_admin } => ContractOwnerAccess::new(deps.storage)
            .grant_to(new_dex_admin)
            .map(|()| response::empty_response())
            .map_err(Into::into),
        SudoMsg::RegisterProtocol {
            name,
            ref contracts,
        } => register_protocol(deps.storage, deps.querier, name, contracts),
        SudoMsg::MigrateContracts(MigrateContracts {
            release,
            admin_contract,
            migration_spec,
            post_migration_execute,
        }) => contracts::migrate(
            deps.storage,
            env.contract.address,
            release,
            admin_contract,
            migration_spec,
            post_migration_execute,
        )
        .map(response::response_only_messages),
    }
}

fn register_protocol(
    storage: &mut dyn Storage,
    querier: QuerierWrapper<'_>,
    name: String,
    contracts: &Protocol<Addr>,
) -> ContractResult<CwResponse> {
    contracts.validate(querier)?;

    state_contracts::add_protocol_set(storage, name, contracts).map(|()| response::empty_response())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<CwResponse> {
    match ContractState::load(deps.storage)? {
        ContractState::Migration { release } => migration_reply(msg, release),
        ContractState::Instantiate {
            expected_code_id,
            expected_address,
        } => instantiate_reply(deps.querier, msg, expected_code_id, expected_address),
    }
}

fn migration_reply(msg: Reply, expected_release: String) -> ContractResult<CwResponse> {
    let reported_release: String =
        platform::reply::from_execute(msg)?.ok_or(ContractError::NoMigrationResponseData {})?;

    ensure_eq!(
        reported_release,
        expected_release,
        ContractError::WrongRelease {
            reported: reported_release,
            expected: expected_release
        }
    );

    Ok(response::empty_response())
}

fn instantiate_reply(
    querier: QuerierWrapper<'_>,
    msg: Reply,
    expected_code_id: CodeId,
    expected_addr: Addr,
) -> ContractResult<CwResponse> {
    let instantiated_addr = msg
        .result
        .into_result()
        .map_err(CwError::generic_err)?
        .events
        .iter()
        .find_map(|event| {
            if event.ty == "wasm" {
                event.attributes.iter().find_map(|attribute| {
                    if attribute.key == "instantiate" && attribute.value == expected_addr.as_str() {
                        Some(&attribute.value)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
        .ok_or(ContractError::FindContractAddress {})?
        .clone();

    let reported_code_id = querier.query_wasm_contract_info(instantiated_addr)?.code_id;

    if reported_code_id == expected_code_id {
        Ok(response::empty_response())
    } else {
        Err(ContractError::DifferentInstantiatedCodeId {
            reported: reported_code_id,
            expected: expected_code_id,
        })
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::InstantiateAddress { code_id, protocol } => {
            let CodeInfoResponse {
                creator, checksum, ..
            } = deps.querier.query_wasm_code_info(code_id)?;

            sdk::cosmwasm_std::to_json_binary(&deps.api.addr_humanize(
                &sdk::cosmwasm_std::instantiate2_address(
                    &checksum,
                    &deps.api.addr_canonicalize(&creator)?,
                    protocol.as_bytes(),
                )?,
            )?)
            .map_err(From::from)
        }
    }
}
