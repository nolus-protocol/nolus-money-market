use std::ops::{Deref, DerefMut};

use access_control::{ContractOwnerAccess, SingleUserAccess};
use dex::{ConnectionParams, Handler as _, Ics20Channel, Response as DexResponse};
use oracle::stub::OracleRef;
use platform::{message::Response as MessageResponse, response};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};
use timealarms::stub::TimeAlarmsRef;
use versioning::{version, VersionSegment};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    profit::Profit,
    result::ContractResult,
    state::{Config, ConfigManagement as _, SetupDexHandler as _, State},
};

// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    platform::contract::validate_addr(&deps.querier, &msg.treasury)?;
    platform::contract::validate_addr(&deps.querier, &msg.oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.timealarms)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    ContractOwnerAccess::new(deps.storage.deref_mut()).grant_to(&info.sender)?;

    SingleUserAccess::new(
        deps.storage.deref_mut(),
        crate::access_control::TIMEALARMS_NAMESPACE,
    )
    .grant_to(&msg.timealarms)?;

    State::new(Config::new(
        msg.cadence_hours,
        msg.treasury,
        OracleRef::try_from(msg.oracle, &deps.querier)?,
        TimeAlarmsRef::new(msg.timealarms, &deps.querier)?,
    ))
    .store(deps.storage)?;

    Ok(response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .and_then(response::response)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
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

            try_time_alarm(deps, env).map(response::response_only_messages)
        }
        ExecuteMsg::Config { cadence_hours } => {
            ContractOwnerAccess::new(deps.storage.deref()).check(&info.sender)?;

            State::load(deps.storage)?
                .try_update_config(cadence_hours)?
                .store(deps.storage)?;

            Ok(response::empty_response())
        }
        ExecuteMsg::DexCallback() => {
            access_control::check(&env.contract.address, &info.sender)?;

            todo!("implement similarly to contracts/lease/src/state/endpoints.rs")
        }
        ExecuteMsg::DexCallbackContinue() => {
            access_control::check(&env.contract.address, &info.sender)?;

            todo!("implement similarly to contracts/lease/src/state/endpoints.rs")
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: NeutronSudoMsg) -> ContractResult<CwResponse> {
    let state: State = State::load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = try_handle_neutron_msg(deps.as_ref(), env, msg, state)?;

    next_state.store(deps.storage)?;

    Ok(response::response_only_messages(response))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(mut _deps: DepsMut<'_>, _env: Env, _msg: Reply) -> ContractResult<CwResponse> {
    todo!("implement similarly to contracts/lease/src/state/endpoints.rs")
}

fn try_handle_neutron_msg(
    deps: Deps<'_>,
    env: Env,
    msg: NeutronSudoMsg,
    state: State,
) -> ContractResult<DexResponse<State>> {
    match msg {
        NeutronSudoMsg::Response { data, .. } => Result::from(state.on_response(data, deps, env)),
        NeutronSudoMsg::Error { .. } => state.on_error(deps, env).map_err(Into::into),
        NeutronSudoMsg::Timeout { .. } => state.on_timeout(deps, env).map_err(Into::into),
        NeutronSudoMsg::OpenAck {
            port_id: connection_id,
            channel_id: local_endpoint,
            counterparty_channel_id: remote_endpoint,
            counterparty_version,
        } if counterparty_version.is_empty() => state.setup_dex(
            deps,
            env,
            ConnectionParams {
                connection_id,
                transfer_channel: Ics20Channel {
                    local_endpoint,
                    remote_endpoint,
                },
            },
        ),
        NeutronSudoMsg::OpenAck {
            counterparty_version,
            ..
        } => state
            .on_open_ica(counterparty_version, deps, env)
            .map_err(Into::into),
        NeutronSudoMsg::TxQueryResult { .. } | NeutronSudoMsg::KVQueryResult { .. } => {
            unimplemented!()
        }
    }
}

fn try_time_alarm(deps: DepsMut<'_>, env: Env) -> ContractResult<MessageResponse> {
    let state: State = State::load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = Result::from(state.on_time_alarm(deps.as_ref(), env))?;

    next_state.store(deps.storage)?;

    Ok(response)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&Profit::query_config(deps.storage)?),
    }
    .map_err(Into::into)
}
