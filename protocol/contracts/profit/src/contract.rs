use std::ops::{Deref, DerefMut};

use access_control::{ContractOwnerAccess, SingleUserAccess};
use dex::{ContinueResult as DexResult, Handler as _, Response as DexResponse};
use oracle_platform::OracleRef;
use platform::{
    error as platform_error, message::Response as MessageResponse, response,
    state_machine::Response as StateMachineResponse,
};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, entry_point, to_json_binary,
    },
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};
use timealarms::stub::TimeAlarmsRef;
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    profit::Profit,
    result::ContractResult,
    state::{Config, ConfigManagement as _, State},
};

const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
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
) -> ContractResult<CwResponse> {
    platform::contract::validate_addr(deps.querier, &msg.treasury)?;
    platform::contract::validate_addr(deps.querier, &msg.oracle)?;
    platform::contract::validate_addr(deps.querier, &msg.timealarms)?;

    ContractOwnerAccess::new(deps.storage.deref_mut()).grant_to(&info.sender)?;

    SingleUserAccess::new(
        deps.storage.deref_mut(),
        crate::access_control::TIMEALARMS_NAMESPACE,
    )
    .grant_to(&msg.timealarms)?;

    let (state, response) = State::start(
        Config::new(
            msg.cadence_hours,
            msg.treasury,
            OracleRef::try_from_base(msg.oracle, deps.querier)?,
            TimeAlarmsRef::new(msg.timealarms, deps.querier)?,
        ),
        msg.dex,
    );

    state
        .store(deps.storage)
        .map(|()| response::response_only_messages(response))
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
) -> ContractResult<CwResponse> {
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
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => {
            SingleUserAccess::new(
                deps.storage.deref(),
                crate::access_control::TIMEALARMS_NAMESPACE,
            )
            .check(&info.sender)?;

            try_handle_execute_message(deps, env, State::on_time_alarm)
                .map(response::response_only_messages)
        }
        ExecuteMsg::Config { cadence_hours } => {
            ContractOwnerAccess::new(deps.storage.deref()).check(&info.sender)?;

            let StateMachineResponse {
                response,
                next_state,
            } = State::load(deps.storage)?.try_update_config(env.block.time, cadence_hours)?;

            next_state.store(deps.storage)?;

            Ok(response::response_only_messages(response))
        }
        ExecuteMsg::DexCallback() => {
            access_control::check(&env.contract.address, &info.sender)?;

            try_handle_execute_message(deps, env, State::on_inner)
                .map(response::response_only_messages)
        }
        ExecuteMsg::DexCallbackContinue() => {
            access_control::check(&env.contract.address, &info.sender)?;

            try_handle_execute_message(deps, env, State::on_inner_continue)
                .map(response::response_only_messages)
        }
    }
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: NeutronSudoMsg) -> ContractResult<CwResponse> {
    let state: State = State::load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = try_handle_neutron_msg(deps.as_ref(), env, msg, state)?;

    next_state.store(deps.storage)?;

    Ok(response::response_only_messages(response))
}

#[entry_point]
pub fn reply(deps: DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    try_handle_reply_message(deps, env, msg, State::reply).map(response::response_only_messages)
}

fn try_handle_neutron_msg(
    deps: Deps<'_>,
    env: Env,
    msg: NeutronSudoMsg,
    state: State,
) -> ContractResult<DexResponse<State>> {
    match msg {
        NeutronSudoMsg::Response { data, .. } => {
            Result::from(state.on_response(data, deps.querier, env))
        }
        NeutronSudoMsg::Error { .. } => state.on_error(deps.querier, env).map_err(Into::into),
        NeutronSudoMsg::Timeout { .. } => state.on_timeout(deps.querier, env).map_err(Into::into),
        NeutronSudoMsg::OpenAck {
            counterparty_version,
            ..
        } => state
            .on_open_ica(counterparty_version, deps.querier, env)
            .map_err(Into::into),
        NeutronSudoMsg::TxQueryResult { .. } | NeutronSudoMsg::KVQueryResult { .. } => {
            unimplemented!()
        }
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

fn try_handle_reply_message<F>(
    deps: DepsMut<'_>,
    env: Env,
    msg: Reply,
    handler: F,
) -> ContractResult<MessageResponse>
where
    F: FnOnce(State, QuerierWrapper<'_>, Env, Reply) -> DexResult<State>,
{
    let state: State = State::load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = handler(state, deps.querier, env, msg)?;

    next_state.store(deps.storage).map(|()| response)
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&Profit::query_config(
            deps.storage,
            env.block.time,
            deps.querier,
        )?),
        QueryMsg::ProtocolPackageRelease {} => to_json_binary(&CURRENT_RELEASE),
    }
    .map_err(Into::into)
}
