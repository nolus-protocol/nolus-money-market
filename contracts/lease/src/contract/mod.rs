#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Response};
use cw2::set_contract_version;

use platform::{bank::BankStub, batch::Emitter};

use crate::{
    contract::{
        alarms::{price::PriceAlarm, time::TimeAlarm, AlarmResult},
        open::OpenLoanReqResult,
    },
    error::{ContractError, ContractResult},
    lease::{self, DownpaymentDTO, LeaseDTO},
    msg::{ExecuteMsg, NewLeaseForm, StateQuery},
    repay_id::ReplyId,
};

use self::{
    close::Close,
    open::{OpenLoanReq, OpenLoanResp},
    repay::{Repay, RepayResult},
    state::LeaseState,
};

mod alarms;
mod close;
mod open;
mod repay;
mod state;

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    form: NewLeaseForm,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let lease = form.into_lease_dto(env.block.time, deps.api, &deps.querier)?;
    lease.store(deps.storage)?;

    let OpenLoanReqResult { batch, downpayment } = lease::execute(
        lease,
        OpenLoanReq::new(&info.funds),
        &env.contract.address,
        &deps.querier,
    )?;

    downpayment.store(deps.storage)?;

    Ok(batch.into())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> ContractResult<Response> {
    // TODO swap the received loan and the downpayment to lease.currency
    let lease = LeaseDTO::load(deps.storage)?;

    let account = BankStub::my_account(&env, &deps.querier);

    let id = ReplyId::try_from(msg.id)
        .map_err(|_| ContractError::InvalidParameters("Invalid reply ID passed!".into()))?;

    match id {
        ReplyId::OpenLoanReq => {
            let downpayment = DownpaymentDTO::remove(deps.storage)?;

            let emitter = lease::execute(
                lease,
                OpenLoanResp::new(msg, downpayment, account, &env),
                &env.contract.address,
                &deps.querier,
            )?;

            Ok(emitter.into())
        }
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    let lease = LeaseDTO::load(deps.storage)?;

    let account = BankStub::my_account(&env, &deps.querier);

    match msg {
        ExecuteMsg::Repay() => {
            let RepayResult { lease_dto, emitter } =
                try_repay(&deps.querier, &env, account, info, lease)?;

            lease_dto.store(deps.storage)?;

            Ok(emitter.into())
        }
        ExecuteMsg::Close() => try_close(&deps.querier, &env, account, info, lease).map(Into::into),
        ExecuteMsg::PriceAlarm() => {
            let AlarmResult {
                response,
                lease_dto: lease,
            } = try_on_price_alarm(&deps.querier, &env, account, info, lease)?;

            lease.store(deps.storage)?;

            Ok(response)
        }
        ExecuteMsg::TimeAlarm() => {
            let AlarmResult {
                response,
                lease_dto: lease,
            } = try_on_time_alarm(&deps.querier, &env, account, info, lease)?;

            lease.store(deps.storage)?;

            Ok(response)
        }
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<Binary> {
    let lease = LeaseDTO::load(deps.storage)?;

    let bank = BankStub::my_account(&env, &deps.querier);

    // TODO think on taking benefit from having a LppView trait
    lease::execute(
        lease,
        LeaseState::new(env.block.time, bank),
        &env.contract.address,
        &deps.querier,
    )
}

fn try_repay(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<RepayResult> {
    lease::execute(
        lease,
        Repay::new(&info.funds, account, env),
        &env.contract.address,
        querier,
    )
}

fn try_close(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<Emitter> {
    let emitter = lease::execute(
        lease,
        Close::new(
            &info.sender,
            env.contract.address.clone(),
            account,
            env.block.time,
        ),
        &env.contract.address,
        querier,
    )?;

    Ok(emitter)
}

fn try_on_price_alarm(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<AlarmResult> {
    lease::execute(
        lease,
        PriceAlarm::new(env, &info.sender, account, env.block.time),
        &env.contract.address,
        querier,
    )
}

fn try_on_time_alarm(
    querier: &QuerierWrapper,
    env: &Env,
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<AlarmResult> {
    lease::execute(
        lease,
        TimeAlarm::new(env, &info.sender, account, env.block.time),
        &env.contract.address,
        querier,
    )
}
