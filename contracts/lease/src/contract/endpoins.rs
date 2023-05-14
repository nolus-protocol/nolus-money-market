use ::currency::lease::LeaseGroup;
use dex::Handler;
use finance::currency;
use platform::response;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply},
    neutron_sdk::sudo::msg::SudoMsg,
};
use versioning::{version, VersionSegment};

use crate::{
    api::{ExecuteMsg, MigrateMsg, NewLeaseContract, StateQuery},
    contract::{state::Handler as LeaseHandler, Contract},
    error::{ContractError, ContractResult},
};

use super::state::{self, Migrate, Response, State};

const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 2;
const CONTRACT_STORAGE_VERSION: VersionSegment = 3;

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
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _>(
        deps.storage,
        version!(CONTRACT_STORAGE_VERSION),
        |storage: &mut _| {
            state::load_v2(storage)
                .map(|lease_v2| lease_v2.into_last_version())
                .and_then(
                    |Response {
                         response,
                         next_state: lease_v3,
                     }| state::save(storage, &lease_v3).map(|()| response),
                )
        },
    )
    .and_then(|(release_label, resp)| response::response_with_messages(release_label, resp))
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
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    let may_resp = matches!(msg, ExecuteMsg::TimeAlarm {} | ExecuteMsg::PriceAlarm {})
        .then(|| env.contract.address.clone());
    state::load(deps.storage)
        .and_then(|state| state.execute(&mut deps, env, info, msg))
        .and_then(
            |Response {
                 response,
                 next_state,
             }| state::save(deps.storage, &next_state).map(|()| response),
        )
        .and_then(|resp| {
            if let Some(contract_resp) = may_resp {
                response::response_with_messages::<_, _, ContractError>(&contract_resp, resp)
            } else {
                Ok(response::response_only_messages(resp))
            }
        })
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
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
    state::load(deps.storage)
        .and_then(|state| state.state(env.block.time, &deps.querier))
        .and_then(|resp| to_binary(&resp).map_err(Into::into))
}

fn process_sudo(msg: SudoMsg, state: State, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
    match msg {
        SudoMsg::OpenAck {
            port_id: _,
            channel_id: _,
            counterparty_channel_id: _,
            counterparty_version,
        } => state.on_open_ica(counterparty_version, deps, env).into(),
        SudoMsg::Response { request: _, data } => state.on_response(data, deps, env),
        SudoMsg::Timeout { request: _ } => state.on_timeout(deps, env).into(),
        SudoMsg::Error {
            request: _,
            details: _,
        } => state.on_error(deps, env).into(),
        _ => unreachable!(),
    }
    .into()
}
