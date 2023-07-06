use currency::lease::LeaseGroup;
use platform::response;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Api, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};
use versioning::{version, VersionSegment};

use crate::{
    api::{ExecuteMsg, MigrateMsg, NewLeaseContract, StateQuery},
    contract::api::Contract,
    error::{ContractError, ContractResult},
};

#[cfg(feature = "migration")]
use super::state::Migrate;
use super::state::{self, Response, State};

#[cfg(feature = "migration")]
const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 3;
const CONTRACT_STORAGE_VERSION: VersionSegment = 4;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    new_lease: NewLeaseContract,
) -> ContractResult<CwResponse> {
    //TODO move the following validation into the deserialization
    currency::validate::<LeaseGroup>(&new_lease.form.currency)?;
    deps.api.addr_validate(new_lease.form.customer.as_str())?;

    platform::contract::validate_addr(&deps.querier, &new_lease.form.time_alarms)?;
    platform::contract::validate_addr(&deps.querier, &new_lease.form.market_price_oracle)?;
    platform::contract::validate_addr(&deps.querier, &new_lease.form.loan.lpp)?;
    platform::contract::validate_addr(&deps.querier, &new_lease.form.loan.profit)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    state::new_lease(&mut deps, info, new_lease)
        .and_then(|(batch, next_state)| state::save(deps.storage, &next_state).map(|()| batch))
        .map(response::response_only_messages)
        .or_else(|err| log_error(err, deps.api))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    #[cfg(feature = "migration")]
    let resp = versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _>(
        deps.storage,
        version!(CONTRACT_STORAGE_VERSION),
        |storage: &mut _| {
            state::load_v2(storage)
                .and_then(|lease_v2| lease_v2.into_last_version(_env.block.time))
                .and_then(
                    |Response {
                         response,
                         next_state: lease_v3,
                     }| state::save(storage, &lease_v3).map(|()| response),
                )
        },
    )
    .and_then(|(release_label, resp)| response::response_with_messages(release_label, resp));

    #[cfg(not(feature = "migration"))]
    let resp = versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .and_then(response::response);
    resp.or_else(|err| log_error(err, deps.api))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
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
        .or_else(|err| log_error(err, deps.api))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
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
        .or_else(|err| log_error(err, deps.api))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
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
        .or_else(|err| log_error(err, deps.api))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
    state::load(deps.storage)
        .and_then(|state| state.state(env.block.time, &deps.querier))
        .and_then(|resp| to_binary(&resp).map_err(Into::into))
        .or_else(|err| log_error(err, deps.api))
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
        ExecuteMsg::Close() => state.close(deps, env, info),
        ExecuteMsg::TimeAlarm {} => state.on_time_alarm(deps.as_ref(), env, info),
        ExecuteMsg::PriceAlarm() => state.on_price_alarm(deps.as_ref(), env, info),
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

fn log_error<T>(err: ContractError, api: &dyn Api) -> ContractResult<T> {
    //TODO switch to calling this with Result::inspect_err once stabilized
    api.debug(&format!("{:?}", err));
    Err(err)
}
