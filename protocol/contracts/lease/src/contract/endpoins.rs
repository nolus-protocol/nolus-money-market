#[cfg(feature = "osmosis")]
use dex::TransferInInit;
#[cfg(feature = "astroport")]
use finance::coin::Amount;
use platform::{error as platform_error, message::Response as MessageResponse, response};
#[cfg(not(feature = "osmosis-osmosis-usdc_noble"))]
use sdk::cosmwasm_std::Addr;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper,
        Reply, Storage,
    },
    neutron_sdk::sudo::msg::SudoMsg,
};
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    api::{
        open::NewLeaseContract, query::StateQuery, ExecuteMsg, LeaseAssetCurrencies, MigrateMsg,
    },
    contract::api::Contract,
    error::ContractResult,
};

use super::state::{self, Response, State};

const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 6;
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
    currency::validate::<LeaseAssetCurrencies>(&new_lease.form.currency)?;
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
pub fn migrate(deps: DepsMut<'_>, env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _, _>(
        deps.storage,
        CONTRACT_VERSION,
        |storage: &mut _| may_migrate(storage, deps.querier, env),
        Into::into,
    )
    .and_then(|(release_label, resp)| response::response_with_messages(release_label, resp))
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

// TODO factor out the implementations of execute, sudo, and reply to use  `fn process_lease`
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

fn may_migrate(
    _storage: &mut dyn Storage,
    _querier: QuerierWrapper<'_>,
    _env: Env,
) -> ContractResult<MessageResponse> {
    #[cfg(feature = "astroport")]
    {
        const NEUTRON_LEASE1: &str =
            "nolus1yhcph5r2x9rss6tluptttma736rknasjwn3659620ysu5fhmx2wq47gmch";
        const NEUTRON_LEASE1_AMOUNT1: Amount = 28883542;
        const NEUTRON_LEASE1_AMOUNT2: Amount = 7220808;

        const NEUTRON_LEASE2: &str =
            "nolus1psw6ugdjm82mqnm4cj649e3jgu9pwe3p8jnrk7qjf284knq3crfsh7pk2r";
        const NEUTRON_LEASE2_AMOUNT1: Amount = 11668296;
        const NEUTRON_LEASE2_AMOUNT2: Amount = 17502294;

        let this_lease = this_contract_ref(&_env);
        if this_lease == &NEUTRON_LEASE1 {
            process_lease(
                _storage,
                add_amounts(NEUTRON_LEASE1_AMOUNT1, NEUTRON_LEASE1_AMOUNT2, this_lease),
            )
        } else if this_lease == &NEUTRON_LEASE2 {
            process_lease(
                _storage,
                add_amounts(NEUTRON_LEASE2_AMOUNT1, NEUTRON_LEASE2_AMOUNT2, this_lease),
            )
        } else {
            Ok(MessageResponse::default())
        }
    }
    #[cfg(feature = "osmosis")]
    {
        const TIMEOUT_LEASE: &str =
            "nolus13z34cafmq553y8y2zywdvv2zzfzp8590qqyg4dwjyvdtj2mj7tgqeusqtt";
        const TRANSFER_OUT_ERROR_LEASE: &str =
            "nolus1jndqg7vkpe7c605c3urf3sug07qwcfqxnrzvxs5phj47flxl2uyqg5fkye";
        if this_contract_ref(&_env) == &TIMEOUT_LEASE {
            process_lease(_storage, transfer_finish_time_out(_querier, _env))
        } else if this_contract_ref(&_env) == &TRANSFER_OUT_ERROR_LEASE {
            process_lease(_storage, |lease| lease.on_dex_error(_querier, _env))
        } else {
            Ok(MessageResponse::default())
        }
    }
    #[cfg(feature = "osmosis-osmosis-usdc_noble")]
    {
        Ok(MessageResponse::default())
    }
}

#[cfg(not(feature = "osmosis-osmosis-usdc_noble"))]
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

#[cfg(feature = "astroport")]
fn add_amounts(
    amount1: Amount,
    amount2: Amount,
    this_lease: &Addr,
) -> impl FnOnce(State) -> ContractResult<Response> + '_ {
    use dex::SwapExactInRespDelivery;

    move |lease| {
        let updated_state = match lease {
            State::BuyAsset(state) => state.map(|dex_state| match dex_state {
                dex::StateRemoteOut::SwapExactInRespDelivery(resp_delivery) => {
                    let resp_with_amounts = swap::migration::build_two_responses(amount1, amount2);
                    let resp_delivery_updated =
                        resp_delivery.patch_response(resp_with_amounts.into());
                    <SwapExactInRespDelivery<_, _, _, _, _> as Into<
                        dex::StateRemoteOut<_, _, _, _, _, _>,
                    >>::into(resp_delivery_updated)
                }
                _ => {
                    unreachable!(
                        "Found a dex sub-state != SwapExactInResponseDelivery for {}",
                        this_lease
                    )
                }
            }),
            _ => unreachable!("Found a state != BuyAsset for {}", this_lease),
        };
        Ok(Response::no_msgs(updated_state))
    }
}

#[cfg(feature = "osmosis")]
fn transfer_finish_time_out(
    querier: QuerierWrapper<'_>,
    env: Env,
) -> impl FnOnce(State) -> ContractResult<Response> + '_ {
    move |lease| {
        let new_state = match lease {
            State::ClosingTransferIn(state) => state.map(|dex_state| match dex_state {
                dex::StateLocalOut::TransferInFinish(transfer_in_finish) => {
                    <TransferInInit<_, _> as Into<dex::StateLocalOut<_, _, _, _, _>>>::into(
                        transfer_in_finish.into_init(),
                    )
                }
                _ => {
                    unreachable!(
                        "Found a dex sub-state != TransferInFinish for {}",
                        this_contract_ref(&env)
                    )
                }
            }),
            _ => unreachable!(
                "Found a state != ClosingTransferIn for {}",
                this_contract(env)
            ),
        };
        new_state.on_dex_timeout(querier, env)
    }
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
        SudoMsg::Timeout { request: _ } => state.on_dex_timeout(deps.querier, env),
        SudoMsg::Error {
            request: _,
            details: _,
        } => state.on_dex_error(deps.querier, env),
        _ => unreachable!(),
    }
}

#[cfg(not(feature = "osmosis-osmosis-usdc_noble"))]
fn this_contract_ref(env: &Env) -> &Addr {
    &env.contract.address
}

#[cfg(feature = "osmosis")]
fn this_contract(env: Env) -> Addr {
    env.contract.address
}
