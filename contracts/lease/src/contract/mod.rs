use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Response};
#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cw2::set_contract_version;

use finance::price::PriceDTO;
use platform::{
    bank::BankStub,
    batch::Emitter,
};

use crate::{
    contract::{
        alarms::price_alarm::PriceAlarm,
        open::OpenLoanReqResult,
    },
    error::{
        ContractError,
        ContractResult
    },
    lease::{self, DownpaymentDTO, LeaseDTO},
    msg::{ExecuteMsg, NewLeaseForm, StateQuery},
};

use self::{close::Close, repay::RepayResult};
use self::open::{OpenLoanReq, OpenLoanResp};
use self::repay::Repay;
use self::state::LeaseState;

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

    deps.api.addr_validate(form.market_price_oracle.as_str())
        .map_err(|_| ContractError::InvalidParameters(
            format!("Invalid Market Price Oracle address provided! Input: {:?}", form.market_price_oracle.as_str())
        ))?;

    let lease = form.into_lease_dto(env.block.time, deps.api, &deps.querier)?;
    lease.store(deps.storage)?;

    let OpenLoanReqResult {
        batch,
        downpayment,
    } = lease::execute(
        lease,
        OpenLoanReq::new(&info.funds),
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

    let emitter = lease::execute(
        lease,
        OpenLoanResp::new(msg, downpayment, account, &env),
        &deps.querier,
    )?;

    Ok(emitter.into())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    let lease = LeaseDTO::load(deps.storage)?;

    match msg {
        ExecuteMsg::Repay() => {
            let res = try_repay(&deps.querier, env, info, lease)?;
            LeaseDTO::store(&res.lease_dto, deps.storage)?;
            Ok(res.emitter.into())
        }
        ExecuteMsg::Close() => try_close(deps, env, info, lease).map(Into::into),
        ExecuteMsg::PriceAlarm {
            price,
        } => run_price_alarm_liquidation(deps, env, info, lease, price),
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
        &deps.querier,
    )
}

fn try_repay(
    querier: &QuerierWrapper,
    env: Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<RepayResult> {
    let account = BankStub::my_account(&env, querier);

    lease::execute(
        lease,
        Repay::new(
            &info.funds,
            account,
            &env,
        ),
        querier,
    )
}

fn try_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lease: LeaseDTO,
) -> ContractResult<Emitter> {
    let bank = BankStub::my_account(&env, &deps.querier);

    let emitter = lease::execute(
        lease,
        Close::new(
            &info.sender,
            env.contract.address.clone(),
            bank,
            env.block.time,
        ),
        &deps.querier,
    )?;

    Ok(emitter)
}

fn run_price_alarm_liquidation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    lease: LeaseDTO,
    price: PriceDTO,
) -> ContractResult<Response> {
    let result = lease::execute(
        lease,
        PriceAlarm::new(
            &info.sender,
            env.contract.address.clone(),
            BankStub::my_account(&env, &deps.querier),
            env.block.time,
            price,
        ),
        &deps.querier,
    )?;

    result.lease.store(deps.storage)?;

    Ok(result.into_response.convert())
}
