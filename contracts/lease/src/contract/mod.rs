use cosmwasm_std::{
    Api, Binary, Deps, DepsMut, ensure, Env, MessageInfo, QuerierWrapper, Reply, Response,
};
#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use finance::price::PriceDTO;
use platform::{bank::BankStub, batch::Emitter};

use crate::{
    contract::{
        alarms::{LiquidationResult, price::PriceAlarm},
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
        deps.api,
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

    let downpayment = DownpaymentDTO::remove(deps.storage)?;

    let id = ReplyId::try_from(msg.id);

    ensure!(
        id.is_ok(),
        ContractError::InvalidParameters("Invalid reply ID passed!".into())
    );

    match id.unwrap() {
        ReplyId::OpenLoanReq => {
            let emitter = lease::execute(
                lease,
                OpenLoanResp::new(msg, downpayment, account, &env),
                deps.api,
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
                try_repay((&deps.querier, deps.api, &env), account, info, lease)?;

            lease_dto.store(deps.storage)?;

            Ok(emitter.into())
        }
        ExecuteMsg::Close() => {
            try_close((&deps.querier, deps.api, &env), account, info, lease).map(Into::into)
        }
        ExecuteMsg::PriceAlarm { price } => {
            let LiquidationResult {
                response,
                lease_dto: lease,
            } = try_on_price_alarm((&deps.querier, deps.api, &env), account, info, lease, price)?;

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
        LeaseState::new(env.block.time, bank, env.contract.address.clone()),
        deps.api,
        &deps.querier,
    )
}

fn try_repay(
    (querier, api, env): (&QuerierWrapper, &dyn Api, &Env),
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<RepayResult> {
    lease::execute(lease, Repay::new(&info.funds, account, env), api, querier)
}

fn try_close(
    (querier, api, env): (&QuerierWrapper, &dyn Api, &Env),
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
        api,
        querier,
    )?;

    Ok(emitter)
}

fn try_on_price_alarm(
    (querier, api, env): (&QuerierWrapper, &dyn Api, &Env),
    account: BankStub,
    info: MessageInfo,
    lease: LeaseDTO,
    price: PriceDTO,
) -> ContractResult<LiquidationResult> {
    lease::execute(
        lease,
        PriceAlarm::new(
            &info.sender,
            env.contract.address.clone(),
            account,
            env.block.time,
            price,
        ),
        api,
        querier,
    )
}
