use std::ops::{Deref, DerefMut};

use access_control::{
    ContractOwnerAccess, SingleUserAccess, permissions::DexResponseSafeDeliveryPermission,
};
use cw_time::IntoInstant;
use dex::{Account, Handler as _, Response as DexResponse};
use oracle_platform::OracleRef;
use platform::{
    batch::Batch,
    contract::{self, CodeId, Validator},
    error as platform_error,
    message::Response as MessageResponse,
    reply, response,
    state_machine::Response as StateMachineResponse,
};
use sdk::{
    api::SudoMsg,
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        Addr, Api, Binary, CodeInfoResponse, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
        Reply, WasmMsg, entry_point, instantiate2_address,
    },
};
use timealarms::stub::TimeAlarmsRef;
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, VersionSegment, package_name, package_version,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    profit::Profit,
    result::ContractResult,
    state::{Config, ConfigManagement as _, State, VaultConfig},
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

/// The `Instantiate2` salt the drain vault is precomputed and instantiated
/// under. The profit instantiates exactly one vault, so a fixed salt suffices.
const VAULT_SALT: &[u8] = b"profit-drain-vault";

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    let addr_validator = contract::validator(deps.querier);
    addr_validator.check_contract(&msg.treasury)?;
    addr_validator.check_contract(&msg.oracle)?;
    addr_validator.check_contract(&msg.remote_profit_controller)?;
    // msg.timealarms is validated on TimeAlarmsRef instantiation

    let vault_code = msg.vault_code_id.try_validate(&addr_validator)?;
    let vault_code_id = CodeId::from(vault_code);
    let drain_vault = precompute_vault_address(deps.api, deps.querier, vault_code_id, &env)?;

    ContractOwnerAccess::new(deps.storage.deref_mut()).grant_to(&info)?;

    SingleUserAccess::new(
        deps.storage.deref_mut(),
        crate::access_control::TIMEALARMS_NAMESPACE,
    )
    .grant_to(&msg.timealarms)?;

    let config = Config::new(
        msg.cadence_hours,
        msg.treasury,
        OracleRef::try_from_base(msg.oracle, deps.querier)?,
        TimeAlarmsRef::new(msg.timealarms, &addr_validator)?,
        Account::funding(env.contract.address.clone(), msg.dex),
        msg.remote_profit_controller,
        VaultConfig {
            code_id: vault_code,
            address: drain_vault.clone(),
        },
    );

    let state = State::start(config);
    let vault_msgs = instantiate_vault(&env, vault_code_id)?;

    state
        .store(deps.storage)
        .map(|()| response::response_only_messages(vault_msgs))
}

/// Migration is unsupported: the remote-swap profit is deployed fresh and the
/// admin registry/time-alarms are repointed by governance, never migrated from
/// the retired ICA profit. The handler always errs, leaving stored state
/// untouched (FM5 hard-reject).
#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    _msg: ProtocolMigrationMessage<MigrateMsg>,
) -> ContractResult<CwResponse> {
    Err(ContractError::MigrationUnsupported).inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => {
            SingleUserAccess::new(
                deps.storage.deref(),
                crate::access_control::TIMEALARMS_NAMESPACE,
            )
            .check(&info)?;

            try_handle_execute_message(deps, env, |state, querier, env| {
                State::on_time_alarm(state, querier, env, info)
            })
            .map(response::response_only_messages)
        }
        ExecuteMsg::Config { cadence_hours } => {
            ContractOwnerAccess::new(deps.storage.deref()).check(&info)?;

            let StateMachineResponse {
                response,
                next_state,
            } = State::load(deps.storage)?
                .try_update_config(env.block.time.into_instant(), cadence_hours)?;

            next_state.store(deps.storage)?;

            Ok(response::response_only_messages(response))
        }
        ExecuteMsg::DexCallback() => {
            access_control::check(
                &DexResponseSafeDeliveryPermission::new(&env.contract),
                &info,
            )?;

            try_handle_execute_message(deps, env, State::on_inner)
                .map(response::response_only_messages)
        }
        ExecuteMsg::RemoteProfitCallback(callback) => {
            try_handle_execute_message(deps, env, |state, querier, env| {
                State::on_remote_profit_callback(state, callback, info, querier, env)
            })
            .map(response::response_only_messages)
        }
        ExecuteMsg::Heal() => try_handle_execute_message(deps, env, |machine, querier, env| {
            State::heal(machine, querier, env, &info)
        })
        .map(response::response_only_messages),
    }
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    State::load(deps.storage)
        .and_then(|state| try_handle_neutron_msg(deps.api, deps.as_ref(), env, msg, state))
        .and_then(
            |DexResponse::<State> {
                 response,
                 next_state,
             }| { next_state.store(deps.storage).map(|()| response) },
        )
        .map(response::response_only_messages)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> ContractResult<CwResponse> {
    let instantiated = reply::from_instantiate2_addr_only(deps.api, msg)?;

    let (batch, next_state) = State::load(deps.storage)?.on_vault_instantiated(instantiated)?;

    next_state
        .store(deps.storage)
        .map(|()| response::response_only_messages(batch))
        .inspect_err(platform_error::log(deps.api))
}

fn try_handle_neutron_msg(
    api: &dyn Api,
    deps: Deps<'_>,
    env: Env,
    msg: SudoMsg,
    state: State,
) -> ContractResult<DexResponse<State>> {
    match msg {
        SudoMsg::Response { data, .. } => state.on_response(data, deps.querier, env).into(),
        SudoMsg::Error { details, .. } => {
            let resp = details.into();
            api.debug(&format!("SudoMsg::Error({resp})",));
            state.on_error(resp, deps.querier, env).into()
        }
        SudoMsg::Timeout { .. } => state.on_timeout(deps.querier, env).map_err(Into::into),
        // The remote-swap profit funds over the ICS-20 transfer channel and
        // registers no ICA, so it can never receive an `OpenAck`.
        SudoMsg::OpenAck { .. } => Err(ContractError::unsupported_operation("open ica response")),
    }
}

fn try_handle_execute_message<F, R, E>(
    deps: DepsMut<'_>,
    env: Env,
    handler: F,
) -> ContractResult<MessageResponse>
where
    F: FnOnce(State, QuerierWrapper<'_>, Env) -> R,
    R: Into<Result<DexResponse<State>, E>>,
    ContractError: From<E>,
{
    let state: State = State::load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = handler(state, deps.querier, env).into()?;

    next_state.store(deps.storage).map(|()| response)
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => cosmwasm_std::to_json_binary(&Profit::query_config(
            deps.storage,
            env.block.time.into_instant(),
            deps.querier,
        )?),
        QueryMsg::ProtocolPackageRelease {} => cosmwasm_std::to_json_binary(&CURRENT_RELEASE),
    }
    .map_err(Into::into)
}

/// Precompute the drain vault's `Instantiate2` address from the vault code's
/// checksum, this contract's canonical address (the creator), and a fixed salt.
/// Mirrors the admin contract's precompute → commit → instantiate → verify
/// template (`platform/contracts/admin/src/endpoints.rs`).
fn precompute_vault_address(
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
    vault_code_id: CodeId,
    env: &Env,
) -> ContractResult<Addr> {
    let CodeInfoResponse { checksum, .. } = querier.query_wasm_code_info(vault_code_id)?;
    let creator = api.addr_canonicalize(env.contract.address.as_str())?;
    let canonical = instantiate2_address(checksum.as_ref(), &creator, VAULT_SALT)?;
    api.addr_humanize(&canonical).map_err(Into::into)
}

fn instantiate_vault(env: &Env, vault_code_id: CodeId) -> ContractResult<Batch> {
    let init_msg = cosmwasm_std::to_json_vec(&drain_vault::api::InstantiateMsg {
        owner: env.contract.address.to_string(),
    })?;

    let mut batch = Batch::default();
    batch.schedule_execute_reply_on_success(
        WasmMsg::Instantiate2 {
            admin: Some(env.contract.address.to_string()),
            code_id: vault_code_id,
            label: format!("drain vault for {}", env.contract.address),
            msg: Binary::new(init_msg),
            funds: vec![],
            salt: Binary::new(VAULT_SALT.to_vec()),
        },
        Default::default(),
    );
    Ok(batch)
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use sdk::cosmwasm_std::testing::{mock_dependencies, mock_env};
    use versioning::{
        ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
        package_name, package_version,
    };

    use crate::{error::ContractError, msg::MigrateMsg};

    use super::{CONTRACT_STORAGE_VERSION, migrate};

    /// C-FM5a: migration is hard-rejected unconditionally — the remote-swap
    /// profit is deployed fresh, never migrated from the ICA profit.
    #[test]
    fn migrate_rejects_unconditionally() {
        let mut deps = mock_dependencies();
        let err = migrate(deps.as_mut(), mock_env(), migrate_msg()).unwrap_err();
        assert!(
            matches!(err, ContractError::MigrationUnsupported),
            "got {err:?}",
        );
    }

    fn migrate_msg() -> ProtocolMigrationMessage<MigrateMsg> {
        const SOFTWARE_ID: &str = env!("SOFTWARE_RELEASE_ID");
        const PROTOCOL_ID: &str = env!("PROTOCOL_RELEASE_ID");
        ProtocolMigrationMessage {
            migrate_from: ProtocolPackageRelease::current(
                package_name!(),
                package_version!(),
                CONTRACT_STORAGE_VERSION,
            ),
            to_release: ProtocolPackageReleaseId::new(
                ReleaseId::new_test(SOFTWARE_ID),
                ReleaseId::new_test(PROTOCOL_ID),
            ),
            message: MigrateMsg {},
        }
    }
}
