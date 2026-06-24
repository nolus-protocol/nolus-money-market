use access_control::permissions::DexResponseSafeDeliveryPermission;
use cw_time::IntoInstant;
use finance::duration::Duration;
use platform::{
    contract::{self, Validator},
    error as platform_error,
    message::Response as MessageResponse,
    response,
};
use sdk::{
    api::SudoMsg,
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        Api, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Storage, entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, VersionSegment, package_name, package_version,
};

use crate::{
    api::{ExecuteMsg, MigrateMsg, open::NewLeaseContract, query::QueryMsg},
    contract::api::Contract,
    error::{ContractError, ContractResult},
};

use super::state::{self, Response, State};

const CONTRACT_STORAGE_VERSION: VersionSegment = 15;
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

    let addr_validator = contract::validator(deps.querier);
    addr_validator.check_contract(&new_lease.form.time_alarms)?;
    addr_validator.check_contract(&new_lease.form.market_price_oracle)?;
    addr_validator.check_contract(&new_lease.form.loan.lpp)?;
    addr_validator.check_contract(&new_lease.form.loan.profit)?;

    state::new_lease(deps.querier, info, new_lease)
        .and_then(|(batch, next_state)| state::save(deps.storage, &next_state).map(|()| batch))
        .map(response::response_only_messages)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    _msg: ProtocolMigrationMessage<MigrateMsg>,
) -> ContractResult<CwResponse> {
    // v10 reshapes the persisted `LeaseDTO` to carry the Solana-side
    // remote-lease PDA as a non-optional field, so a pre-v10 lease cannot
    // be deserialised under the new layout. A v9 lease has no meaningful
    // `remote_lease_id` to synthesise — its `dex_account` is an ICA host on
    // the DEX chain, not a Solana PDA — so a real v9→v10 migration would
    // have to invent a sentinel and leave the lease permanently Cosmos-side
    // only. Mainnet v9-lease population is zero (plan §10.A.1), so no
    // in-flight state is at risk there; reject any migrate attempt loudly
    // rather than silently failing the first post-upgrade load.
    //
    // Operational posture for non-mainnet (devnet/testnet/local): drain all
    // v9 leases to a terminal state before upgrading the lease code to v10.
    // There is no `ExecuteMsg` escape hatch for a stranded v9 lease — the
    // storage layout is binary-incompatible — so the drain is a prerequisite,
    // not a recovery step. See `protocol/docs/remote-lease-callback-flow.md`.
    //
    // v11 reshapes the opening-swap state: the BuyAsset spec gains the
    // controller address and slippage bound, and the swap leg moved from the
    // ICA `SwapExactIn` to the `RemoteSwap` transport; no live v10
    // remote-lease population exists, so the refusal stays.
    //
    // v12 reshapes the persisted `LeaseDTO` again: it gains the
    // non-optional `remote_lease_controller` so the post-opening legs can
    // emit operations without re-querying the leaser. No live v11
    // remote-lease population exists, so the refusal stays.
    //
    // v13 funds the opening lease by an ICS-20 transfer to the Solana-side
    // `LeaseAuthority` instead of opening an ICA: the dex `Account.host`
    // becomes optional (`None` for remote-lease leases), the opening
    // composite drops the ICA-open and ICA transfer-out legs for a sequential
    // funding leg, and the `BuyAsset` spec gains the bridged funding receiver.
    // No live v12 remote-lease population exists, so the refusal stays.
    //
    // v14 adds the `OpeningUnwind` in-flight state: a zero-acked opening-swap
    // error now drains the downpayment and principal back from the
    // `LeaseAuthority` and refunds before reaching `OpenFailed`, instead of
    // parking. The persisted `State` enum gains the variant; the change is
    // forward-only with no data transform. Once any lease persists as
    // `OpeningUnwind` a rolled-back v13 binary cannot load it — the enum layout
    // is binary-incompatible — so v14 is not rollback-safe past that point. No
    // live v13 remote-lease population exists, so the refusal stays.
    //
    // v15 gives the two sibling home-bound drains — the opened-lease proceeds
    // drain and the paid-lease asset transfer-out — the same persisted
    // per-currency `baseline` the v14 `OpeningUnwind` drain carries, so their
    // arrival check measures every coin against its pre-drain balance instead
    // of an absolute one. The persisted drain `State` variants gain the field;
    // the change is forward-only with no data transform. Once any lease
    // persists mid-drain a rolled-back v14 binary cannot load it — the drain
    // layout is binary-incompatible — so v15 is not rollback-safe past that
    // point. No live v14 remote-lease population exists, so the refusal stays.
    Err(ContractError::UnsupportedMigration).inspect_err(platform_error::log(deps.api))
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
                    env.block.time.into_instant(),
                    Duration::from_secs(due_projection),
                    deps.querier,
                )
            })
            .and_then(|resp| cosmwasm_std::to_json_binary(&resp).map_err(Into::into)),
        QueryMsg::ProtocolPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
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
        ExecuteMsg::TimeAlarm {} => state.on_time_alarm(querier, env, info),
        ExecuteMsg::PriceAlarm() => state.on_price_alarm(querier, env, info),
        ExecuteMsg::DexCallback() => {
            access_control::check(
                &DexResponseSafeDeliveryPermission::new(&env.contract),
                &info,
            )?;
            state.on_dex_inner(querier, env)
        }
        ExecuteMsg::RemoteLeaseCallback(callback) => {
            state.on_remote_lease_callback(callback, info, querier, env)
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
        SudoMsg::Response { request: _, data } => state.on_dex_response(data, querier, env),
        SudoMsg::Error {
            request: _,
            details,
        } => {
            let resp = details.into();
            api.debug(&format!("SudoMsg::Error({resp})"));
            state.on_dex_error(resp, querier, env)
        }
        SudoMsg::Timeout { request: _ } => state.on_dex_timeout(querier, env),
        // The lease funds over the ICS-20 transfer channel and never registers
        // an ICA, so it can never receive an `OpenAck`.
        SudoMsg::OpenAck { .. } => Err(ContractError::unsupported_operation("open ica response")),
    }
}
