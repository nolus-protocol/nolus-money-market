use finance::duration::Duration;
use platform::{error as platform_error, message::Response as MessageResponse, response};
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        Api, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Storage, entry_point,
        to_json_binary,
    },
    neutron_sdk::sudo::msg::SudoMsg,
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    api::{ExecuteMsg, MigrateMsg, open::NewLeaseContract, query::QueryMsg},
    contract::api::Contract,
    error::{ContractError, ContractResult},
};

use super::state::{self, Response, State};

const CONTRACT_STORAGE_VERSION: VersionSegment = 9;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    new_lease: NewLeaseContract,
) -> ContractResult<CwResponse> {
    //TODO move the following validations into the deserialization
    deps.api.addr_validate(new_lease.finalizer.as_str())?;
    deps.api.addr_validate(new_lease.form.customer.as_str())?;

    platform::contract::validate_addr(deps.querier, &new_lease.form.time_alarms)?;
    platform::contract::validate_addr(deps.querier, &new_lease.form.market_price_oracle)?;
    platform::contract::validate_addr(deps.querier, &new_lease.form.loan.lpp)?;
    platform::contract::validate_addr(deps.querier, &new_lease.form.loan.profit)?;

    state::new_lease(deps.querier, info, new_lease)
        .and_then(|(batch, next_state)| state::save(deps.storage, &next_state).map(|()| batch))
        .map(response::response_only_messages)
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
) -> ContractResult<CwResponse> {
    migrate_from
        .update_software(&CURRENT_RELEASE, &to_release)
        .map(|()| response::empty_response())
        .map_err(ContractError::UpdateSoftware)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn reply(deps: DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    process_lease(deps.storage, |lease| lease.reply(deps.querier, env, msg))
        .map(response::response_only_messages)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    process_lease(deps.storage, |lease| {
        process_execute(msg, lease, deps.querier, env, info)
    })
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    process_lease(deps.storage, |lease| {
        process_sudo(msg, lease, deps.api, deps.querier, env)
    })
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::State { due_projection } => state::load(deps.storage)
            .and_then(|state| {
                state.state(
                    env.block.time,
                    Duration::from_secs(due_projection),
                    deps.querier,
                )
            })
            .and_then(|resp| to_json_binary(&resp).map_err(Into::into)),
        QueryMsg::ProtocolPackageRelease {} => to_json_binary(&CURRENT_RELEASE).map_err(Into::into),
    }
    .inspect_err(platform_error::log(deps.api))
}

fn process_lease<ProcFn>(
    storage: &mut dyn Storage,
    process_fn: ProcFn,
) -> ContractResult<MessageResponse>
where
    ProcFn: FnOnce(State) -> ContractResult<Response>,
{
    state::load(storage).and_then(process_fn).and_then(
        |Response {
             response,
             next_state,
         }| state::save(storage, &next_state).map(|()| response),
    )
}

fn process_execute(
    msg: ExecuteMsg,
    state: State,
    querier: QuerierWrapper<'_>,
    env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Repay() => state.repay(querier, env, info),
        ExecuteMsg::ChangeClosePolicy(change) => {
            state.change_close_policy(change, querier, env, info)
        }
        ExecuteMsg::ClosePosition(spec) => state.close_position(spec, querier, env, info),
        ExecuteMsg::Close() => state.close(querier, env, info),
        ExecuteMsg::TimeAlarm {} => state.on_time_alarm(querier, env, info),
        ExecuteMsg::PriceAlarm() => state.on_price_alarm(querier, env, info),
        ExecuteMsg::DexCallback() => {
            access_control::check(&info.sender, &env.contract.address)?;
            state.on_dex_inner(querier, env)
        }
        ExecuteMsg::DexCallbackContinue() => {
            access_control::check(&info.sender, &env.contract.address)?;
            state.on_dex_inner_continue(querier, env)
        }
        ExecuteMsg::Heal() => state.heal(querier, env, info),
    }
}

fn process_sudo(
    msg: SudoMsg,
    state: State,
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
    env: Env,
) -> ContractResult<Response> {
    match msg {
        SudoMsg::OpenAck {
            port_id: _,
            channel_id: _,
            counterparty_channel_id: _,
            counterparty_version,
        } => state.on_open_ica(counterparty_version, querier, env),
        SudoMsg::Response { request: _, data } => state.on_dex_response(data, querier, env),
        SudoMsg::Timeout { request: _ } => state.on_dex_timeout(querier, env),
        SudoMsg::Error {
            request: _,
            details,
        } => {
            let resp = details.into();
            api.debug(&format!("SudoMsg::Error({})", resp));
            state.on_dex_error(resp, querier, env)
        }
        _ => unreachable!(),
    }
}
