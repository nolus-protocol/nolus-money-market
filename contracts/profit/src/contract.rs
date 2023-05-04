use access_control::SingleUserAccess;
use dex::{Handler as _, Ics20Channel, Response as DexResponse};
use platform::{message::Response as MessageResponse, response};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo},
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};
use versioning::{version, VersionSegment};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    profit::Profit,
    result::ContractResult,
    state::{Config, ConfigManagement as _, ProfitMessageHandler as _, State},
    ContractError,
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    platform::contract::validate_addr(&deps.querier, &msg.treasury)?;
    platform::contract::validate_addr(&deps.querier, &msg.oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.timealarms)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::new_contract_owner(info.sender).store(deps.storage)?;

    SingleUserAccess::new(
        crate::access_control::TIMEALARMS_NAMESPACE,
        msg.timealarms.clone(),
    )
    .store(deps.storage)?;

    State::new(
        &deps.querier,
        Config::new(msg.cadence_hours, msg.treasury),
        msg.connection_id,
        msg.oracle,
        msg.timealarms,
    )?
    .store(deps.storage)?;

    Ok(response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software::<ContractError>(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .and_then(response::response)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => {
            SingleUserAccess::load(deps.storage, crate::access_control::TIMEALARMS_NAMESPACE)?
                .check_access(&info.sender)?;

            let alarm_recepient = env.contract.address.clone();

            try_time_alarm(deps.branch(), env).and_then(|resp| {
                response::response_with_messages::<_, _, ContractError>(&alarm_recepient, resp)
            })
        }
        ExecuteMsg::Config { cadence_hours } => {
            SingleUserAccess::check_owner_access::<ContractError>(deps.storage, &info.sender)?;

            State::load(deps.storage)?
                .try_update_config(cadence_hours)?
                .store(deps.storage)?;

            Ok(response::empty_response())
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: NeutronSudoMsg) -> ContractResult<CwResponse> {
    let state: State = State::load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = match msg {
        NeutronSudoMsg::Response { data, .. } => state
            .on_response(data, deps.as_ref(), env)
            .continue_or_ok()?,
        NeutronSudoMsg::Error { .. } => state.on_error(deps.as_ref(), env)?,
        NeutronSudoMsg::Timeout { .. } => state.on_timeout(deps.as_ref(), env)?,
        NeutronSudoMsg::OpenAck {
            channel_id,
            counterparty_channel_id,
            counterparty_version,
            ..
        } => state.confirm_open(
            deps.as_ref(),
            env,
            Ics20Channel {
                local_endpoint: channel_id,
                remote_endpoint: counterparty_channel_id,
            },
            counterparty_version,
        )?,
        NeutronSudoMsg::TxQueryResult { .. } => {
            unimplemented!()
        }
        NeutronSudoMsg::KVQueryResult { .. } => {
            unimplemented!()
        }
    };

    next_state.store(deps.storage)?;

    Ok(response::response_only_messages(response))
}

fn try_time_alarm(deps: DepsMut<'_>, env: Env) -> ContractResult<MessageResponse> {
    let state: State = State::load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = state.on_time_alarm(deps.as_ref(), env).continue_or_ok()?;

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
