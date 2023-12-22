use currencies::LeaseGroup;
use platform::{error as platform_error, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    api::{ExecuteMsg, MigrateMsg, NewLeaseContract, StateQuery},
    contract::api::Contract,
    error::ContractResult,
};

use super::state::{self, Response, State};

const CONTRACT_STORAGE_VERSION: VersionSegment = 7;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[entry_point]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    new_lease: NewLeaseContract,
) -> ContractResult<CwResponse> {
    //TODO move the following validations into the deserialization
    deps.api.addr_validate(new_lease.finalizer.as_str())?;
    currency::validate::<LeaseGroup>(&new_lease.form.currency)?;
    deps.api.addr_validate(new_lease.form.customer.as_str())?;

    platform::contract::validate_addr(deps.querier, &new_lease.form.time_alarms)?;
    platform::contract::validate_addr(deps.querier, &new_lease.form.market_price_oracle)?;
    platform::contract::validate_addr(deps.querier, &new_lease.form.loan.lpp)?;
    platform::contract::validate_addr(deps.querier, &new_lease.form.loan.profit)?;

    versioning::initialize(deps.storage, CONTRACT_VERSION)?;

    state::new_lease(&mut deps, info, new_lease)
        .and_then(|(batch, next_state)| state::save(deps.storage, &next_state).map(|()| batch))
        .map(response::response_only_messages)
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn migrate(deps: DepsMut<'_>, _env: Env, msg: MigrateMsg) -> ContractResult<CwResponse> {
    // Statically assert that the message is empty when doing a software-only update.
    let MigrateMsg {} = msg;

    versioning::update_software(deps.storage, CONTRACT_VERSION, Into::into)
        .and_then(response::response)
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn reply(mut deps: DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    state::load(deps.storage)
        .and_then(|state| state.reply(&mut deps, env, msg))
        .and_then(
            |Response {
                 response,
                 next_state,
             }| state::save(deps.storage, &next_state).map(|()| response),
        )
        .map(response::response_only_messages)
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    state::load(deps.storage)
        .and_then(|state| process_execute(msg, state, &mut deps, env, info))
        .and_then(
            |Response {
                 response,
                 next_state,
             }| state::save(deps.storage, &next_state).map(|()| response),
        )
        .map(response::response_only_messages)
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    state::load(deps.storage)
        .and_then(|state| process_sudo(msg, state, deps.as_ref(), env))
        .and_then(
            |Response {
                 response,
                 next_state,
             }| state::save(deps.storage, &next_state).map(|()| response),
        )
        .map(response::response_only_messages)
        .or_else(|err| platform_error::log(err, deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
    state::load(deps.storage)
        .and_then(|state| state.state(env.block.time, deps.querier))
        .and_then(|resp| to_json_binary(&resp).map_err(Into::into))
        .or_else(|err| platform_error::log(err, deps.api))
}

fn process_execute(
    msg: ExecuteMsg,
    state: State,
    deps: &mut DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Repay() => state.repay(deps, env, info),
        ExecuteMsg::ClosePosition(spec) => state.close_position(spec, deps, env, info),
        ExecuteMsg::Close() => state.close(deps, env, info),
        ExecuteMsg::TimeAlarm {} => state.on_time_alarm(deps.as_ref(), env, info),
        ExecuteMsg::PriceAlarm() => state.on_price_alarm(deps.as_ref(), env, info),
        ExecuteMsg::DexCallback() => {
            access_control::check(&info.sender, &env.contract.address)?;
            state.on_dex_inner(deps.as_ref(), env)
        }
        ExecuteMsg::DexCallbackContinue() => {
            access_control::check(&info.sender, &env.contract.address)?;
            state.on_dex_inner_continue(deps.as_ref(), env)
        }
        ExecuteMsg::Heal() => state.heal(deps.as_ref(), env),
    }
}

fn process_sudo(msg: SudoMsg, state: State, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
    match msg {
        SudoMsg::OpenAck {
            port_id: _,
            channel_id: _,
            counterparty_channel_id: _,
            counterparty_version,
        } => state.on_open_ica(counterparty_version, deps, env),
        SudoMsg::Response { request: _, data } => state.on_dex_response(data, deps, env),
        SudoMsg::Timeout { request: _ } => state.on_dex_timeout(deps, env),
        SudoMsg::Error {
            request: _,
            details: _,
        } => state.on_dex_error(deps, env),
        _ => unreachable!(),
    }
}
